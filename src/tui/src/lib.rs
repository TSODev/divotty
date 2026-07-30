//! divotty-tui : rendu ratatui pour Divotty.
//!
//! - `render`  : vue de la carte (viewport suivant la balle)
//! - `sidebar` : colonne d'infos de jeu (titre, trou, score, club, visée...)
//! - `menu`    : écran de sélection de parcours
//! - `lang`    : langue d'affichage de l'interface (anglais par défaut)
//! - `format`  : petits helpers d'affichage partagés (étoiles de difficulté...)

pub mod format;
pub mod lang;
pub mod menu;
pub mod render;
pub mod sidebar;

pub use lang::Lang;
pub use menu::CourseMenuState;
pub use render::{CourseView, Viewport};
pub use sidebar::SidebarState;
