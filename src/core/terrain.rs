use serde::{Deserialize, Serialize};

/// Type de terrain d'une case du parcours.
///
/// Chaque variante porte ses propres modificateurs via `TerrainKind::profile()`.
/// Pour ajouter un nouveau type de terrain : ajouter la variante ici, un
/// caractère dans `TerrainKind::from_char`, et un profil dans `profile()`.
/// Le moteur de résolution de coup (`shot.rs`) n'a rien à connaître de plus.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum TerrainKind {
    Tee,
    Fairway,
    Rough,
    Bunker,
    Water,
    Tree,
    Green,
    Hole,
    OutOfBounds,
}

/// Profil de jeu associé à un terrain : comment il affecte un coup joué
/// *depuis* cette case.
#[derive(Debug, Clone, Copy)]
pub struct TerrainProfile {
    /// Multiplicateur de distance (1.0 = neutre).
    pub distance_mult: f32,
    /// Multiplicateur de dispersion/imprécision (1.0 = neutre, plus haut = moins précis).
    pub dispersion_mult: f32,
    /// Coups de pénalité ajoutés si la balle *s'arrête* sur cette case.
    pub landing_penalty: u8,
    /// Si vrai, une balle qui atterrit ici est automatiquement droppée
    /// (relancée) à la position précédente ou au dernier point valide.
    pub forces_drop: bool,
    /// Si vrai, un tir traversant cette case peut être dévié/bloqué
    /// (utilisé pour les arbres).
    pub blocks_trajectory: bool,
}

impl TerrainKind {
    pub fn profile(self) -> TerrainProfile {
        match self {
            TerrainKind::Tee | TerrainKind::Fairway => TerrainProfile {
                distance_mult: 1.0,
                dispersion_mult: 1.0,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: false,
            },
            TerrainKind::Rough => TerrainProfile {
                distance_mult: 0.85,
                dispersion_mult: 1.35,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: false,
            },
            TerrainKind::Bunker => TerrainProfile {
                distance_mult: 0.6,
                dispersion_mult: 1.6,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: false,
            },
            TerrainKind::Green => TerrainProfile {
                distance_mult: 1.0,
                dispersion_mult: 0.5,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: false,
            },
            TerrainKind::Water => TerrainProfile {
                distance_mult: 1.0,
                dispersion_mult: 1.0,
                landing_penalty: 1,
                forces_drop: true,
                blocks_trajectory: false,
            },
            TerrainKind::Tree => TerrainProfile {
                distance_mult: 0.4,
                dispersion_mult: 2.0,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: true,
            },
            TerrainKind::Hole => TerrainProfile {
                distance_mult: 1.0,
                dispersion_mult: 1.0,
                landing_penalty: 0,
                forces_drop: false,
                blocks_trajectory: false,
            },
            TerrainKind::OutOfBounds => TerrainProfile {
                distance_mult: 1.0,
                dispersion_mult: 1.0,
                landing_penalty: 1,
                forces_drop: true,
                blocks_trajectory: false,
            },
        }
    }

    /// Caractère utilisé dans les fichiers `.course` pour représenter ce terrain.
    pub fn from_char(c: char) -> Option<Self> {
        Some(match c {
            'D' => TerrainKind::Tee,
            '.' => TerrainKind::Fairway,
            '=' => TerrainKind::Rough,
            'B' => TerrainKind::Bunker,
            '~' => TerrainKind::Water,
            'T' => TerrainKind::Tree,
            'G' => TerrainKind::Green,
            'H' => TerrainKind::Hole,
            ' ' => TerrainKind::OutOfBounds,
            _ => return None,
        })
    }
}
