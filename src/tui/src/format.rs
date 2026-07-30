/// Affichage en étoiles d'une difficulté 1-4 (ex: "★★☆☆"). Universel, pas
/// besoin de traduction. Partagé entre le sidebar (en jeu) et le menu de
/// sélection de parcours.
pub fn stars(difficulty: u8) -> String {
    let filled = difficulty.min(4) as usize;
    "★".repeat(filled) + &"☆".repeat(4 - filled)
}
