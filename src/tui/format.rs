use crate::core::{Direction, ScoreLabel};
use crate::tui::lang::Lang;
use ratatui::style::Color;

/// Affichage en étoiles d'une difficulté 1-4 (ex: "★★☆☆"). Universel, pas
/// besoin de traduction. Partagé entre le sidebar (en jeu) et le menu de
/// sélection de parcours.
pub fn stars(difficulty: u8) -> String {
    let filled = difficulty.min(4) as usize;
    "★".repeat(filled) + &"☆".repeat(4 - filled)
}

/// Couleur d'un label de score : doré pour un très bon score, vert pour un
/// birdie, neutre au par, orange/rouge à mesure qu'on s'en éloigne — lisible
/// d'un coup d'œil sans avoir à lire le texte. Partagé entre le panneau
/// Score (trou courant) et l'écran de scorecard complet (fin de partie).
pub fn score_color(label: ScoreLabel) -> Color {
    match label {
        ScoreLabel::Albatross | ScoreLabel::Eagle => Color::Rgb(255, 200, 0),
        ScoreLabel::Birdie => Color::LightGreen,
        ScoreLabel::Par => Color::White,
        ScoreLabel::Bogey => Color::Rgb(255, 140, 0),
        ScoreLabel::DoubleBogey | ScoreLabel::TripleBogeyOrWorse => Color::Red,
    }
}

pub fn score_label_text(label: ScoreLabel, lang: Lang) -> &'static str {
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

/// Écart au par affiché ("E" pour égalité, sinon signé : "+2", "-1"...).
pub fn format_relative(relative: i32) -> String {
    match relative {
        0 => "E".to_string(),
        r if r > 0 => format!("+{r}"),
        r => format!("{r}"),
    }
}

/// Barre "curseur" pour le plafond de dé (ex: "-+--"), un `+` marquant la
/// position courante parmi les 4 valeurs possibles (3 à 6, voir
/// `GameState::die_strength`/`DIE_STRENGTH_FLOOR` dans `main.rs`) — plus
/// lisible d'un coup d'œil qu'un simple texte "N/6". Même esprit que
/// `stars()` : la plage (3-6) est un fait du jeu, pas une valeur qu'il faut
/// faire voyager depuis `main.rs` pour ce seul besoin d'affichage.
pub fn die_cap_bar(value: u8) -> String {
    const MIN: u8 = 3;
    const MAX: u8 = 6;
    let clamped = value.clamp(MIN, MAX);
    let marker_index = (clamped - MIN) as usize;
    (MIN..=MAX)
        .enumerate()
        .map(|(i, _)| if i == marker_index { '+' } else { '-' })
        .collect()
}

/// Secteur de boussole (8 directions, 0=Est puis sens horaire) le plus
/// proche d'une direction donnée — calcul partagé entre `compass_arrow`
/// (glyphe affiché) et le placement de la flèche de visée dans la carte
/// zoomée (`render.rs`), pour que les deux restent toujours cohérents.
pub fn compass_sector(direction: Direction) -> usize {
    let angle_deg = direction.dy.atan2(direction.dx).to_degrees();
    let normalized = (angle_deg + 360.0) % 360.0;
    ((normalized + 22.5) / 45.0) as usize % 8
}

/// Flèche de boussole approximant une direction (8 secteurs de 45°).
/// Partagée entre le panneau Visée (`sidebar.rs`) et la flèche de visée
/// sur la carte zoomée (`render.rs`).
pub fn compass_arrow(direction: Direction) -> &'static str {
    match compass_sector(direction) {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn die_cap_bar_marks_the_right_slot() {
        assert_eq!(die_cap_bar(3), "+---");
        assert_eq!(die_cap_bar(4), "-+--");
        assert_eq!(die_cap_bar(5), "--+-");
        assert_eq!(die_cap_bar(6), "---+");
    }

    #[test]
    fn compass_arrow_matches_the_expected_sector() {
        let east = Direction { dx: 1.0, dy: 0.0 };
        let south = Direction { dx: 0.0, dy: 1.0 };
        let west = Direction { dx: -1.0, dy: 0.0 };
        let north = Direction { dx: 0.0, dy: -1.0 };

        assert_eq!(compass_arrow(east), "→");
        assert_eq!(compass_arrow(south), "↓");
        assert_eq!(compass_arrow(west), "←");
        assert_eq!(compass_arrow(north), "↑");
        assert_eq!(compass_sector(east), 0);
        assert_eq!(compass_sector(south), 2);
        assert_eq!(compass_sector(west), 4);
        assert_eq!(compass_sector(north), 6);
    }
}
