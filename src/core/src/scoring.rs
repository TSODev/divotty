use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HoleScore {
    pub strokes: u8,
    pub par: u8,
}

/// Nom courant d'un score par rapport au par (Birdie, Bogey, etc.).
/// Volontairement sans texte affichable : `core` reste indépendant de la
/// langue d'affichage, c'est à la couche UI de traduire chaque variante.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ScoreLabel {
    Albatross,
    Eagle,
    Birdie,
    Par,
    Bogey,
    DoubleBogey,
    TripleBogeyOrWorse,
}

impl HoleScore {
    pub fn relative_to_par(&self) -> i16 {
        self.strokes as i16 - self.par as i16
    }

    pub fn label(&self) -> ScoreLabel {
        match self.relative_to_par() {
            i16::MIN..=-3 => ScoreLabel::Albatross,
            -2 => ScoreLabel::Eagle,
            -1 => ScoreLabel::Birdie,
            0 => ScoreLabel::Par,
            1 => ScoreLabel::Bogey,
            2 => ScoreLabel::DoubleBogey,
            _ => ScoreLabel::TripleBogeyOrWorse,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scorecard {
    pub holes: Vec<HoleScore>,
}

impl Scorecard {
    pub fn total_strokes(&self) -> u32 {
        self.holes.iter().map(|h| h.strokes as u32).sum()
    }

    pub fn total_par(&self) -> u32 {
        self.holes.iter().map(|h| h.par as u32).sum()
    }

    pub fn total_relative(&self) -> i32 {
        self.total_strokes() as i32 - self.total_par() as i32
    }
}
