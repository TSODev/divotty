//! `core` : logique pure du jeu (aucune dépendance à `ratatui`/`crossterm`,
//! voir CLAUDE.md — cette frontière est une convention de module, plus une
//! séparation de crate, depuis la fusion en un seul package).
//!
//! - `terrain` : types de cases et leurs profils de jeu
//! - `course`  : structure de grille, parsing des fichiers `.course`, validation
//! - `shot`    : résolution d'un coup (dé + trajectoire + modificateurs)
//! - `scoring` : suivi du score

pub mod course;
pub mod scoring;
pub mod shot;
pub mod terrain;

pub use course::{Course, CourseIndex, Hole, HoleMeta, Pos, COURSE_HEIGHT, COURSE_WIDTH};
pub use scoring::{HoleScore, ScoreLabel, Scorecard};
pub use shot::{preview_shot, resolve_shot, Club, Direction, Shot, ShotPreview, ShotResult, Wind};
pub use terrain::TerrainKind;
