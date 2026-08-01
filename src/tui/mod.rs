//! `tui` : rendu ratatui pour Divotty.
//!
//! - `render`  : vue de la carte (viewport suivant la balle)
//! - `sidebar` : colonne d'infos de jeu (titre, trou, score, club, visée...)
//! - `menu`    : écran de sélection de parcours
//! - `scorecard` : écran de fin de partie (détail trou par trou + total)
//! - `builder` : écran d'édition d'un trou (voir ROADMAP.md, builder de trous)
//! - `lang`    : langue d'affichage de l'interface (anglais par défaut)
//! - `format`  : petits helpers d'affichage partagés (étoiles de difficulté...)

pub mod builder;
pub mod course_builder;
pub mod format;
pub mod lang;
pub mod menu;
pub mod render;
pub mod scorecard;
pub mod sidebar;

pub use builder::{
    BuilderMode, BuilderOrientation, BuilderSetupView, BuilderSidebarView, BuilderView,
    HolePickerView, HolePreviewView,
};
pub use course_builder::{
    CourseBuilderMode, CourseBuilderSidebarView, CoursePickerView, CourseSetupView,
    HoleAddPickerView,
};
pub use lang::Lang;
pub use menu::CourseMenuState;
pub use render::{CourseView, Viewport, ZoomLevel};
pub use scorecard::ScorecardView;
pub use sidebar::SidebarState;
