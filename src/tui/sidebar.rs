use crate::core::{Club, Direction, HoleMeta, HoleScore, ScoreLabel, ShotResult, TerrainKind};
use crate::tui::format::stars;
use crate::tui::lang::Lang;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, Borders, Paragraph, Widget},
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
    pub strokes: u8,
    pub club: Club,
    pub aim: Direction,
    pub last_die: Option<u8>,
    pub last_shot: Option<&'a ShotResult>,
    /// Vrai juste après une sauvegarde réussie : affiche une confirmation à
    /// la place du dernier message de coup, jusqu'à la prochaine action.
    pub just_saved: bool,
    /// Une première pression sur q est en attente de confirmation (`qq`).
    pub quit_confirm: bool,
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
    die: &'static str,
    club_hint: &'static str,
    controls_body: &'static str,
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
            die: "Die",
            club_hint: "[Tab] next",
            controls_body: "← →  aim\nTab  club\nSpace  play\nS  save\nL  language\nqq  quit",
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
            die: "Dé",
            club_hint: "[Tab] suivant",
            controls_body: "← →  viser\nTab  club\nEspace  jouer\nS  sauvegarder\nL  langue\nqq  quitter",
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

fn score_label_text(label: ScoreLabel, lang: Lang) -> &'static str {
    match (label, lang) {
        (ScoreLabel::Albatross, Lang::En) => "Albatross",
        (ScoreLabel::Albatross, Lang::Fr) => "Albatros",
        (ScoreLabel::Eagle, _) => "Eagle",
        (ScoreLabel::Birdie, _) => "Birdie",
        (ScoreLabel::Par, _) => "Par",
        (ScoreLabel::Bogey, _) => "Bogey",
        (ScoreLabel::DoubleBogey, Lang::En) => "Double bogey",
        (ScoreLabel::DoubleBogey, Lang::Fr) => "Double bogey",
        (ScoreLabel::TripleBogeyOrWorse, Lang::En) => "Triple bogey or worse",
        (ScoreLabel::TripleBogeyOrWorse, Lang::Fr) => "Triple bogey ou plus",
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

fn format_relative(relative: i16) -> String {
    match relative {
        0 => "E".to_string(),
        r if r > 0 => format!("+{r}"),
        r => format!("{r}"),
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

fn panel(area: Rect, buf: &mut Buffer, title: &str, body: String) {
    let block = Block::default()
        .borders(Borders::ALL)
        .title(title.to_string())
        .style(Style::default().fg(Color::White));
    Paragraph::new(body).block(block).render(area, buf);
}

impl<'a> Widget for SidebarState<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let l = labels(self.lang);

        let chunks = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(3), // Titre
                Constraint::Length(5), // Infos du trou
                Constraint::Length(4), // Score
                Constraint::Length(4), // Club
                Constraint::Length(4), // Dernier coup
                Constraint::Length(3), // Visée
                Constraint::Min(0),    // Contrôles
            ])
            .split(area);

        panel(chunks[0], buf, "Divotty", "⛳ Divotty".to_string());

        let hole_line = match self.lang {
            Lang::En => format!("Hole {}/{}", self.hole_index + 1, self.hole_count),
            Lang::Fr => format!("Trou {}/{}", self.hole_index + 1, self.hole_count),
        };
        panel(
            chunks[1],
            buf,
            l.panel_hole,
            format!(
                "{}\n{}\nPar {}  {}",
                hole_line,
                self.hole_meta.name,
                self.hole_meta.par,
                stars(self.course_difficulty)
            ),
        );

        let score_line = if self.strokes == 0 {
            "—".to_string()
        } else {
            let hole_score = HoleScore {
                strokes: self.strokes,
                par: self.hole_meta.par,
            };
            format!(
                "{} ({})",
                score_label_text(hole_score.label(), self.lang),
                format_relative(hole_score.relative_to_par())
            )
        };
        panel(
            chunks[2],
            buf,
            l.panel_score,
            format!("{}: {}\n{}", l.strokes, self.strokes, score_line),
        );

        panel(
            chunks[3],
            buf,
            l.panel_club,
            format!("{}\n{}", club_label(self.club, self.lang), l.club_hint),
        );

        let die_text = self
            .last_die
            .map(|d| d.to_string())
            .unwrap_or_else(|| "—".to_string());
        let message = if self.just_saved {
            l.saved_message.to_string()
        } else {
            match self.last_shot {
                Some(shot) => shot_message(shot, self.strokes, self.lang),
                None => l.ready_message.to_string(),
            }
        };
        panel(
            chunks[4],
            buf,
            l.panel_last_shot,
            format!("{}: {}\n{}", l.die, die_text, message),
        );

        let angle_deg = self.aim.dy.atan2(self.aim.dx).to_degrees();
        panel(
            chunks[5],
            buf,
            l.panel_aim,
            format!("{} {:.0}°", compass_arrow(self.aim), (angle_deg + 360.0) % 360.0),
        );

        let controls_body = if self.quit_confirm {
            format!("{}\n{}", l.quit_confirm_hint, l.controls_body)
        } else {
            l.controls_body.to_string()
        };
        panel(chunks[6], buf, l.panel_controls, controls_body);
    }
}
