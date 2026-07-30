/// Langue d'affichage de l'interface. Anglais par défaut. Ajouter une
/// langue = ajouter une variante ici + les branches correspondantes dans
/// les fonctions de traduction de `sidebar.rs` — pas de fichiers de
/// ressources externes pour l'instant, le projet est encore trop jeune
/// pour justifier cette complexité.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Lang {
    #[default]
    En,
    Fr,
}

impl Lang {
    pub fn next(self) -> Self {
        match self {
            Lang::En => Lang::Fr,
            Lang::Fr => Lang::En,
        }
    }
}
