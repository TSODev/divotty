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
