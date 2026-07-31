use crate::core::ScoreLabel;
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
}
