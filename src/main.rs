mod core;
mod tui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use crate::core::{
    preview_shot, resolve_shot, Club, Course, Direction, Hole, HoleMeta, HoleScore, Pos, Scorecard,
    Shot, ShotResult, TerrainKind, Wind, COURSE_HEIGHT, COURSE_WIDTH,
};
use crate::tui::{
    BuilderHeaderView, BuilderMode, BuilderOrientation, BuilderSetupView, BuilderView,
    CourseMenuState, CourseView, HolePickerView, Lang, ScorecardView, SidebarState, Viewport,
};
use directories::ProjectDirs;
use rand::Rng;
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction as LayoutDirection, Layout}, Terminal};
use serde::{Deserialize, Serialize};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SAVE_PATH: &str = "save.yaml";

/// Plus bas plafond de dé sélectionnable par le joueur (`+`/`-`, voir
/// `GameState::adjust_die_strength`). En dessous de 3, un Putter (portée
/// die*0.5) ne pourrait plus parcourir que 0.5-1 case, rendant le coup
/// souvent inutile plutôt que "sûr" — le plancher garde une marge de
/// réduction utile sans ce piège.
const DIE_STRENGTH_FLOOR: u8 = 3;

/// Racine de données effective pour cette exécution — préfixe appliqué à
/// `courses/`, `save.yaml` et `courses/_library/` partout dans le fichier.
/// Deux niveaux, pas un remplacement pur et simple du cwd (voir
/// `ROADMAP.md`, "Répertoire de données par défaut") :
/// 1. `./courses` (chemin vide, donc relatif au cwd) si ce dossier existe
///    déjà — workflow de développement inchangé (`cargo run` depuis le
///    dépôt cloné).
/// 2. Sinon le dossier de données de la plateforme (`ProjectDirs`,
///    `~/.local/share/divotty` sur Linux, équivalents macOS/Windows) — un
///    binaire installé via `cargo install` et lancé depuis n'importe où
///    obtient ainsi un emplacement stable plutôt qu'un `courses/`
///    différent à chaque répertoire courant. Reste la cible même s'il est
///    encore vide (rien n'y a encore été sauvegardé) : seule la *liste*
///    des parcours affichés retombe sur les parcours embarqués
///    (`embedded_courses`, voir `discover_courses`) dans ce cas, pas
///    l'emplacement où écrire les futures sauvegardes/trous du builder.
/// Repli sur le cwd si `ProjectDirs` échoue (rare : `$HOME` non résolvable).
fn resolve_data_root() -> PathBuf {
    resolve_data_root_from(
        Path::new("courses").exists(),
        ProjectDirs::from("", "", "divotty").map(|dirs| dirs.data_dir().to_path_buf()),
    )
}

/// Logique pure derrière `resolve_data_root`, testable sans toucher au vrai
/// système de fichiers ni à `$HOME`.
fn resolve_data_root_from(cwd_has_courses: bool, platform_data_dir: Option<PathBuf>) -> PathBuf {
    if cwd_has_courses {
        PathBuf::new()
    } else {
        platform_data_dir.unwrap_or_default()
    }
}

/// Liste les parcours jouables sous `<data_root>/courses/`, avec leur
/// dossier d'origine (nécessaire pour recharger/sauvegarder). Si ce dossier
/// n'existe pas ou ne contient rien (ex. premier lancement après un
/// `cargo install`, avant toute sauvegarde), bascule sur les parcours
/// embarqués (`embedded_courses()`) — sans dossier associé, donc non
/// sauvegardables.
fn discover_courses(data_root: &Path) -> Result<Vec<(Option<PathBuf>, Course)>> {
    let root = data_root.join("courses");
    if root.exists() {
        let found = Course::discover(&root)?;
        if !found.is_empty() {
            return Ok(found.into_iter().map(|(dir, course)| (Some(dir), course)).collect());
        }
    }
    Ok(embedded_courses()?.into_iter().map(|course| (None, course)).collect())
}

/// Contenu de `courses/demo/` et `courses/quick3/` figé dans le binaire à la
/// compilation (`include_str!`), pour qu'un joueur sans le dossier
/// `courses/` sur disque voie quand même les vrais parcours (dont
/// l'enchaînement multi-trous de Quick 3) plutôt qu'un unique trou
/// générique — voir `discover_courses`. Un test (`embedded_courses_parse`)
/// vérifie que ce contenu reste parsable à chaque `cargo test`.
fn embedded_courses() -> Result<Vec<Course>> {
    const DEMO_COURSE_YAML: &str = include_str!("../courses/demo/course.yaml");
    const DEMO_HOLE_01: &str = include_str!("../courses/demo/hole_01.course");

    const QUICK3_COURSE_YAML: &str = include_str!("../courses/quick3/course.yaml");
    const QUICK3_HOLE_01: &str = include_str!("../courses/quick3/hole_01.course");
    const QUICK3_HOLE_02: &str = include_str!("../courses/quick3/hole_02.course");
    const QUICK3_HOLE_03: &str = include_str!("../courses/quick3/hole_03.course");

    Ok(vec![
        Course::from_embedded(DEMO_COURSE_YAML, &[DEMO_HOLE_01])?,
        Course::from_embedded(
            QUICK3_COURSE_YAML,
            &[QUICK3_HOLE_01, QUICK3_HOLE_02, QUICK3_HOLE_03],
        )?,
    ])
}

/// Trou de secours minimal généré en mémoire, réservé aux tests unitaires
/// (rapide, sans dépendre du contenu réel des parcours) — le repli "pas de
/// dossier `courses/`" en jeu utilise désormais `embedded_courses()`.
#[cfg(test)]
fn fallback_course() -> Result<Course> {
    // Trou de secours généré en mémoire : fairway droit, un bunker, un peu
    // d'eau sur le côté, green autour du trou. Tee et trou espacés d'environ
    // 50 cases (~2-3 coups de Driver en moyenne), cohérent avec un par 4 —
    // pas juste étalé sur toute la largeur du canevas, qui est bien plus
    // grand que ce dont un par 4 a besoin.
    let width = crate::core::COURSE_WIDTH;
    let height = crate::core::COURSE_HEIGHT;
    let mut lines = Vec::with_capacity(height);
    for y in 0..height {
        let mut row = vec!['.'; width];
        if y == height / 2 {
            row[2] = 'D';
            row[50] = 'H';
            for x in 44..50 {
                row[x] = 'G';
            }
            for x in 30..35 {
                row[x] = 'B';
            }
        }
        if y == height / 2 - 3 || y == height / 2 + 3 {
            for x in 14..19 {
                row[x] = '~';
            }
        }
        lines.push(row.into_iter().collect::<String>());
    }
    let raw = format!(
        "name: \"Trou de démonstration\"\npar: 4\n---\n{}\n",
        lines.join("\n")
    );
    Ok(Course {
        name: "Parcours de démonstration".to_string(),
        difficulty: 1,
        holes: vec![Hole::parse(&raw)?],
    })
}

/// Tire un vent aléatoire (direction + force) au chargement d'un trou.
/// `core` ne fait qu'appliquer l'effet du vent — c'est `app` qui le tire,
/// comme le dé, jamais `rand::thread_rng()` à l'intérieur de `core`.
fn random_wind() -> Wind {
    let mut rng = rand::thread_rng();
    let angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    Wind {
        direction: Direction {
            dx: angle.cos(),
            dy: angle.sin(),
        },
        strength: rng.gen_range(0.0..3.0),
    }
}

/// Plafond de dé par défaut (3 à 6, voir `DIE_STRENGTH_FLOOR`) : 6 = pas de
/// plafond, comportement d'origine. Fonction plutôt que constante inline
/// pour servir de valeur `#[serde(default)]` (sauvegardes écrites avant
/// l'ajout du champ).
fn default_die_strength() -> u8 {
    6
}

/// Ce qui est persisté d'une partie en cours : juste assez pour retrouver le
/// parcours sur disque et reprendre exactement où le joueur s'est arrêté.
#[derive(Serialize, Deserialize)]
struct SaveData {
    course_dir: PathBuf,
    hole_index: usize,
    strokes: u8,
    ball: Pos,
    wind: Wind,
    club: Club,
    aim: Direction,
    #[serde(default)]
    scorecard: Scorecard,
    #[serde(default = "default_die_strength")]
    die_strength: u8,
}

fn save_game(state: &GameState, data_root: &Path) -> Result<()> {
    let course_dir = state
        .course_dir
        .clone()
        .ok_or_else(|| anyhow::anyhow!("ce parcours n'a pas de dossier associé, sauvegarde impossible"))?;
    let data = SaveData {
        course_dir,
        hole_index: state.hole_index,
        strokes: state.strokes,
        ball: state.ball,
        wind: state.wind,
        club: state.club,
        aim: state.aim,
        scorecard: state.scorecard.clone(),
        die_strength: state.die_strength,
    };
    let save_path = data_root.join(SAVE_PATH);
    if let Some(parent) = save_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(save_path, serde_yaml::to_string(&data)?)?;
    Ok(())
}

fn load_game(lang: Lang, data_root: &Path) -> Result<GameState> {
    let raw = std::fs::read_to_string(data_root.join(SAVE_PATH))?;
    let data: SaveData = serde_yaml::from_str(&raw)?;
    let course = Course::load_from_dir(&data.course_dir)?;
    let hole_count = course.holes.len();
    if data.hole_index >= hole_count {
        return Err(anyhow::anyhow!("numéro de trou invalide dans la sauvegarde"));
    }

    Ok(GameState {
        holes: course.holes,
        hole_index: data.hole_index,
        hole_count,
        course_name: course.name,
        course_difficulty: course.difficulty,
        course_dir: Some(data.course_dir),
        ball: data.ball,
        wind: data.wind,
        strokes: data.strokes,
        club: data.club,
        aim: data.aim,
        scorecard: data.scorecard,
        die_strength: data.die_strength,
        // Pas persisté (voir `Scorecard` et `die_strength` pour ce qui l'est
        // déjà) : une reprise de sauvegarde repart avec un historique
        // limité à la position actuelle, pas tout le trajet depuis le
        // départ du trou.
        shot_history: vec![data.ball],
        lang,
        last_die: None,
        last_shot: None,
        just_saved: false,
        quit_confirm: false,
        zoom: false,
    })
}

struct GameState {
    holes: Vec<Hole>,
    hole_index: usize,
    hole_count: usize,
    course_name: String,
    course_difficulty: u8,
    /// Dossier d'origine du parcours, `None` pour le parcours de secours
    /// généré en mémoire (pas de fichier à sauvegarder).
    course_dir: Option<PathBuf>,
    ball: Pos,
    /// Direction et force du vent, tirées au hasard au chargement du trou
    /// (`random_wind()`) — affecte les coups (sauf le putt) dans
    /// `resolve_shot`/`preview_shot`.
    wind: Wind,
    strokes: u8,
    club: Club,
    aim: Direction,
    /// Scores des trous déjà quittés (ni le trou en cours, ni un trou
    /// terminé mais pas encore confirmé via `advance_hole` — rejouer un
    /// trou fini avec `restart_hole` ne laisse donc aucune trace ici).
    scorecard: Scorecard,
    /// Plafond choisi par le joueur pour le tirage du dé (3 à 6, voir
    /// `DIE_STRENGTH_FLOOR`, 6 = pas de plafond). Remis à 6 à chaque nouveau
    /// trou et à chaque changement de club — un réglage fin pour le
    /// club/trou courant, pas une préférence durable (voir
    /// `cycle_club`/`restart_hole`/`advance_hole`).
    die_strength: u8,
    /// Points d'arrêt de la balle sur le trou en cours (le départ inclus,
    /// puis l'atterrissage de chaque coup) — sert à retracer le parcours
    /// sur la carte une fois le trou terminé (`CourseView::path`). Remis à
    /// `[tee]` à chaque nouveau trou ou reprise du trou courant.
    shot_history: Vec<Pos>,
    lang: Lang,
    last_die: Option<u8>,
    last_shot: Option<ShotResult>,
    just_saved: bool,
    quit_confirm: bool,
    /// Zoom sur la carte, activé/désactivé par le joueur (touche `Z`) —
    /// désactivé par défaut.
    zoom: bool,
}

impl GameState {
    fn new(course_dir: Option<PathBuf>, course: Course, lang: Lang) -> Self {
        let hole_count = course.holes.len();
        assert!(hole_count > 0, "un parcours doit avoir au moins un trou");
        let holes = course.holes;
        let ball = holes[0].tee;
        let aim = Direction::towards(holes[0].tee, holes[0].hole_pos);
        GameState {
            holes,
            hole_index: 0,
            hole_count,
            course_name: course.name,
            course_difficulty: course.difficulty,
            course_dir,
            ball,
            wind: random_wind(),
            strokes: 0,
            club: Club::Driver,
            aim,
            scorecard: Scorecard::default(),
            die_strength: 6,
            shot_history: vec![ball],
            lang,
            last_die: None,
            last_shot: None,
            just_saved: false,
            quit_confirm: false,
            zoom: false,
        }
    }

    fn current_hole(&self) -> &Hole {
        &self.holes[self.hole_index]
    }

    fn play_shot(&mut self) {
        let die: u8 = rand::thread_rng().gen_range(1..=self.die_strength);
        let shot = Shot {
            club: self.club,
            direction: self.aim,
            die_roll: die,
        };
        let mut rng = rand::thread_rng();
        let result = resolve_shot(self.current_hole(), self.ball, shot, self.wind, &mut rng);
        self.strokes += 1 + result.penalty_strokes;
        self.ball = result.landing;
        self.shot_history.push(self.ball);
        self.aim = Direction::towards(self.ball, self.current_hole().hole_pos);
        self.last_die = Some(die);
        self.last_shot = Some(result);
        self.just_saved = false;
    }

    fn nudge_aim(&mut self, angle_delta: f32) {
        let current_angle = self.aim.dy.atan2(self.aim.dx);
        let new_angle = current_angle + angle_delta;
        self.aim = Direction {
            dx: new_angle.cos(),
            dy: new_angle.sin(),
        };
    }

    fn cycle_club(&mut self) {
        self.club = match self.club {
            Club::Driver => Club::Wood,
            Club::Wood => Club::Hybrid,
            Club::Hybrid => Club::Iron,
            Club::Iron => Club::Wedge,
            Club::Wedge => Club::Putter,
            Club::Putter => Club::Driver,
        };
        self.die_strength = 6;
    }

    /// Comme `cycle_club`, dans l'autre sens (`Shift+Tab`) — pratique pour
    /// revenir au club précédent sans faire tout le tour.
    fn cycle_club_reverse(&mut self) {
        self.club = match self.club {
            Club::Driver => Club::Putter,
            Club::Wood => Club::Driver,
            Club::Hybrid => Club::Wood,
            Club::Iron => Club::Hybrid,
            Club::Wedge => Club::Iron,
            Club::Putter => Club::Wedge,
        };
        self.die_strength = 6;
    }

    /// Ajuste le plafond de dé (`+`/`-`), borné entre `DIE_STRENGTH_FLOOR`
    /// et 6. Le plancher évite qu'un plafond trop bas (1 ou 2) ne rende un
    /// putt inutile : à plafond 1, un Putter ne peut plus parcourir que 0.5
    /// case, souvent pas assez pour atteindre le trou.
    fn adjust_die_strength(&mut self, delta: i8) {
        let current = self.die_strength as i8;
        self.die_strength = (current + delta).clamp(DIE_STRENGTH_FLOOR as i8, 6) as u8;
    }

    /// Vrai si le dernier coup a atteint le trou — le joueur ne peut plus
    /// rien jouer tant qu'il n'a pas choisi de rejouer ou de retourner au
    /// menu (voir `run_loop`).
    fn finished(&self) -> bool {
        self.last_shot.as_ref().is_some_and(|shot| shot.holed)
    }

    /// Remet le trou courant à zéro pour le rejouer (touche `R` une fois
    /// `finished()`) : position de balle, coups et club repartent comme au
    /// début, le vent est retiré au sort pour varier d'un essai à l'autre.
    /// Le scorecard n'est pas touché : tant que ce trou n'a pas été quitté
    /// via `advance_hole`, il n'y a jamais été ajouté.
    fn restart_hole(&mut self) {
        let (tee, hole_pos) = {
            let hole = self.current_hole();
            (hole.tee, hole.hole_pos)
        };
        self.ball = tee;
        self.shot_history = vec![tee];
        self.aim = Direction::towards(tee, hole_pos);
        self.wind = random_wind();
        self.strokes = 0;
        self.club = Club::Driver;
        self.die_strength = 6;
        self.last_die = None;
        self.last_shot = None;
        self.just_saved = false;
    }

    /// Enregistre le score du trou courant dans le scorecard puis passe au
    /// trou suivant (balle/visée/vent/coups/club réinitialisés comme un
    /// nouveau trou). Renvoie `false` sans rien charger de plus si le trou
    /// courant était le dernier du parcours — c'est à l'appelant de gérer
    /// la fin de partie (écran de scorecard complet, v0.2 phase 3).
    fn advance_hole(&mut self) -> bool {
        self.scorecard.push(HoleScore {
            strokes: self.strokes,
            par: self.current_hole().meta.par,
        });

        if self.hole_index + 1 >= self.hole_count {
            return false;
        }
        self.hole_index += 1;
        let (tee, hole_pos) = {
            let hole = self.current_hole();
            (hole.tee, hole.hole_pos)
        };
        self.ball = tee;
        self.shot_history = vec![tee];
        self.aim = Direction::towards(tee, hole_pos);
        self.wind = random_wind();
        self.strokes = 0;
        self.club = Club::Driver;
        self.die_strength = 6;
        self.last_die = None;
        self.last_shot = None;
        self.just_saved = false;
        true
    }
}

fn main() -> Result<()> {
    let data_root = resolve_data_root();
    let courses = discover_courses(&data_root)?;

    enable_raw_mode()?;
    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(backend)?;

    let mut lang = Lang::default();
    let result = (|| -> Result<()> {
        loop {
            let has_save = data_root.join(SAVE_PATH).exists();
            match select_course(&mut terminal, &courses, &mut lang, has_save)? {
                MenuChoice::Quit => return Ok(()),
                MenuChoice::Resume => {
                    let mut state = load_game(lang, &data_root)?;
                    match run_loop(&mut terminal, &mut state, &data_root)? {
                        LoopExit::Quit => return Ok(()),
                        LoopExit::BackToMenu => continue,
                        LoopExit::RoundComplete { course_name, course_difficulty, entries } => {
                            match show_scorecard(&mut terminal, &course_name, course_difficulty, &entries, &mut lang)? {
                                ScorecardExit::Quit => return Ok(()),
                                ScorecardExit::BackToMenu => continue,
                            }
                        }
                    }
                }
                MenuChoice::Play(index) => {
                    // Cloné plutôt que retiré de la liste : le joueur peut
                    // revenir au menu (touche `M` en fin de trou) et
                    // choisir/rejouer le même parcours sans relancer le jeu.
                    let (course_dir, course) = courses[index].clone();
                    let mut state = GameState::new(course_dir, course, lang);
                    match run_loop(&mut terminal, &mut state, &data_root)? {
                        LoopExit::Quit => return Ok(()),
                        LoopExit::BackToMenu => continue,
                        LoopExit::RoundComplete { course_name, course_difficulty, entries } => {
                            match show_scorecard(&mut terminal, &course_name, course_difficulty, &entries, &mut lang)? {
                                ScorecardExit::Quit => return Ok(()),
                                ScorecardExit::BackToMenu => continue,
                            }
                        }
                    }
                }
                MenuChoice::NewHole => {
                    let Some(launch) = pick_hole_to_build(&mut terminal, &mut lang, &data_root)? else {
                        continue;
                    };
                    let mut builder = match launch {
                        BuilderLaunch::New => {
                            let Some(b) = setup_builder(&mut terminal, &mut lang)? else {
                                continue;
                            };
                            b
                        }
                        BuilderLaunch::Existing(path, modify_in_place) => {
                            let raw = std::fs::read_to_string(&path)?;
                            let hole = Hole::parse(&raw)?;
                            BuilderState::from_existing_hole(
                                &hole,
                                if modify_in_place { Some(path) } else { None },
                                lang,
                            )
                        }
                    };
                    match run_builder(&mut terminal, &mut builder, &data_root)? {
                        BuilderExit::Quit => return Ok(()),
                        BuilderExit::BackToMenu => continue,
                    }
                }
            }
        }
    })();

    disable_raw_mode()?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)?;
    terminal.show_cursor()?;

    result
}

enum MenuChoice {
    Play(usize),
    Resume,
    NewHole,
    Quit,
}

/// Comment on quitte la boucle de jeu : arrêt complet du programme, retour
/// à l'écran de sélection de parcours (fin de trou, touche `M`), ou fin de
/// parcours confirmée sur le dernier trou (`Enter`) — dans ce dernier cas,
/// le détail des trous joués est renvoyé pour l'écran de scorecard.
enum LoopExit {
    Quit,
    BackToMenu,
    RoundComplete {
        course_name: String,
        course_difficulty: u8,
        entries: Vec<(String, HoleScore)>,
    },
}

/// Sortie de l'écran de fin de partie (scorecard complet) : uniquement
/// quitter ou revenir au menu, jamais "trou suivant" (ce n'est plus une
/// partie en cours à ce stade).
enum ScorecardExit {
    Quit,
    BackToMenu,
}

/// Écran de sélection de parcours.
fn select_course<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    courses: &[(Option<PathBuf>, Course)],
    lang: &mut Lang,
    has_save: bool,
) -> Result<MenuChoice> {
    let course_refs: Vec<&Course> = courses.iter().map(|(_, c)| c).collect();
    let mut selected = 0usize;
    let mut quit_confirm = false;
    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                CourseMenuState {
                    lang: *lang,
                    courses: &course_refs,
                    selected,
                    has_save,
                    quit_confirm,
                },
                frame.size(),
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if quit_confirm {
                            return Ok(MenuChoice::Quit);
                        }
                        quit_confirm = true;
                        continue;
                    }
                    KeyCode::Char('c') | KeyCode::Char('C') if has_save => {
                        return Ok(MenuChoice::Resume)
                    }
                    KeyCode::Char('e') | KeyCode::Char('E') => return Ok(MenuChoice::NewHole),
                    KeyCode::Up => selected = selected.saturating_sub(1),
                    KeyCode::Down => selected = (selected + 1).min(course_refs.len().saturating_sub(1)),
                    KeyCode::Enter => return Ok(MenuChoice::Play(selected)),
                    KeyCode::Char('l') | KeyCode::Char('L') => *lang = lang.next(),
                    _ => {}
                }
                quit_confirm = false;
            }
        }
    }
}

fn run_loop<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut GameState,
    data_root: &Path,
) -> Result<LoopExit> {
    loop {
        let finished = state.finished();

        terminal.draw(|frame| {
            let columns = Layout::default()
                .direction(LayoutDirection::Horizontal)
                .constraints([Constraint::Length(28), Constraint::Min(0)])
                .split(frame.size());

            frame.render_widget(
                SidebarState {
                    lang: state.lang,
                    hole_meta: &state.current_hole().meta,
                    course_difficulty: state.course_difficulty,
                    hole_index: state.hole_index,
                    hole_count: state.hole_count,
                    scorecard: &state.scorecard,
                    strokes: state.strokes,
                    club: state.club,
                    die_strength: state.die_strength,
                    aim: state.aim,
                    wind: state.wind,
                    last_die: state.last_die,
                    last_shot: state.last_shot.as_ref(),
                    just_saved: state.just_saved,
                    quit_confirm: state.quit_confirm,
                    finished,
                },
                columns[0],
            );

            // Une fois le trou fini, plus rien à viser : l'aperçu de coup
            // cède la place au rappel du parcours joué (voir
            // `GameState::shot_history` / `CourseView::path`).
            let preview = if finished {
                None
            } else {
                Some(preview_shot(
                    state.current_hole(),
                    state.ball,
                    state.club,
                    state.aim,
                    state.die_strength,
                ))
            };
            let path = finished.then(|| state.shot_history.as_slice());
            frame.render_widget(
                CourseView {
                    hole: state.current_hole(),
                    ball: state.ball,
                    viewport: Viewport {
                        // -2 : la carte est maintenant encadrée (bordure).
                        width: columns[1].width.saturating_sub(2) as usize,
                        height: columns[1].height.saturating_sub(2) as usize,
                    },
                    preview,
                    path,
                    zoomed: state.zoom,
                },
                columns[1],
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if finished {
                    // Trou terminé : plus de visée/coup/sauvegarde. En plus
                    // de rejouer/menu/quitter : `N` passe au trou suivant
                    // s'il en reste un ; `Enter` sur le dernier trou termine
                    // la partie et bascule vers l'écran de scorecard complet.
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if state.quit_confirm {
                                return Ok(LoopExit::Quit);
                            }
                            state.quit_confirm = true;
                            continue;
                        }
                        KeyCode::Char('n') | KeyCode::Char('N')
                            if state.hole_index + 1 < state.hole_count =>
                        {
                            state.advance_hole();
                        }
                        KeyCode::Enter if state.hole_index + 1 >= state.hole_count => {
                            state.advance_hole(); // enregistre le score du dernier trou
                            let entries = state
                                .holes
                                .iter()
                                .zip(state.scorecard.holes.iter())
                                .map(|(hole, score)| (hole.meta.name.clone(), score.clone()))
                                .collect();
                            return Ok(LoopExit::RoundComplete {
                                course_name: state.course_name.clone(),
                                course_difficulty: state.course_difficulty,
                                entries,
                            });
                        }
                        KeyCode::Char('r') | KeyCode::Char('R') => state.restart_hole(),
                        KeyCode::Char('m') | KeyCode::Char('M') => return Ok(LoopExit::BackToMenu),
                        KeyCode::Char('l') | KeyCode::Char('L') => state.lang = state.lang.next(),
                        KeyCode::Char('z') | KeyCode::Char('Z') => state.zoom = !state.zoom,
                        _ => {}
                    }
                    state.quit_confirm = false;
                } else {
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if state.quit_confirm {
                                return Ok(LoopExit::Quit);
                            }
                            state.quit_confirm = true;
                            continue;
                        }
                        KeyCode::Char(' ') => state.play_shot(),
                        KeyCode::Tab => state.cycle_club(),
                        KeyCode::BackTab => state.cycle_club_reverse(),
                        KeyCode::Char('-') => state.adjust_die_strength(-1),
                        KeyCode::Char('+') => state.adjust_die_strength(1),
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            state.just_saved = save_game(state, data_root).is_ok();
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => state.lang = state.lang.next(),
                        KeyCode::Char('z') | KeyCode::Char('Z') => state.zoom = !state.zoom,
                        KeyCode::Left => state.nudge_aim(-0.1),
                        KeyCode::Right => state.nudge_aim(0.1),
                        _ => {}
                    }
                    state.quit_confirm = false;
                }
            }
        }
    }
}

/// Écran de fin de partie : détail trou par trou + total, affiché une fois
/// après `LoopExit::RoundComplete`, avant de revenir au menu.
fn show_scorecard<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    course_name: &str,
    course_difficulty: u8,
    entries: &[(String, HoleScore)],
    lang: &mut Lang,
) -> Result<ScorecardExit> {
    let mut quit_confirm = false;
    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                ScorecardView {
                    lang: *lang,
                    course_name,
                    entries,
                    course_difficulty,
                    quit_confirm,
                },
                frame.size(),
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Char('q') | KeyCode::Char('Q') => {
                        if quit_confirm {
                            return Ok(ScorecardExit::Quit);
                        }
                        quit_confirm = true;
                        continue;
                    }
                    KeyCode::Enter | KeyCode::Char('m') | KeyCode::Char('M') => {
                        return Ok(ScorecardExit::BackToMenu)
                    }
                    KeyCode::Char('l') | KeyCode::Char('L') => *lang = lang.next(),
                    _ => {}
                }
                quit_confirm = false;
            }
        }
    }
}

/// Comment on quitte l'écran d'édition : arrêt complet du programme, ou
/// retour au menu (qq, ou sauvegarde réussie qui ramène au menu).
enum BuilderExit {
    Quit,
    BackToMenu,
}

/// Le golf n'a pas vraiment de par en dessous de 3 — le reste du jeu
/// (distances de club, les autres parcours) est calibré pour des pars
/// réalistes (3 à 7+). Plancher appliqué au par choisi à l'en-tête du
/// builder (`setup_builder`) : sans lui, `Down` pouvait descendre jusqu'à 1,
/// une valeur qui n'a pas de sens golfique.
const MIN_HOLE_PAR: u8 = 3;

/// Résout un caractère tapé dans le builder vers un terrain, insensible à
/// la casse (`d` comme `D` pour le tee, `t` comme `T` pour l'arbre, etc.) —
/// seule la saisie interactive est tolérante à la casse ; le format
/// `.course` lui-même (`TerrainKind::from_char`) reste strict, inchangé.
fn terrain_from_builder_key(c: char) -> Option<TerrainKind> {
    TerrainKind::from_char(c.to_ascii_uppercase())
}

/// Dossier (suffixe relatif à `data_root`, voir `resolve_data_root`) où le
/// builder sauvegarde systématiquement chaque trou — une bibliothèque de
/// trous pas encore assignés à un parcours, en attendant le futur builder
/// de parcours (voir `ROADMAP.md`). Le joueur ne choisit jamais
/// l'emplacement lui-même, seulement un nom de fichier.
const HOLE_LIBRARY_DIR: &str = "courses/_library";

/// Nettoie un nom de fichier tapé à la main pour la bibliothèque de trous :
/// ne garde que les caractères alphanumériques ASCII, `-` et `_` (remplace
/// le reste, y compris espaces et séparateurs de chemin, par `_`) — élimine
/// au passage tout risque de remontée `..` ou de chemin absolu, puisque `/`
/// et `.` ne survivent jamais au nettoyage. Retombe sur "hole" si le
/// résultat est vide (saisie entièrement blanche).
fn sanitize_hole_filename(input: &str) -> String {
    let cleaned: String = input
        .trim()
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() || c == '-' || c == '_' { c } else { '_' })
        .collect();
    if cleaned.is_empty() {
        "hole".to_string()
    } else {
        cleaned
    }
}

/// Taille de grille suggérée à partir du par visé, cohérente avec les
/// distances déjà utilisées par `tools/make_hole_canvas.py` (rayons idéaux
/// tee-green par par) : un par court n'a pas besoin d'un canevas 100x60
/// entièrement hors-limites autour d'un tout petit trou. Le "petit côté"
/// est dérivé proportionnellement du "grand côté" (rapport 3/5, celui du
/// canevas complet 100x60 — `COURSE_HEIGHT`/`COURSE_WIDTH`) plutôt que
/// d'être une constante fixe indépendante du par. Le canevas n'étant pas
/// carré (100x60), le plafond du "grand côté" dépend de l'orientation
/// (100 en horizontal, seulement 60 en vertical) : il est donc clampé
/// *avant* d'en dériver le petit côté, pas après — sinon un par 7+ en
/// vertical donnerait un carré 60x60 (grand côté rabattu de 100 à 60, puis
/// petit côté calculé sur l'ancienne valeur 100 non clampée) plutôt qu'un
/// couloir 36x60 correctement proportionné. Avec ce clampage dans le bon
/// ordre, un par 7+ en horizontal atteint exactement le canevas 100x60
/// plein (aucune taille déclarée n'est alors plus restrictive que le
/// comportement par défaut).
fn suggested_declared_size(par: u8, orientation: BuilderOrientation) -> (usize, usize) {
    let raw_long_side = match par {
        0..=MIN_HOLE_PAR => 25,
        4 => 55,
        5 => 70,
        6 => 85,
        _ => 100,
    };
    let (long_max, short_max) = match orientation {
        BuilderOrientation::Horizontal => (COURSE_WIDTH, COURSE_HEIGHT),
        BuilderOrientation::Vertical => (COURSE_HEIGHT, COURSE_WIDTH),
    };
    let long_side = raw_long_side.min(long_max);
    let short_side = (long_side * 3 / 5).min(short_max);
    match orientation {
        BuilderOrientation::Horizontal => (long_side, short_side),
        BuilderOrientation::Vertical => (short_side, long_side),
    }
}

/// État de l'éditeur de trou (voir `ROADMAP.md`, "Builder de trous"). Édite
/// un seul fichier `.course` à la fois : grille locale (taille déclarée à
/// l'en-tête, pas nécessairement 100x60 pleine), curseur, historique
/// d'annulation. La sauvegarde sérialise directement cet état — pas besoin
/// de construire un `Hole` complet, qui exigerait un tee/trou déjà
/// validés — et valide le résultat via `Hole::parse` avant d'écrire sur le
/// disque : sauvegarder, c'est réussir ce parsing.
struct BuilderState {
    name: String,
    par: u8,
    orientation: BuilderOrientation,
    width: usize,
    height: usize,
    tiles: Vec<Vec<TerrainKind>>,
    cursor: Pos,
    undo_stack: Vec<(Pos, TerrainKind)>,
    mode: BuilderMode,
    text_input: String,
    /// Chemin en attente de confirmation d'écrasement
    /// (`BuilderMode::ConfirmOverwrite`) — `None` en dehors de ce mode.
    pending_save_path: Option<PathBuf>,
    message: Option<String>,
    lang: Lang,
    quit_confirm: bool,
    /// Deuxième pression sur `Échap` en attente (retour au menu, distinct de
    /// `quit_confirm` qui quitte l'application entière) — annulée par
    /// n'importe quelle autre touche, y compris `S` qui bascule plutôt vers
    /// la sauvegarde (permet de sauvegarder le travail en cours avant de
    /// sortir, sans logique dédiée : la frappe `S` habituelle prend le relai
    /// normalement et cette confirmation se réinitialise au passage).
    exit_confirm: bool,
    /// Chemin d'origine si ce trou a été chargé en mode "Modifier" (voir
    /// `pick_hole_to_build`/`load_builder_from_file`) : `S` réécrit alors
    /// directement ce fichier, sans redemander de nom ni passer par
    /// `BuilderMode::ConfirmOverwrite` (c'est justement le fichier que le
    /// joueur a choisi d'éditer). `None` pour un trou neuf ou chargé en
    /// "Dupliquer" — sauvegarde normale dans la bibliothèque à chaque fois.
    source_path: Option<PathBuf>,
}

impl BuilderState {
    fn new(par: u8, orientation: BuilderOrientation, lang: Lang) -> Self {
        let (width, height) = suggested_declared_size(par, orientation);
        let name = match lang {
            Lang::En => "New hole".to_string(),
            Lang::Fr => "Nouveau trou".to_string(),
        };
        BuilderState {
            name,
            par,
            orientation,
            width,
            height,
            tiles: vec![vec![TerrainKind::OutOfBounds; width]; height],
            cursor: Pos { x: 0, y: 0 },
            undo_stack: Vec::new(),
            mode: BuilderMode::Drawing,
            text_input: String::new(),
            pending_save_path: None,
            message: None,
            lang,
            quit_confirm: false,
            exit_confirm: false,
            source_path: None,
        }
    }

    /// Charge un trou existant (fichier `.course` déjà parsé) dans un
    /// `BuilderState` pour édition — voir `ROADMAP.md`, phase "charger un
    /// fichier existant". `source_path` fixe le comportement de `S` :
    /// `Some` réécrit directement ce fichier ("Modifier"), `None` redemande
    /// un nom à chaque sauvegarde comme un trou neuf ("Dupliquer").
    /// L'orientation est déduite du rapport largeur/hauteur de la grille
    /// chargée (déterminante seulement pour le sens d'avancée automatique
    /// de la frappe — la taille elle-même reste celle du fichier).
    fn from_existing_hole(hole: &Hole, source_path: Option<PathBuf>, lang: Lang) -> Self {
        let width = hole.meta.width.unwrap_or(COURSE_WIDTH);
        let height = hole.meta.height.unwrap_or(COURSE_HEIGHT);
        let orientation = if width >= height {
            BuilderOrientation::Horizontal
        } else {
            BuilderOrientation::Vertical
        };
        BuilderState {
            name: hole.meta.name.clone(),
            par: hole.meta.par,
            orientation,
            width,
            height,
            tiles: hole.local_tiles(),
            cursor: Pos { x: 0, y: 0 },
            undo_stack: Vec::new(),
            mode: BuilderMode::Drawing,
            text_input: String::new(),
            pending_save_path: None,
            message: None,
            lang,
            quit_confirm: false,
            exit_confirm: false,
            source_path,
        }
    }

    /// Peint la case sous le curseur avec `terrain` (empile l'ancienne
    /// valeur pour l'annulation) puis avance le curseur d'un cran dans le
    /// sens de `orientation` — ligne par ligne en horizontal, colonne par
    /// colonne en vertical, comme du texte avec retour à la ligne. S'arrête
    /// net à la dernière case plutôt que de boucler au début.
    fn type_terrain(&mut self, terrain: TerrainKind) {
        let old = self.tiles[self.cursor.y][self.cursor.x];
        self.undo_stack.push((self.cursor, old));
        self.tiles[self.cursor.y][self.cursor.x] = terrain;
        self.advance_cursor();
    }

    fn advance_cursor(&mut self) {
        match self.orientation {
            BuilderOrientation::Horizontal => {
                if self.cursor.x + 1 < self.width {
                    self.cursor.x += 1;
                } else if self.cursor.y + 1 < self.height {
                    self.cursor.y += 1;
                    self.cursor.x = 0;
                }
            }
            BuilderOrientation::Vertical => {
                if self.cursor.y + 1 < self.height {
                    self.cursor.y += 1;
                } else if self.cursor.x + 1 < self.width {
                    self.cursor.x += 1;
                    self.cursor.y = 0;
                }
            }
        }
    }

    /// Dépile la dernière case peinte, restaure son ancien terrain, et
    /// replace le curseur dessus.
    fn undo(&mut self) {
        if let Some((pos, old)) = self.undo_stack.pop() {
            self.tiles[pos.y][pos.x] = old;
            self.cursor = pos;
        }
    }

    fn move_cursor(&mut self, dx: i32, dy: i32) {
        let nx = self.cursor.x as i32 + dx;
        let ny = self.cursor.y as i32 + dy;
        if nx >= 0 && (nx as usize) < self.width {
            self.cursor.x = nx as usize;
        }
        if ny >= 0 && (ny as usize) < self.height {
            self.cursor.y = ny as usize;
        }
    }

    /// Sérialise l'état courant vers le format `.course` (frontmatter YAML +
    /// grille ASCII) — même structure que `Hole::to_course_string`, mais
    /// construite directement depuis l'état d'édition puisqu'aucun `Hole`
    /// complet n'existe encore avant que la sauvegarde ait réussi.
    fn to_course_raw(&self) -> String {
        let declare_size = self.width != COURSE_WIDTH || self.height != COURSE_HEIGHT;
        let meta = HoleMeta {
            name: self.name.clone(),
            par: self.par,
            description: None,
            width: declare_size.then_some(self.width),
            height: declare_size.then_some(self.height),
        };
        let mut meta_yaml =
            serde_yaml::to_string(&meta).expect("HoleMeta se sérialise toujours en YAML");
        if !meta_yaml.ends_with('\n') {
            meta_yaml.push('\n');
        }
        let lines: Vec<String> = self
            .tiles
            .iter()
            .map(|row| row.iter().map(|t| t.to_char()).collect::<String>())
            .collect();
        format!("{}---\n{}\n", meta_yaml, lines.join("\n"))
    }
}

/// Liste tous les fichiers `.course` trouvés sous `root/*/` (un seul niveau
/// de sous-dossiers, comme `Course::discover` — cohérent avec la structure
/// `courses/<parcours>/*.course` et `courses/_library/*.course`). Pas
/// seulement ceux référencés par un `course.yaml` : sert au builder pour
/// retrouver aussi un trou orphelin de la bibliothèque. Triés pour un
/// affichage stable.
fn discover_hole_files(root: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let Ok(entries) = std::fs::read_dir(root) else {
        return files;
    };
    for entry in entries.flatten() {
        let dir = entry.path();
        if !dir.is_dir() {
            continue;
        }
        let Ok(sub_entries) = std::fs::read_dir(&dir) else {
            continue;
        };
        for sub in sub_entries.flatten() {
            let path = sub.path();
            if path.extension().and_then(|e| e.to_str()) == Some("course") {
                files.push(path);
            }
        }
    }
    files.sort();
    files
}

/// Choix fait par le joueur à l'écran de sélection du builder
/// (`pick_hole_to_build`) : un trou neuf, ou un trou existant chargé pour
/// modification sur place (réécrit le même fichier) ou duplication
/// (nouveau nom demandé, comme un trou neuf) — voir `ROADMAP.md`, phase
/// "charger un fichier existant".
enum BuilderLaunch {
    New,
    /// `true` = modifier sur place, `false` = dupliquer.
    Existing(PathBuf, bool),
}

/// Écran d'entrée du builder : "+ Nouveau trou" en premier, puis chaque
/// fichier `.course` trouvé sous `courses/` (`discover_hole_files`). Choisir
/// un fichier existant demande ensuite "Modifier" ou "Dupliquer" avant de
/// continuer. `None` si annulé (Échap, retour au menu).
fn pick_hole_to_build<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    lang: &mut Lang,
    data_root: &Path,
) -> Result<Option<BuilderLaunch>> {
    let courses_root = data_root.join("courses");
    let files = discover_hole_files(&courses_root);
    let mut selected = 0usize;
    let total = files.len() + 1; // +1 pour "+ Nouveau trou"
    let mut confirm_target: Option<PathBuf> = None;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                HolePickerView {
                    lang: *lang,
                    files: &files,
                    selected,
                    confirm_target: confirm_target.as_deref(),
                    courses_root: &courses_root,
                },
                frame.size(),
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if confirm_target.is_some() {
                    match key.code {
                        KeyCode::Char('m') | KeyCode::Char('M') => {
                            let path = confirm_target.take().unwrap();
                            return Ok(Some(BuilderLaunch::Existing(path, true)));
                        }
                        KeyCode::Char('d') | KeyCode::Char('D') => {
                            let path = confirm_target.take().unwrap();
                            return Ok(Some(BuilderLaunch::Existing(path, false)));
                        }
                        KeyCode::Esc => confirm_target = None,
                        _ => {}
                    }
                } else {
                    match key.code {
                        KeyCode::Esc => return Ok(None),
                        KeyCode::Up => selected = selected.saturating_sub(1),
                        KeyCode::Down => selected = (selected + 1).min(total.saturating_sub(1)),
                        KeyCode::Enter => {
                            if selected == 0 {
                                return Ok(Some(BuilderLaunch::New));
                            }
                            confirm_target = Some(files[selected - 1].clone());
                        }
                        KeyCode::Char('l') | KeyCode::Char('L') => *lang = lang.next(),
                        _ => {}
                    }
                }
            }
        }
    }
}

/// Petit écran d'en-tête avant de dessiner un nouveau trou : demande le par
/// visé (seul champ requis) et l'orientation, qui suggèrent ensemble une
/// taille de grille. `None` si annulé (Échap, retour au menu).
fn setup_builder<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    lang: &mut Lang,
) -> Result<Option<BuilderState>> {
    let mut par: u8 = 4;
    let mut orientation = BuilderOrientation::Horizontal;

    loop {
        terminal.draw(|frame| {
            frame.render_widget(
                BuilderSetupView {
                    lang: *lang,
                    par,
                    orientation,
                    grid_size: suggested_declared_size(par, orientation),
                },
                frame.size(),
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match key.code {
                    KeyCode::Esc => return Ok(None),
                    KeyCode::Enter => return Ok(Some(BuilderState::new(par, orientation, *lang))),
                    KeyCode::Up => par = (par + 1).min(9),
                    KeyCode::Down => par = par.saturating_sub(1).max(MIN_HOLE_PAR),
                    KeyCode::Left | KeyCode::Right => orientation = orientation.toggled(),
                    KeyCode::Char('l') | KeyCode::Char('L') => *lang = lang.next(),
                    _ => {}
                }
            }
        }
    }
}

/// Boucle principale de l'écran d'édition : rendu (en-tête + grille),
/// frappe directe des caractères de terrain (peinture + avancée
/// automatique), déplacement libre au clavier, annulation, renommage,
/// sauvegarde.
fn run_builder<B: ratatui::backend::Backend>(
    terminal: &mut Terminal<B>,
    state: &mut BuilderState,
    data_root: &Path,
) -> Result<BuilderExit> {
    loop {
        terminal.draw(|frame| {
            let rows = Layout::default()
                .direction(LayoutDirection::Vertical)
                .constraints([Constraint::Length(7), Constraint::Min(0)])
                .split(frame.size());

            frame.render_widget(
                BuilderHeaderView {
                    lang: state.lang,
                    name: &state.name,
                    par: state.par,
                    orientation: state.orientation,
                    cursor: state.cursor,
                    grid_width: state.width,
                    grid_height: state.height,
                    mode: state.mode,
                    text_input: &state.text_input,
                    pending_save_name: state
                        .pending_save_path
                        .as_ref()
                        .and_then(|p| p.file_stem())
                        .and_then(|s| s.to_str()),
                    message: state.message.as_deref(),
                    quit_confirm: state.quit_confirm,
                    exit_confirm: state.exit_confirm,
                },
                rows[0],
            );

            frame.render_widget(
                BuilderView {
                    tiles: &state.tiles,
                    cursor: state.cursor,
                    viewport_w: rows[1].width.saturating_sub(2) as usize,
                    viewport_h: rows[1].height.saturating_sub(2) as usize,
                },
                rows[1],
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                match state.mode {
                    BuilderMode::Drawing => {
                        state.message = None;
                        match key.code {
                            KeyCode::Char('q') | KeyCode::Char('Q') => {
                                if state.quit_confirm {
                                    return Ok(BuilderExit::Quit);
                                }
                                state.quit_confirm = true;
                                state.exit_confirm = false;
                                continue;
                            }
                            KeyCode::Esc => {
                                if state.exit_confirm {
                                    return Ok(BuilderExit::BackToMenu);
                                }
                                state.exit_confirm = true;
                                state.quit_confirm = false;
                                continue;
                            }
                            KeyCode::Up => state.move_cursor(0, -1),
                            KeyCode::Down => state.move_cursor(0, 1),
                            KeyCode::Left => state.move_cursor(-1, 0),
                            KeyCode::Right => state.move_cursor(1, 0),
                            KeyCode::Char('u') | KeyCode::Char('U') => state.undo(),
                            KeyCode::Char('n') | KeyCode::Char('N') => {
                                state.text_input.clear();
                                state.mode = BuilderMode::EditingName;
                            }
                            KeyCode::Char('s') | KeyCode::Char('S') => {
                                if let Some(path) = state.source_path.clone() {
                                    finish_save(state, path);
                                } else {
                                    state.text_input.clear();
                                    state.mode = BuilderMode::EnteringSaveName;
                                }
                            }
                            KeyCode::Char(c) => {
                                if let Some(terrain) = terrain_from_builder_key(c) {
                                    state.type_terrain(terrain);
                                }
                            }
                            _ => {}
                        }
                        state.quit_confirm = false;
                        state.exit_confirm = false;
                    }
                    BuilderMode::EditingName => match key.code {
                        KeyCode::Enter => {
                            let trimmed = state.text_input.trim();
                            if !trimmed.is_empty() {
                                state.name = trimmed.to_string();
                            }
                            state.mode = BuilderMode::Drawing;
                        }
                        KeyCode::Esc => state.mode = BuilderMode::Drawing,
                        KeyCode::Backspace => {
                            state.text_input.pop();
                        }
                        KeyCode::Char(c) => state.text_input.push(c),
                        _ => {}
                    },
                    BuilderMode::EnteringSaveName => match key.code {
                        KeyCode::Enter => {
                            if state.text_input.trim().is_empty() {
                                state.mode = BuilderMode::Drawing;
                                continue;
                            }
                            let base_name = sanitize_hole_filename(&state.text_input);
                            let path =
                                data_root.join(HOLE_LIBRARY_DIR).join(format!("{base_name}.course"));
                            if path.exists() {
                                // Ne jamais ajouter un compteur automatique
                                // ici : ça empêcherait de mettre à jour un
                                // trou déjà sauvegardé (chaque sauvegarde
                                // créerait un nouveau fichier au lieu de
                                // remplacer l'ancien). On demande plutôt
                                // confirmation avant d'écraser.
                                state.pending_save_path = Some(path);
                                state.mode = BuilderMode::ConfirmOverwrite;
                            } else {
                                finish_save(state, path);
                            }
                        }
                        KeyCode::Esc => state.mode = BuilderMode::Drawing,
                        KeyCode::Backspace => {
                            state.text_input.pop();
                        }
                        KeyCode::Char(c) => state.text_input.push(c),
                        _ => {}
                    },
                    BuilderMode::ConfirmOverwrite => match key.code {
                        KeyCode::Enter | KeyCode::Char('y') | KeyCode::Char('Y') => {
                            if let Some(path) = state.pending_save_path.take() {
                                finish_save(state, path);
                            } else {
                                state.mode = BuilderMode::Drawing;
                            }
                        }
                        KeyCode::Esc | KeyCode::Char('n') | KeyCode::Char('N') => {
                            state.pending_save_path = None;
                            state.mode = BuilderMode::EnteringSaveName;
                        }
                        _ => {}
                    },
                }
            }
        }
    }
}

/// Sérialise `state` et valide le résultat via `Hole::parse` avant de
/// l'écrire sur le disque — sauvegarder, c'est réussir ce parsing (voir
/// `BuilderState::to_course_raw`). N'écrit rien si la validation échoue.
/// Crée le dossier parent de `path` s'il n'existe pas encore (typiquement
/// `courses/_library/` à la première sauvegarde).
fn save_builder(state: &BuilderState, path: &Path) -> Result<()> {
    let raw = state.to_course_raw();
    Hole::parse(&raw)?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, raw)?;
    Ok(())
}

/// Termine la sauvegarde vers `path` (déjà résolu, collision déjà tranchée
/// par l'appelant) : sauvegarde effective, message de succès/erreur, et
/// retour au mode dessin — partagé entre la sauvegarde directe (pas de
/// collision) et la confirmation d'écrasement.
fn finish_save(state: &mut BuilderState, path: PathBuf) {
    match save_builder(state, &path) {
        Ok(()) => {
            state.message = Some(match state.lang {
                Lang::En => format!("Saved as {} — not yet part of any course.", path.display()),
                Lang::Fr => format!(
                    "Sauvegardé sous {} — pas encore inclus dans un parcours.",
                    path.display()
                ),
            });
        }
        Err(err) => {
            state.message = Some(err.to_string());
        }
    }
    state.mode = BuilderMode::Drawing;
}

#[cfg(test)]
mod tests {
    use super::*;

    /// État de jeu minimal pour les tests, basé sur le parcours de secours
    /// généré en mémoire (pas besoin de fichier `.course` sur disque).
    fn test_state() -> GameState {
        GameState::new(None, fallback_course().unwrap(), Lang::default())
    }

    /// Parcours de test à 2 trous (pars différents), pour tester
    /// l'enchaînement (`advance_hole`) sans dépendre d'un fichier sur disque.
    fn two_hole_course() -> Course {
        fn hole_raw(par: u8) -> String {
            let width = crate::core::COURSE_WIDTH;
            let height = crate::core::COURSE_HEIGHT;
            let mut lines = Vec::with_capacity(height);
            for y in 0..height {
                let mut row = vec!['.'; width];
                if y == 0 {
                    row[0] = 'D';
                }
                if y == height - 1 {
                    row[width - 1] = 'H';
                }
                lines.push(row.into_iter().collect::<String>());
            }
            format!("name: \"Test\"\npar: {}\n---\n{}\n", par, lines.join("\n"))
        }
        Course {
            name: "Parcours de test".to_string(),
            difficulty: 1,
            holes: vec![
                Hole::parse(&hole_raw(3)).unwrap(),
                Hole::parse(&hole_raw(5)).unwrap(),
            ],
        }
    }

    fn two_hole_test_state() -> GameState {
        GameState::new(None, two_hole_course(), Lang::default())
    }

    #[test]
    fn cycle_club_goes_through_every_club_and_wraps_around() {
        let mut state = test_state();
        assert_eq!(state.club, Club::Driver);
        for expected in [
            Club::Wood,
            Club::Hybrid,
            Club::Iron,
            Club::Wedge,
            Club::Putter,
            Club::Driver,
        ] {
            state.cycle_club();
            assert_eq!(state.club, expected);
        }
    }

    #[test]
    fn cycle_club_reverse_goes_backwards_and_wraps_around() {
        let mut state = test_state();
        assert_eq!(state.club, Club::Driver);
        for expected in [
            Club::Putter,
            Club::Wedge,
            Club::Iron,
            Club::Hybrid,
            Club::Wood,
            Club::Driver,
        ] {
            state.cycle_club_reverse();
            assert_eq!(state.club, expected);
        }
    }

    #[test]
    fn cycle_club_reverse_undoes_cycle_club() {
        let mut state = test_state();
        for _ in 0..6 {
            let before = state.club;
            state.cycle_club();
            state.cycle_club_reverse();
            assert_eq!(state.club, before, "reverse doit annuler le cycle direct");
            state.cycle_club(); // avance pour tester le club suivant
        }
    }

    #[test]
    fn nudge_aim_rotates_and_stays_normalized() {
        let mut state = test_state();
        let before = state.aim;
        state.nudge_aim(0.3);
        let magnitude = (state.aim.dx.powi(2) + state.aim.dy.powi(2)).sqrt();
        assert!((magnitude - 1.0).abs() < 1e-4, "la direction doit rester normalisée");
        assert!(
            (state.aim.dx - before.dx).abs() > 1e-4 || (state.aim.dy - before.dy).abs() > 1e-4,
            "la direction doit avoir changé"
        );
    }

    #[test]
    fn play_shot_increments_strokes_and_records_the_shot() {
        let mut state = test_state();
        assert_eq!(state.strokes, 0);
        assert!(state.last_shot.is_none());

        state.play_shot();

        assert!(state.strokes >= 1, "au moins un coup doit être compté");
        assert!(state.last_die.is_some());
        assert!(state.last_shot.is_some());
    }

    #[test]
    fn shot_history_starts_at_the_tee_and_grows_with_each_shot() {
        let mut state = test_state();
        let tee = state.current_hole().tee;
        assert_eq!(state.shot_history, vec![tee]);

        state.play_shot();
        assert_eq!(state.shot_history.len(), 2);
        assert_eq!(*state.shot_history.last().unwrap(), state.ball);

        state.play_shot();
        assert_eq!(state.shot_history.len(), 3);
        assert_eq!(*state.shot_history.last().unwrap(), state.ball);
    }

    #[test]
    fn shot_history_resets_on_restart_and_advance() {
        let mut state = two_hole_test_state();
        let tee = state.current_hole().tee;
        state.play_shot();
        assert!(state.shot_history.len() > 1);

        state.restart_hole();
        assert_eq!(state.shot_history, vec![tee]);

        state.play_shot();
        assert!(state.shot_history.len() > 1);

        state.advance_hole();
        let next_tee = state.current_hole().tee;
        assert_eq!(state.shot_history, vec![next_tee]);
    }

    #[test]
    fn finished_reflects_the_holed_flag_of_the_last_shot() {
        let mut state = test_state();
        assert!(!state.finished(), "aucun coup joué, pas encore fini");

        state.last_shot = Some(ShotResult {
            landing: state.ball,
            landing_terrain: crate::core::TerrainKind::Fairway,
            penalty_strokes: 0,
            holed: false,
            dropped: false,
        });
        assert!(!state.finished(), "coup sur le fairway, pas fini");

        state.last_shot = Some(ShotResult {
            landing: state.ball,
            landing_terrain: crate::core::TerrainKind::Hole,
            penalty_strokes: 0,
            holed: true,
            dropped: false,
        });
        assert!(state.finished(), "coup dans le trou, doit être fini");
    }

    #[test]
    fn restart_hole_resets_progress_but_keeps_the_hole() {
        let mut state = test_state();
        state.play_shot();
        state.cycle_club();
        let tee = state.current_hole().tee;

        state.restart_hole();

        assert_eq!(state.ball, tee);
        assert_eq!(state.strokes, 0);
        assert_eq!(state.club, Club::Driver);
        assert!(state.last_die.is_none());
        assert!(state.last_shot.is_none());
        assert!(!state.just_saved);
        assert!(!state.finished());
    }

    #[test]
    fn restart_hole_does_not_touch_the_scorecard() {
        let mut state = test_state();
        state.play_shot();

        state.restart_hole();

        assert!(
            state.scorecard.holes.is_empty(),
            "rejouer un trou non quitté ne doit rien enregistrer au scorecard"
        );
    }

    #[test]
    fn advance_hole_records_score_and_loads_the_next_hole() {
        let mut state = two_hole_test_state();
        assert_eq!(state.hole_index, 0);
        assert_eq!(state.current_hole().meta.par, 3);

        state.strokes = 4;
        let advanced = state.advance_hole();

        assert!(advanced, "il reste un deuxième trou");
        assert_eq!(state.hole_index, 1);
        assert_eq!(state.current_hole().meta.par, 5);
        assert_eq!(state.strokes, 0, "les coups repartent à zéro sur le nouveau trou");
        assert_eq!(state.ball, state.current_hole().tee);
        assert_eq!(state.club, Club::Driver);
        assert_eq!(state.scorecard.holes.len(), 1);
        assert_eq!(state.scorecard.holes[0].strokes, 4);
        assert_eq!(state.scorecard.holes[0].par, 3);
    }

    #[test]
    fn advance_hole_on_the_last_hole_records_score_but_reports_no_more_holes() {
        let mut state = two_hole_test_state();
        state.advance_hole(); // passe au trou 2 (le dernier)
        state.strokes = 6;

        let advanced = state.advance_hole();

        assert!(!advanced, "c'était le dernier trou du parcours");
        assert_eq!(state.hole_index, 1, "l'index ne doit pas dépasser le dernier trou");
        assert_eq!(state.scorecard.holes.len(), 2);
        assert_eq!(state.scorecard.holes[1].strokes, 6);
        assert_eq!(state.scorecard.holes[1].par, 5);
    }

    #[test]
    fn adjust_die_strength_is_clamped_between_the_floor_and_6() {
        let mut state = test_state();
        assert_eq!(state.die_strength, 6, "pas de plafond par défaut");

        for _ in 0..10 {
            state.adjust_die_strength(-1);
        }
        assert_eq!(state.die_strength, DIE_STRENGTH_FLOOR, "ne doit jamais descendre sous le plancher");

        for _ in 0..10 {
            state.adjust_die_strength(1);
        }
        assert_eq!(state.die_strength, 6, "ne doit jamais dépasser 6");
    }

    #[test]
    fn play_shot_respects_the_die_strength_cap() {
        let mut state = test_state();
        state.adjust_die_strength(-10); // plafonne au plancher (3)
        assert_eq!(state.die_strength, DIE_STRENGTH_FLOOR);

        for _ in 0..20 {
            state.play_shot();
            let die = state.last_die.expect("un dé a été tiré");
            assert!(
                die <= DIE_STRENGTH_FLOOR,
                "le dé plafonné à {DIE_STRENGTH_FLOOR} ne doit jamais dépasser ce plafond, obtenu {die}"
            );
        }
    }

    #[test]
    fn changing_club_resets_the_die_strength_cap() {
        let mut state = test_state();
        state.adjust_die_strength(-3);
        assert_eq!(state.die_strength, DIE_STRENGTH_FLOOR);

        state.cycle_club();
        assert_eq!(state.die_strength, 6, "changer de club (Tab) doit remettre le plafond à 6");

        state.adjust_die_strength(-2);
        assert_eq!(state.die_strength, 4);

        state.cycle_club_reverse();
        assert_eq!(
            state.die_strength, 6,
            "changer de club (Shift+Tab) doit aussi remettre le plafond à 6"
        );
    }

    #[test]
    fn new_hole_resets_the_die_strength_cap() {
        let mut state = two_hole_test_state();
        state.adjust_die_strength(-4);
        assert_eq!(state.die_strength, DIE_STRENGTH_FLOOR);

        state.advance_hole();
        assert_eq!(state.die_strength, 6, "un nouveau trou doit remettre le plafond à 6");

        state.adjust_die_strength(-4);
        assert_eq!(state.die_strength, DIE_STRENGTH_FLOOR);

        state.restart_hole();
        assert_eq!(state.die_strength, 6, "rejouer le trou doit aussi remettre le plafond à 6");
    }

    #[test]
    fn embedded_courses_parse_and_match_the_real_courses_on_disk() {
        // Garde-fou : si `courses/demo/` ou `courses/quick3/` change sans
        // que le contenu embarqué (`include_str!`) suive, ce test doit le
        // détecter avant publication plutôt qu'un joueur sans dossier
        // `courses/` sur disque.
        let embedded = embedded_courses().expect("le contenu embarqué doit parser");
        assert_eq!(embedded.len(), 2);

        let demo = &embedded[0];
        assert_eq!(demo.name, "Parcours de démonstration");
        assert_eq!(demo.holes.len(), 1);
        assert_eq!(demo.holes[0].meta.par, 4);

        let quick3 = &embedded[1];
        assert_eq!(quick3.name, "Quick 3");
        assert_eq!(quick3.holes.len(), 3);
        assert_eq!(
            quick3.holes.iter().map(|h| h.meta.par as u32).sum::<u32>(),
            12
        );
    }

    #[test]
    fn terrain_from_builder_key_is_case_insensitive_for_letters() {
        assert_eq!(terrain_from_builder_key('d'), Some(TerrainKind::Tee));
        assert_eq!(terrain_from_builder_key('D'), Some(TerrainKind::Tee));
        assert_eq!(terrain_from_builder_key('t'), Some(TerrainKind::Tree));
        assert_eq!(terrain_from_builder_key('T'), Some(TerrainKind::Tree));
        assert_eq!(terrain_from_builder_key('g'), Some(TerrainKind::Green));
        assert_eq!(terrain_from_builder_key('h'), Some(TerrainKind::Hole));
        assert_eq!(terrain_from_builder_key('b'), Some(TerrainKind::Bunker));
        assert_eq!(terrain_from_builder_key('x'), Some(TerrainKind::PenaltyZone));
    }

    #[test]
    fn terrain_from_builder_key_still_recognizes_non_letter_symbols() {
        assert_eq!(terrain_from_builder_key('.'), Some(TerrainKind::Fairway));
        assert_eq!(terrain_from_builder_key('='), Some(TerrainKind::Rough));
        assert_eq!(terrain_from_builder_key('~'), Some(TerrainKind::Water));
        assert_eq!(terrain_from_builder_key(' '), Some(TerrainKind::OutOfBounds));
    }

    #[test]
    fn terrain_from_builder_key_rejects_unrecognized_chars() {
        assert_eq!(terrain_from_builder_key('q'), None);
        assert_eq!(terrain_from_builder_key('1'), None);
    }

    #[test]
    fn sanitize_hole_filename_keeps_only_safe_characters() {
        assert_eq!(sanitize_hole_filename("My Hole #1!"), "My_Hole__1_");
        assert_eq!(sanitize_hole_filename("valid-name_2"), "valid-name_2");
    }

    #[test]
    fn sanitize_hole_filename_strips_path_separators_and_dots() {
        // Ni `/` ni `.` ne survivent au nettoyage : aucune remontée `..`
        // ni chemin absolu ne peut jamais sortir de `HOLE_LIBRARY_DIR`.
        assert_eq!(sanitize_hole_filename("../../etc/passwd"), "______etc_passwd");
    }

    #[test]
    fn sanitize_hole_filename_falls_back_to_hole_when_input_is_blank() {
        assert_eq!(sanitize_hole_filename("   "), "hole");
        assert_eq!(sanitize_hole_filename(""), "hole");
    }

    #[test]
    fn suggested_declared_size_swaps_sides_with_orientation() {
        // Par 4 (long côté 55) tient sans être écrêté dans les deux sens
        // (largeur max 100, hauteur max 60) — un bon cas pour vérifier que
        // l'orientation échange proprement largeur/hauteur.
        let (hw, hh) = suggested_declared_size(4, BuilderOrientation::Horizontal);
        let (vw, vh) = suggested_declared_size(4, BuilderOrientation::Vertical);
        assert_eq!((hw, hh), (vh, vw));
    }

    #[test]
    fn suggested_declared_size_never_exceeds_the_full_canvas() {
        for par in 0..=9u8 {
            for orientation in [BuilderOrientation::Horizontal, BuilderOrientation::Vertical] {
                let (w, h) = suggested_declared_size(par, orientation);
                assert!(w <= COURSE_WIDTH && h <= COURSE_HEIGHT);
            }
        }
    }

    #[test]
    fn suggested_declared_size_reaches_the_full_canvas_at_par_7_horizontal() {
        // Le petit côté est dérivé proportionnellement du grand côté (voir
        // `suggested_declared_size`) : à par 7+ en horizontal, le grand
        // côté sature à 100 (COURSE_WIDTH), donc le petit côté doit saturer
        // à 60 (COURSE_HEIGHT) — pas rester bloqué à une constante fixe
        // indépendante du par (bug corrigé après un signalement : le petit
        // côté restait toujours 30, quel que soit le par choisi).
        assert_eq!(
            suggested_declared_size(7, BuilderOrientation::Horizontal),
            (COURSE_WIDTH, COURSE_HEIGHT)
        );
    }

    #[test]
    fn suggested_declared_size_caps_the_long_side_before_scaling_when_vertical() {
        // Le canevas n'est pas carré (100x60) : en vertical, le grand côté
        // ne peut jamais dépasser 60 (COURSE_HEIGHT), même à par 7+ où le
        // grand côté "brut" viserait 100. Il doit être clampé à 60 *avant*
        // d'en dériver le petit côté (36 = 60*3/5), pas après (ce qui
        // donnerait à tort un carré 60x60).
        assert_eq!(
            suggested_declared_size(7, BuilderOrientation::Vertical),
            (36, COURSE_HEIGHT)
        );
    }

    #[test]
    fn suggested_declared_size_shrinks_both_sides_together_for_shorter_pars() {
        // Les deux côtés doivent grandir ensemble avec le par, pas
        // seulement le grand côté.
        let (w3, h3) = suggested_declared_size(3, BuilderOrientation::Horizontal);
        let (w5, h5) = suggested_declared_size(5, BuilderOrientation::Horizontal);
        assert!(w3 < w5 && h3 < h5);
    }

    #[test]
    fn typing_terrain_paints_and_advances_horizontally_then_wraps() {
        let mut builder = BuilderState::new(4, BuilderOrientation::Horizontal, Lang::En);
        builder.width = 3;
        builder.height = 2;
        builder.tiles = vec![vec![TerrainKind::OutOfBounds; 3]; 2];
        builder.cursor = Pos { x: 2, y: 0 };

        builder.type_terrain(TerrainKind::Fairway);
        assert_eq!(builder.tiles[0][2], TerrainKind::Fairway);
        // Fin de ligne : passe au début de la suivante plutôt que de boucler
        // sur la même ligne.
        assert_eq!(builder.cursor, Pos { x: 0, y: 1 });
    }

    #[test]
    fn typing_terrain_stops_at_the_last_cell_instead_of_wrapping_around() {
        let mut builder = BuilderState::new(4, BuilderOrientation::Horizontal, Lang::En);
        builder.width = 2;
        builder.height = 1;
        builder.tiles = vec![vec![TerrainKind::OutOfBounds; 2]; 1];
        builder.cursor = Pos { x: 1, y: 0 };

        builder.type_terrain(TerrainKind::Bunker);
        assert_eq!(builder.cursor, Pos { x: 1, y: 0 }, "dernière case : le curseur ne doit pas boucler");
    }

    #[test]
    fn typing_terrain_advances_vertically_then_wraps_to_the_next_column() {
        let mut builder = BuilderState::new(4, BuilderOrientation::Vertical, Lang::En);
        builder.width = 2;
        builder.height = 2;
        builder.tiles = vec![vec![TerrainKind::OutOfBounds; 2]; 2];
        builder.cursor = Pos { x: 0, y: 1 };

        builder.type_terrain(TerrainKind::Water);
        assert_eq!(builder.tiles[1][0], TerrainKind::Water);
        assert_eq!(builder.cursor, Pos { x: 1, y: 0 });
    }

    #[test]
    fn undo_restores_the_previous_terrain_and_moves_the_cursor_back() {
        let mut builder = BuilderState::new(4, BuilderOrientation::Horizontal, Lang::En);
        builder.width = 3;
        builder.height = 1;
        builder.tiles = vec![vec![TerrainKind::Rough; 3]];
        builder.cursor = Pos { x: 0, y: 0 };

        builder.type_terrain(TerrainKind::Fairway);
        assert_eq!(builder.tiles[0][0], TerrainKind::Fairway);
        assert_eq!(builder.cursor, Pos { x: 1, y: 0 });

        builder.undo();
        assert_eq!(builder.tiles[0][0], TerrainKind::Rough, "l'ancien terrain doit être restauré");
        assert_eq!(builder.cursor, Pos { x: 0, y: 0 }, "le curseur doit revenir sur la case annulée");
    }

    #[test]
    fn move_cursor_is_clamped_to_the_grid_bounds() {
        let mut builder = BuilderState::new(4, BuilderOrientation::Horizontal, Lang::En);
        builder.width = 3;
        builder.height = 3;
        builder.tiles = vec![vec![TerrainKind::OutOfBounds; 3]; 3];
        builder.cursor = Pos { x: 0, y: 0 };

        builder.move_cursor(-1, -1);
        assert_eq!(builder.cursor, Pos { x: 0, y: 0 }, "ne doit pas sortir de la grille en haut/à gauche");

        builder.cursor = Pos { x: 2, y: 2 };
        builder.move_cursor(1, 1);
        assert_eq!(builder.cursor, Pos { x: 2, y: 2 }, "ne doit pas sortir de la grille en bas/à droite");
    }

    #[test]
    fn to_course_raw_produces_a_file_that_hole_parse_accepts() {
        let mut builder = BuilderState::new(3, BuilderOrientation::Horizontal, Lang::En);
        builder.width = 5;
        builder.height = 3;
        builder.tiles = vec![vec![TerrainKind::Fairway; 5]; 3];
        builder.tiles[1][0] = TerrainKind::Tee;
        builder.tiles[1][4] = TerrainKind::Hole;

        let raw = builder.to_course_raw();
        let hole = Hole::parse(&raw).expect("un trou valide doit parser");
        assert_eq!(hole.meta.par, 3);
        assert_eq!(hole.tee, Pos { x: (COURSE_WIDTH - 5) / 2, y: (COURSE_HEIGHT - 3) / 2 + 1 });
    }

    #[test]
    fn discover_hole_files_finds_course_files_one_level_deep() {
        let root = std::env::temp_dir().join(format!("divotty_test_discover_{}", std::process::id()));
        std::fs::create_dir_all(root.join("demo")).unwrap();
        std::fs::create_dir_all(root.join("_library")).unwrap();
        std::fs::write(root.join("demo/hole_01.course"), "x").unwrap();
        std::fs::write(root.join("_library/mon_trou.course"), "x").unwrap();
        std::fs::write(root.join("demo/course.yaml"), "x").unwrap(); // pas un .course, ignoré

        let files = discover_hole_files(&root);
        assert_eq!(files.len(), 2);
        assert!(files.contains(&root.join("demo/hole_01.course")));
        assert!(files.contains(&root.join("_library/mon_trou.course")));

        std::fs::remove_dir_all(&root).unwrap();
    }

    #[test]
    fn discover_hole_files_returns_empty_for_a_missing_root() {
        let root = std::env::temp_dir().join("divotty_test_discover_never_exists_xyz");
        assert!(discover_hole_files(&root).is_empty());
    }

    #[test]
    fn resolve_data_root_prefers_cwd_when_courses_exists() {
        let platform_dir = Some(PathBuf::from("/home/x/.local/share/divotty"));
        assert_eq!(resolve_data_root_from(true, platform_dir), PathBuf::new());
    }

    #[test]
    fn resolve_data_root_falls_back_to_platform_dir_when_cwd_has_no_courses() {
        let platform_dir = PathBuf::from("/home/x/.local/share/divotty");
        assert_eq!(resolve_data_root_from(false, Some(platform_dir.clone())), platform_dir);
    }

    #[test]
    fn resolve_data_root_falls_back_to_cwd_if_platform_dir_unavailable() {
        assert_eq!(resolve_data_root_from(false, None), PathBuf::new());
    }

    /// Grille déclarée `width`x`height` avec un `D`/`H` aux coins opposés —
    /// équivalent local à `core::course::tests::build_small_raw`, pas
    /// réutilisable directement d'un module de test à l'autre.
    fn small_hole_raw(width: usize, height: usize) -> String {
        let mut lines = Vec::with_capacity(height);
        for y in 0..height {
            let mut row = vec!['.'; width];
            if y == 0 {
                row[0] = 'D';
            }
            if y == height - 1 {
                row[width - 1] = 'H';
            }
            lines.push(row.into_iter().collect::<String>());
        }
        format!(
            "name: \"Petit trou\"\npar: 3\nwidth: {width}\nheight: {height}\n---\n{}\n",
            lines.join("\n")
        )
    }

    #[test]
    fn from_existing_hole_preserves_meta_and_infers_orientation() {
        let raw = small_hole_raw(20, 10); // largeur > hauteur : horizontal
        let hole = Hole::parse(&raw).expect("le petit trou doit parser");

        let builder = BuilderState::from_existing_hole(&hole, None, Lang::En);
        assert_eq!(builder.name, hole.meta.name);
        assert_eq!(builder.par, hole.meta.par);
        assert_eq!(builder.width, 20);
        assert_eq!(builder.height, 10);
        assert_eq!(builder.orientation, BuilderOrientation::Horizontal);
        assert_eq!(builder.tiles, hole.local_tiles());
        assert_eq!(builder.source_path, None);
    }

    #[test]
    fn from_existing_hole_infers_vertical_orientation_and_keeps_source_path() {
        let raw = small_hole_raw(10, 20); // hauteur > largeur : vertical
        let hole = Hole::parse(&raw).expect("le petit trou doit parser");
        let path = PathBuf::from("courses/demo/hole_01.course");

        let builder = BuilderState::from_existing_hole(&hole, Some(path.clone()), Lang::Fr);
        assert_eq!(builder.orientation, BuilderOrientation::Vertical);
        assert_eq!(builder.source_path, Some(path));
    }
}
