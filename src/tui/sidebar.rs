use crate::core::{Club, Direction, HoleMeta, HoleScore, Scorecard, ShotResult, TerrainKind, Wind};
use crate::tui::format::{die_cap_bar, format_relative, score_color, score_label_text, stars};
use crate::tui::lang::Lang;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Paragraph, Widget},
};

/// Panneau d'informations à gauche de l'écran : le plateau de jeu (carte)
/// occupe la colonne de droite, cette colonne empile les infos de la partie
/// en cours en plusieurs volets plutôt qu'une seule barre HUD.
pub struct SidebarState<'a> {
    pub lang: Lang,
    pub hole_meta: &'a HoleMeta,
    /// Difficulté du parcours (1 à 4), purement indicative — voir
    /// `core::Course::difficulty`.
    pub course_difficulty: u8,
    pub hole_index: usize,
    pub hole_count: usize,
    /// Scores des trous déjà quittés (pas le trou en cours) — sert à
    /// afficher un total cumulé une fois qu'il y a plus d'un trou.
    pub scorecard: &'a Scorecard,
    pub strokes: u8,
    pub club: Club,
    /// Plafond choisi par le joueur pour le tirage du dé (3 à 6, 6 = pas de
    /// plafond) — voir `GameState::die_strength` dans `main.rs`. Affiché
    /// sous un libellé golf ("Force du coup"/"Shot power") plutôt que
    /// "plafond" : la mécanique de plafonnement du dé reste un détail
    /// interne, pas quelque chose que le joueur a besoin de comprendre.
    pub die_strength: u8,
    pub aim: Direction,
    pub wind: Wind,
    pub last_die: Option<u8>,
    pub last_shot: Option<&'a ShotResult>,
    /// Vrai juste après une sauvegarde réussie : affiche une confirmation à
    /// la place du dernier message de coup, jusqu'à la prochaine action.
    pub just_saved: bool,
    /// Une première pression sur q est en attente de confirmation (`qq`).
    pub quit_confirm: bool,
    /// Vrai si le dernier coup a atteint le trou : plus d'action de jeu
    /// possible, juste rejouer (`R`) ou revenir au menu (`M`) — voir
    /// `GameState::finished` dans `main.rs`.
    pub finished: bool,
}

/// Couleur d'accent (bordure + titre) par panneau — identité visuelle fixe,
/// distincte de la couleur réactive éventuelle de son contenu (score,
/// dernier coup, vent).
const TITLE_ACCENT: Color = Color::White;
const HOLE_ACCENT: Color = Color::LightGreen;
const SCORE_ACCENT: Color = Color::Yellow;
const CLUB_ACCENT: Color = Color::LightBlue;
const LAST_SHOT_ACCENT: Color = Color::LightMagenta;
const AIM_ACCENT: Color = Color::Cyan;
const CONTROLS_ACCENT: Color = Color::DarkGray;

const NEUTRAL: Style = Style::new().fg(Color::White);
const DIM: Style = Style::new().fg(Color::DarkGray);

fn bold(color: Color) -> Style {
    Style::new().fg(color).add_modifier(Modifier::BOLD)
}

/// Couleur du vent selon sa force : vert calme, jaune modéré, rouge fort.
fn wind_color(strength: f32) -> Color {
    if strength < 1.0 {
        Color::LightGreen
    } else if strength < 2.0 {
        Color::Yellow
    } else {
        Color::Red
    }
}

/// Libellés fixes de l'interface (titres de volets, aides), par langue.
struct Labels {
    panel_hole: &'static str,
    panel_score: &'static str,
    panel_club: &'static str,
    panel_last_shot: &'static str,
    panel_aim: &'static str,
    panel_controls: &'static str,
    strokes: &'static str,
    total: &'static str,
    die: &'static str,
    die_cap: &'static str,
    wind: &'static str,
    club_hint: &'static str,
    controls_body: &'static str,
    finished_controls_body: &'static str,
    finished_controls_body_more_holes: &'static str,
    ready_message: &'static str,
    saved_message: &'static str,
    quit_confirm_hint: &'static str,
}

fn labels(lang: Lang) -> Labels {
    match lang {
        Lang::En => Labels {
            panel_hole: "Hole",
            panel_score: "Score",
            panel_club: "Club",
            panel_last_shot: "Last shot",
            panel_aim: "Aim",
            panel_controls: "Controls",
            strokes: "Strokes",
            total: "Total",
            die: "Die",
            die_cap: "Shot power",
            wind: "Wind",
            club_hint: "Tab club  +/- power",
            controls_body: "← →  aim\nTab  club\nSpace  play\nZ  zoom\nS  save\nL  language\nqq  quit",
            finished_controls_body: "Enter  finish round\nR  replay\nM  menu\nZ  zoom\nL  language\nqq  quit",
            finished_controls_body_more_holes: "N  next hole\nR  replay\nM  menu\nZ  zoom\nL  language\nqq  quit",
            ready_message: "Ready to play.",
            saved_message: "Game saved.",
            quit_confirm_hint: "Press q again to quit",
        },
        Lang::Fr => Labels {
            panel_hole: "Trou",
            panel_score: "Score",
            panel_club: "Club",
            panel_last_shot: "Dernier coup",
            panel_aim: "Visée",
            panel_controls: "Contrôles",
            strokes: "Coups",
            total: "Total",
            die: "Dé",
            die_cap: "Force du coup",
            wind: "Vent",
            club_hint: "Tab club  +/- force",
            controls_body: "← →  viser\nTab  club\nEspace  jouer\nZ  zoom\nS  sauvegarder\nL  langue\nqq  quitter",
            finished_controls_body: "Entrée  terminer\nR  rejouer\nM  menu\nZ  zoom\nL  langue\nqq  quitter",
            finished_controls_body_more_holes: "N  trou suivant\nR  rejouer\nM  menu\nZ  zoom\nL  langue\nqq  quitter",
            ready_message: "Prêt à jouer.",
            saved_message: "Partie sauvegardée.",
            quit_confirm_hint: "Appuyez encore sur q pour quitter",
        },
    }
}

fn club_label(club: Club, lang: Lang) -> &'static str {
    match (club, lang) {
        (Club::Driver, Lang::En) => "Driver",
        (Club::Driver, Lang::Fr) => "Driver",
        (Club::Wood, Lang::En) => "Wood",
        (Club::Wood, Lang::Fr) => "Bois",
        (Club::Hybrid, Lang::En) => "Hybrid",
        (Club::Hybrid, Lang::Fr) => "Hybride",
        (Club::Iron, Lang::En) => "Iron",
        (Club::Iron, Lang::Fr) => "Fer",
        (Club::Wedge, Lang::En) => "Wedge",
        (Club::Wedge, Lang::Fr) => "Wedge",
        (Club::Putter, Lang::En) => "Putter",
        (Club::Putter, Lang::Fr) => "Putter",
    }
}

/// Nom du terrain tel qu'inséré dans un message ("Ball on {}" / "Balle sur
/// {}") — l'article est inclus dans la traduction plutôt que géré à part,
/// pour rester correct dans les deux langues sans logique grammaticale.
fn terrain_name(terrain: TerrainKind, lang: Lang) -> &'static str {
    match (terrain, lang) {
        (TerrainKind::Tee, Lang::En) => "the tee",
        (TerrainKind::Tee, Lang::Fr) => "le départ",
        (TerrainKind::Fairway, Lang::En) => "the fairway",
        (TerrainKind::Fairway, Lang::Fr) => "le fairway",
        (TerrainKind::Rough, Lang::En) => "the rough",
        (TerrainKind::Rough, Lang::Fr) => "le rough",
        (TerrainKind::Bunker, Lang::En) => "the bunker",
        (TerrainKind::Bunker, Lang::Fr) => "le bunker",
        (TerrainKind::Water, Lang::En) => "the water",
        (TerrainKind::Water, Lang::Fr) => "l'eau",
        (TerrainKind::Tree, Lang::En) => "a tree",
        (TerrainKind::Tree, Lang::Fr) => "un arbre",
        (TerrainKind::Green, Lang::En) => "the green",
        (TerrainKind::Green, Lang::Fr) => "le green",
        (TerrainKind::Hole, Lang::En) => "the hole",
        (TerrainKind::Hole, Lang::Fr) => "le trou",
        (TerrainKind::OutOfBounds, Lang::En) => "out of bounds",
        (TerrainKind::OutOfBounds, Lang::Fr) => "hors-limites",
        (TerrainKind::PenaltyZone, Lang::En) => "a penalty area",
        (TerrainKind::PenaltyZone, Lang::Fr) => "une zone à pénalité",
    }
}

fn shot_message(shot: &ShotResult, strokes: u8, lang: Lang) -> String {
    if shot.holed {
        match lang {
            Lang::En => format!("Holed in {strokes} strokes!"),
            Lang::Fr => format!("Dans le trou en {strokes} coups !"),
        }
    } else if shot.dropped {
        match lang {
            Lang::En => "Penalty, ball dropped".to_string(),
            Lang::Fr => "Pénalité, balle droppée".to_string(),
        }
    } else {
        match lang {
            Lang::En => format!("Ball on {}", terrain_name(shot.landing_terrain, lang)),
            Lang::Fr => format!("Balle sur {}", terrain_name(shot.landing_terrain, lang)),
        }
    }
}

/// Flèche de boussole approximant la direction visée (8 secteurs de 45°).
fn compass_arrow(direction: Direction) -> &'static str {
    let angle_deg = direction.dy.atan2(direction.dx).to_degrees();
    let normalized = (angle_deg + 360.0) % 360.0;
    let sector = ((normalized + 22.5) / 45.0) as i32 % 8;
    match sector {
        0 => "→",
        1 => "↘",
        2 => "↓",
        3 => "↙",
        4 => "←",
        5 => "↖",
        6 => "↑",
        _ => "↗",
    }
}

/// Panneau à bordures arrondies avec une couleur d'accent (bordure + titre)
/// propre à chaque panneau, et un contenu ligne par ligne dont chaque ligne
/// porte son propre style — pour que seule l'information pertinente (score,
/// dernier coup, vent) réagisse en couleur, pas tout le panneau.
fn panel(area: Rect, buf: &mut Buffer, title: &str, accent: Color, lines: Vec<Line<'static>>) {
    let block = Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .title(title.to_string())
        .style(Style::default().fg(accent));
    Paragraph::new(lines).block(block).render(area, buf);
}

/// Comme `panel`, mais le contenu est plaqué en bas du panneau plutôt qu'en
/// haut (`Paragraph` ne supporte pas l'alignement vertical nativement — on
/// préfixe simplement des lignes vides calculées à partir de la hauteur
/// disponible).
fn panel_bottom_aligned(area: Rect, buf: &mut Buffer, title: &str, accent: Color, lines: Vec<Line<'static>>) {
    let block = Block::default()
        .border_type(BorderType::Rounded)
        .borders(Borders::ALL)
        .title(title.to_string())
        .style(Style::default().fg(accent));
    let inner_height = block.inner(area).height as usize;
    let mut padded = vec![Line::from(""); inner_height.saturating_sub(lines.len())];
    padded.extend(lines);
    Paragraph::new(padded).block(block).render(area, buf);
}

impl<'a> Widget for SidebarState<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let l = labels(self.lang);
        // Une ligne "Total" en plus dans le panneau Score dès qu'il y a
        // plus d'un trou (sinon le total serait toujours égal au trou
        // courant, sans intérêt).
        let score_panel_height = if self.hole_count > 1 { 5 } else { 4 };

        let chunks = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(3), // Titre
                Constraint::Length(5), // Infos du trou
                Constraint::Length(score_panel_height), // Score
                Constraint::Length(5), // Club
                Constraint::Length(4), // Dernier coup
                Constraint::Length(4), // Visée (aim + vent)
                Constraint::Min(0),    // Contrôles
            ])
            .split(area);

        panel(
            chunks[0],
            buf,
            "Divotty",
            TITLE_ACCENT,
            vec![Line::styled(
                format!("⛳ Divotty v{}", env!("CARGO_PKG_VERSION")),
                bold(TITLE_ACCENT),
            )],
        );

        let hole_line = match self.lang {
            Lang::En => format!("Hole {}/{}", self.hole_index + 1, self.hole_count),
            Lang::Fr => format!("Trou {}/{}", self.hole_index + 1, self.hole_count),
        };
        panel(
            chunks[1],
            buf,
            l.panel_hole,
            HOLE_ACCENT,
            vec![
                Line::styled(hole_line, NEUTRAL),
                Line::styled(self.hole_meta.name.clone(), NEUTRAL),
                Line::styled(
                    format!("Par {}  {}", self.hole_meta.par, stars(self.course_difficulty)),
                    NEUTRAL,
                ),
            ],
        );

        let score_line = if self.strokes == 0 {
            Line::styled("—", NEUTRAL)
        } else {
            let hole_score = HoleScore {
                strokes: self.strokes,
                par: self.hole_meta.par,
            };
            let label = hole_score.label();
            Line::styled(
                format!(
                    "{} ({})",
                    score_label_text(label, self.lang),
                    format_relative(hole_score.relative_to_par() as i32)
                ),
                bold(score_color(label)),
            )
        };
        let mut score_lines = vec![Line::styled(format!("{}: {}", l.strokes, self.strokes), NEUTRAL), score_line];
        if self.hole_count > 1 {
            score_lines.push(Line::styled(
                format!(
                    "{}: {} ({})",
                    l.total,
                    self.scorecard.total_strokes(),
                    format_relative(self.scorecard.relative_to_par())
                ),
                DIM,
            ));
        }
        panel(chunks[2], buf, l.panel_score, SCORE_ACCENT, score_lines);

        let die_cap_style = if self.die_strength < 6 {
            bold(Color::Yellow)
        } else {
            DIM
        };
        panel(
            chunks[3],
            buf,
            l.panel_club,
            CLUB_ACCENT,
            vec![
                Line::styled(club_label(self.club, self.lang), bold(Color::White)),
                Line::styled(format!("{}: {}", l.die_cap, die_cap_bar(self.die_strength)), die_cap_style),
                Line::styled(l.club_hint, DIM),
            ],
        );

        let die_text = self
            .last_die
            .map(|d| d.to_string())
            .unwrap_or_else(|| "—".to_string());
        let message_line = if self.just_saved {
            Line::styled(l.saved_message, bold(Color::Cyan))
        } else {
            match self.last_shot {
                Some(shot) if shot.holed => {
                    Line::styled(shot_message(shot, self.strokes, self.lang), bold(Color::LightGreen))
                }
                Some(shot) if shot.penalty_strokes > 0 => {
                    Line::styled(shot_message(shot, self.strokes, self.lang), bold(Color::Red))
                }
                Some(shot) => Line::styled(shot_message(shot, self.strokes, self.lang), NEUTRAL),
                None => Line::styled(l.ready_message, NEUTRAL),
            }
        };
        panel(
            chunks[4],
            buf,
            l.panel_last_shot,
            LAST_SHOT_ACCENT,
            vec![Line::styled(format!("{}: {}", l.die, die_text), NEUTRAL), message_line],
        );

        let angle_deg = self.aim.dy.atan2(self.aim.dx).to_degrees();
        panel(
            chunks[5],
            buf,
            l.panel_aim,
            AIM_ACCENT,
            vec![
                Line::styled(
                    format!("{} {:.0}°", compass_arrow(self.aim), (angle_deg + 360.0) % 360.0),
                    NEUTRAL,
                ),
                Line::styled(
                    format!("{}: {} {:.1}", l.wind, compass_arrow(self.wind.direction), self.wind.strength),
                    bold(wind_color(self.wind.strength)),
                ),
            ],
        );

        let base_controls: &str = if self.finished {
            if self.hole_index + 1 < self.hole_count {
                l.finished_controls_body_more_holes
            } else {
                l.finished_controls_body
            }
        } else {
            l.controls_body
        };
        let mut controls_lines: Vec<Line<'static>> = Vec::new();
        if self.quit_confirm {
            controls_lines.push(Line::styled(l.quit_confirm_hint, bold(Color::Red)));
        }
        controls_lines.extend(base_controls.lines().map(|line| Line::styled(line.to_string(), DIM)));
        panel_bottom_aligned(chunks[6], buf, l.panel_controls, CONTROLS_ACCENT, controls_lines);
    }
}
