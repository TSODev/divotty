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

/// Cumul des scores d'un parcours, un `HoleScore` par trou déjà quitté (le
/// trou en cours de jeu n'y figure pas tant qu'il n'est pas terminé et
/// confirmé — voir `GameState::advance_hole` côté `app`).
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Scorecard {
    pub holes: Vec<HoleScore>,
}

impl Scorecard {
    pub fn push(&mut self, score: HoleScore) {
        self.holes.push(score);
    }

    pub fn total_strokes(&self) -> u32 {
        self.holes.iter().map(|h| h.strokes as u32).sum()
    }

    pub fn total_par(&self) -> u32 {
        self.holes.iter().map(|h| h.par as u32).sum()
    }

    pub fn relative_to_par(&self) -> i32 {
        self.total_strokes() as i32 - self.total_par() as i32
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn label_matches_strokes_relative_to_par() {
        let case = |strokes, par| HoleScore { strokes, par }.label();
        assert_eq!(case(2, 5), ScoreLabel::Albatross);
        assert_eq!(case(3, 5), ScoreLabel::Eagle);
        assert_eq!(case(4, 5), ScoreLabel::Birdie);
        assert_eq!(case(5, 5), ScoreLabel::Par);
        assert_eq!(case(6, 5), ScoreLabel::Bogey);
        assert_eq!(case(7, 5), ScoreLabel::DoubleBogey);
        assert_eq!(case(8, 5), ScoreLabel::TripleBogeyOrWorse);
    }

    #[test]
    fn scorecard_accumulates_totals_across_holes() {
        let mut card = Scorecard::default();
        assert_eq!(card.total_strokes(), 0);
        assert_eq!(card.total_par(), 0);
        assert_eq!(card.relative_to_par(), 0);

        card.push(HoleScore { strokes: 5, par: 4 }); // +1
        card.push(HoleScore { strokes: 3, par: 4 }); // -1
        card.push(HoleScore { strokes: 6, par: 5 }); // +1

        assert_eq!(card.total_strokes(), 14);
        assert_eq!(card.total_par(), 13);
        assert_eq!(card.relative_to_par(), 1);
    }
}
