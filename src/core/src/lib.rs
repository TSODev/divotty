//! divotty-core : logique pure du jeu (aucune dépendance UI).
//!
//! - `terrain` : types de cases et leurs profils de jeu
//! - `course`  : structure de grille, parsing des fichiers `.course`, validation
//! - `shot`    : résolution d'un coup (dé + trajectoire + modificateurs)
//! - `scoring` : suivi du score

pub mod course;
pub mod scoring;
pub mod shot;
pub mod terrain;

pub use course::{Course, Hole, HoleMeta, Pos, COURSE_HEIGHT, COURSE_WIDTH};
pub use scoring::{HoleScore, Scorecard, ScoreLabel};
pub use shot::{preview_shot, resolve_shot, Club, Direction, Shot, ShotPreview, ShotResult};
pub use terrain::{TerrainKind, TerrainProfile};
