mod core;
mod tui;

use anyhow::Result;
use crossterm::{
    event::{self, Event, KeyCode},
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
    execute,
};
use crate::core::{
    preview_shot, resolve_shot, Club, Course, Direction, Hole, Pos, Shot, ShotResult, Wind,
};
use crate::tui::{CourseMenuState, CourseView, Lang, SidebarState, Viewport};
use rand::Rng;
use ratatui::{backend::CrosstermBackend, layout::{Constraint, Direction as LayoutDirection, Layout}, Terminal};
use serde::{Deserialize, Serialize};
use std::io::stdout;
use std::path::{Path, PathBuf};
use std::time::Duration;

const SAVE_PATH: &str = "save.yaml";

/// Liste les parcours jouables sous `courses/`, avec leur dossier d'origine
/// (nécessaire pour recharger/sauvegarder). Si `courses/` n'existe pas ou ne
/// contient rien, renvoie un unique parcours de démonstration codé en dur —
/// sans dossier associé, donc non sauvegardable.
fn discover_courses() -> Result<Vec<(Option<PathBuf>, Course)>> {
    let root = Path::new("courses");
    if root.exists() {
        let found = Course::discover(root)?;
        if !found.is_empty() {
            return Ok(found.into_iter().map(|(dir, course)| (Some(dir), course)).collect());
        }
    }
    Ok(vec![(None, fallback_course()?)])
}

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
}

fn save_game(state: &GameState) -> Result<()> {
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
    };
    std::fs::write(SAVE_PATH, serde_yaml::to_string(&data)?)?;
    Ok(())
}

fn load_game(lang: Lang) -> Result<GameState> {
    let raw = std::fs::read_to_string(SAVE_PATH)?;
    let data: SaveData = serde_yaml::from_str(&raw)?;
    let course = Course::load_from_dir(&data.course_dir)?;
    let hole_count = course.holes.len();
    let hole = course
        .holes
        .into_iter()
        .nth(data.hole_index)
        .ok_or_else(|| anyhow::anyhow!("numéro de trou invalide dans la sauvegarde"))?;

    Ok(GameState {
        hole,
        hole_index: data.hole_index,
        hole_count,
        course_difficulty: course.difficulty,
        course_dir: Some(data.course_dir),
        ball: data.ball,
        wind: data.wind,
        strokes: data.strokes,
        club: data.club,
        aim: data.aim,
        lang,
        last_die: None,
        last_shot: None,
        just_saved: false,
        quit_confirm: false,
        zoom: false,
    })
}

struct GameState {
    hole: Hole,
    hole_index: usize,
    hole_count: usize,
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
        let hole = course
            .holes
            .into_iter()
            .next()
            .expect("un parcours doit avoir au moins un trou");
        let ball = hole.tee;
        let aim = Direction::towards(hole.tee, hole.hole_pos);
        GameState {
            hole,
            hole_index: 0,
            hole_count,
            course_difficulty: course.difficulty,
            course_dir,
            ball,
            wind: random_wind(),
            strokes: 0,
            club: Club::Driver,
            aim,
            lang,
            last_die: None,
            last_shot: None,
            just_saved: false,
            quit_confirm: false,
            zoom: false,
        }
    }

    fn play_shot(&mut self) {
        let die: u8 = rand::thread_rng().gen_range(1..=6);
        let shot = Shot {
            club: self.club,
            direction: self.aim,
            die_roll: die,
        };
        let mut rng = rand::thread_rng();
        let result = resolve_shot(&self.hole, self.ball, shot, self.wind, &mut rng);
        self.strokes += 1 + result.penalty_strokes;
        self.ball = result.landing;
        self.aim = Direction::towards(self.ball, self.hole.hole_pos);
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
    fn restart_hole(&mut self) {
        self.ball = self.hole.tee;
        self.aim = Direction::towards(self.hole.tee, self.hole.hole_pos);
        self.wind = random_wind();
        self.strokes = 0;
        self.club = Club::Driver;
        self.last_die = None;
        self.last_shot = None;
        self.just_saved = false;
    }
}

fn main() -> Result<()> {
    let courses = discover_courses()?;

    enable_raw_mode()?;
    let mut stdout_handle = stdout();
    execute!(stdout_handle, EnterAlternateScreen)?;
    let backend = CrosstermBackend::new(stdout_handle);
    let mut terminal = Terminal::new(backend)?;

    let mut lang = Lang::default();
    let result = (|| -> Result<()> {
        loop {
            let has_save = Path::new(SAVE_PATH).exists();
            match select_course(&mut terminal, &courses, &mut lang, has_save)? {
                MenuChoice::Quit => return Ok(()),
                MenuChoice::Resume => {
                    let mut state = load_game(lang)?;
                    match run_loop(&mut terminal, &mut state)? {
                        LoopExit::Quit => return Ok(()),
                        LoopExit::BackToMenu => continue,
                    }
                }
                MenuChoice::Play(index) => {
                    // Cloné plutôt que retiré de la liste : le joueur peut
                    // revenir au menu (touche `M` en fin de trou) et
                    // choisir/rejouer le même parcours sans relancer le jeu.
                    let (course_dir, course) = courses[index].clone();
                    let mut state = GameState::new(course_dir, course, lang);
                    match run_loop(&mut terminal, &mut state)? {
                        LoopExit::Quit => return Ok(()),
                        LoopExit::BackToMenu => continue,
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
    Quit,
}

/// Comment on quitte la boucle de jeu : arrêt complet du programme, ou
/// retour à l'écran de sélection de parcours (fin de trou, touche `M`).
enum LoopExit {
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
                    hole_meta: &state.hole.meta,
                    course_difficulty: state.course_difficulty,
                    hole_index: state.hole_index,
                    hole_count: state.hole_count,
                    strokes: state.strokes,
                    club: state.club,
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

            let preview = preview_shot(&state.hole, state.ball, state.club, state.aim, state.wind);
            frame.render_widget(
                CourseView {
                    hole: &state.hole,
                    ball: state.ball,
                    viewport: Viewport {
                        // -2 : la carte est maintenant encadrée (bordure).
                        width: columns[1].width.saturating_sub(2) as usize,
                        height: columns[1].height.saturating_sub(2) as usize,
                    },
                    preview: Some(preview),
                    zoomed: state.zoom,
                },
                columns[1],
            );
        })?;

        if event::poll(Duration::from_millis(200))? {
            if let Event::Key(key) = event::read()? {
                if finished {
                    // Trou terminé : plus de visée/coup/sauvegarde, juste
                    // rejouer, revenir au menu, ou quitter (double q).
                    match key.code {
                        KeyCode::Char('q') | KeyCode::Char('Q') => {
                            if state.quit_confirm {
                                return Ok(LoopExit::Quit);
                            }
                            state.quit_confirm = true;
                            continue;
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
                        KeyCode::Char('s') | KeyCode::Char('S') => {
                            state.just_saved = save_game(state).is_ok();
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

#[cfg(test)]
mod tests {
    use super::*;

    /// État de jeu minimal pour les tests, basé sur le parcours de secours
    /// généré en mémoire (pas besoin de fichier `.course` sur disque).
    fn test_state() -> GameState {
        GameState::new(None, fallback_course().unwrap(), Lang::default())
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
        let tee = state.hole.tee;

        state.restart_hole();

        assert_eq!(state.ball, tee);
        assert_eq!(state.strokes, 0);
        assert_eq!(state.club, Club::Driver);
        assert!(state.last_die.is_none());
        assert!(state.last_shot.is_none());
        assert!(!state.just_saved);
        assert!(!state.finished());
    }
}
