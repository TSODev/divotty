use crate::core::terrain::TerrainKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const COURSE_WIDTH: usize = 50;
pub const COURSE_HEIGHT: usize = 25;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct Pos {
    pub x: usize,
    pub y: usize,
}

/// Métadonnées d'un trou, lues depuis le frontmatter YAML du fichier `.course`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HoleMeta {
    pub name: String,
    pub par: u8,
    #[serde(default)]
    pub description: Option<String>,
}

/// Un trou complet : métadonnées + grille de terrain.
#[derive(Debug, Clone)]
pub struct Hole {
    pub meta: HoleMeta,
    pub tiles: Vec<Vec<TerrainKind>>, // [y][x], dimensions COURSE_HEIGHT x COURSE_WIDTH
    pub tee: Pos,
    pub hole_pos: Pos,
}

/// Un parcours complet : 1, 9 ou 18 trous joués dans l'ordre.
#[derive(Debug, Clone)]
pub struct Course {
    pub name: String,
    /// Indice de difficulté du parcours, de 1 à 4 étoiles. Fixé manuellement
    /// par le créateur de la carte dans `course.yaml` — ce n'est qu'une
    /// indication pour le joueur (choix de parcours, tri), aucun calcul de
    /// jeu n'en dépend.
    pub difficulty: u8,
    pub holes: Vec<Hole>,
}

#[derive(Debug, Error)]
pub enum CourseError {
    #[error("fichier de trou invalide, frontmatter YAML manquant ou mal formé")]
    MissingFrontmatter,
    #[error("erreur de parsing YAML: {0}")]
    Yaml(#[from] serde_yaml::Error),
    #[error("erreur de lecture fichier: {0}")]
    Io(#[from] std::io::Error),
    #[error("grille invalide: attendu {expected_w}x{expected_h}, ligne {line} a une largeur de {actual_w}")]
    BadDimensions {
        expected_w: usize,
        expected_h: usize,
        line: usize,
        actual_w: usize,
    },
    #[error("caractère de terrain inconnu '{0}' à la ligne {1}, colonne {2}")]
    UnknownTerrainChar(char, usize, usize),
    #[error("aucune case de départ (D) trouvée sur le trou '{0}'")]
    NoTee(String),
    #[error("aucune case d'arrivée (H) trouvée sur le trou '{0}'")]
    NoHole(String),
    #[error("plusieurs cases de départ trouvées sur le trou '{0}', une seule est autorisée")]
    MultipleTee(String),
    #[error("plusieurs cases d'arrivée trouvées sur le trou '{0}', une seule est autorisée")]
    MultipleHole(String),
    #[error("difficulté invalide pour le parcours '{name}': {difficulty} (attendu 1 à 4)")]
    InvalidDifficulty { name: String, difficulty: u8 },
}

impl Hole {
    /// Parse un fichier `.course` unique : frontmatter YAML délimité par `---`,
    /// suivi de la grille ASCII (voir `TerrainKind::from_char` pour la légende).
    pub fn parse(raw: &str) -> Result<Self, CourseError> {
        let mut parts = raw.splitn(3, "---");
        // Le fichier commence directement par le frontmatter, donc le premier
        // split (avant le premier ---) doit être vide ou absent selon le format choisi.
        let (meta_str, grid_str) = match (parts.next(), parts.next(), parts.next()) {
            (Some(m), Some(g), None) => (m, g), // pas de --- initial
            (Some(_empty), Some(m), Some(g)) => (m, g), // --- initial présent
            _ => return Err(CourseError::MissingFrontmatter),
        };

        let meta: HoleMeta = serde_yaml::from_str(meta_str.trim())?;

        let mut tiles = Vec::with_capacity(COURSE_HEIGHT);
        let mut tee = None;
        let mut hole_pos = None;

        let lines: Vec<&str> = grid_str
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.is_empty())
            .collect();

        for (y, line) in lines.iter().enumerate() {
            if line.chars().count() != COURSE_WIDTH {
                return Err(CourseError::BadDimensions {
                    expected_w: COURSE_WIDTH,
                    expected_h: COURSE_HEIGHT,
                    line: y,
                    actual_w: line.chars().count(),
                });
            }
            let mut row = Vec::with_capacity(COURSE_WIDTH);
            for (x, c) in line.chars().enumerate() {
                let terrain = TerrainKind::from_char(c)
                    .ok_or(CourseError::UnknownTerrainChar(c, y, x))?;
                match terrain {
                    TerrainKind::Tee => {
                        if tee.is_some() {
                            return Err(CourseError::MultipleTee(meta.name.clone()));
                        }
                        tee = Some(Pos { x, y });
                    }
                    TerrainKind::Hole => {
                        if hole_pos.is_some() {
                            return Err(CourseError::MultipleHole(meta.name.clone()));
                        }
                        hole_pos = Some(Pos { x, y });
                    }
                    _ => {}
                }
                row.push(terrain);
            }
            tiles.push(row);
        }

        let tee = tee.ok_or_else(|| CourseError::NoTee(meta.name.clone()))?;
        let hole_pos = hole_pos.ok_or_else(|| CourseError::NoHole(meta.name.clone()))?;

        Ok(Hole {
            meta,
            tiles,
            tee,
            hole_pos,
        })
    }

    pub fn terrain_at(&self, pos: Pos) -> Option<TerrainKind> {
        self.tiles.get(pos.y)?.get(pos.x).copied()
    }
}

impl Course {
    /// Charge un parcours depuis un dossier contenant un `course.yaml`
    /// (nom du parcours + ordre des fichiers de trous) et les fichiers `.course`
    /// correspondants.
    pub fn load_from_dir(dir: &Path) -> Result<Self, CourseError> {
        #[derive(Deserialize)]
        struct CourseIndex {
            name: String,
            difficulty: u8,
            holes: Vec<String>,
        }

        let index_raw = std::fs::read_to_string(dir.join("course.yaml"))?;
        let index: CourseIndex = serde_yaml::from_str(&index_raw)?;

        if !(1..=4).contains(&index.difficulty) {
            return Err(CourseError::InvalidDifficulty {
                name: index.name,
                difficulty: index.difficulty,
            });
        }

        let mut holes = Vec::with_capacity(index.holes.len());
        for filename in &index.holes {
            let raw = std::fs::read_to_string(dir.join(filename))?;
            holes.push(Hole::parse(&raw)?);
        }

        Ok(Course {
            name: index.name,
            difficulty: index.difficulty,
            holes,
        })
    }

    /// Liste les parcours jouables sous `root` : chaque sous-dossier contenant
    /// un `course.yaml` est chargé via `load_from_dir`. Triés par nom, pour un
    /// affichage stable dans un menu de sélection. Le dossier d'origine de
    /// chaque parcours est renvoyé avec lui, pour permettre de le recharger
    /// plus tard (ex: reprise d'une partie sauvegardée).
    pub fn discover(root: &Path) -> Result<Vec<(PathBuf, Course)>, CourseError> {
        let mut courses = Vec::new();
        for entry in std::fs::read_dir(root)? {
            let path = entry?.path();
            if path.is_dir() && path.join("course.yaml").exists() {
                let course = Course::load_from_dir(&path)?;
                courses.push((path, course));
            }
        }
        courses.sort_by(|a, b| a.1.name.cmp(&b.1.name));
        Ok(courses)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_grid_line(kind: char) -> String {
        std::iter::repeat(kind).take(COURSE_WIDTH).collect()
    }

    fn build_valid_raw() -> String {
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        for y in 0..COURSE_HEIGHT {
            let mut line: Vec<char> = sample_grid_line('.').chars().collect();
            if y == 0 {
                line[0] = 'D';
            }
            if y == COURSE_HEIGHT - 1 {
                line[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(line.into_iter().collect::<String>());
        }
        format!(
            "name: \"Trou de test\"\npar: 3\n---\n{}\n",
            lines.join("\n")
        )
    }

    #[test]
    fn parses_valid_hole() {
        let raw = build_valid_raw();
        let hole = Hole::parse(&raw).expect("le trou de test doit parser");
        assert_eq!(hole.meta.par, 3);
        assert_eq!(hole.tee, Pos { x: 0, y: 0 });
        assert_eq!(hole.hole_pos, Pos { x: COURSE_WIDTH - 1, y: COURSE_HEIGHT - 1 });
    }

    #[test]
    fn rejects_missing_tee() {
        let raw = build_valid_raw().replace('D', ".");
        let err = Hole::parse(&raw).unwrap_err();
        assert!(matches!(err, CourseError::NoTee(_)));
    }

    #[test]
    fn loads_demo_course_from_disk() {
        // Vérifie que le fichier .course d'exemple fourni dans courses/demo/
        // parse correctement (sert de test de non-régression sur le format).
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("courses/demo");
        let course = Course::load_from_dir(&dir).expect("le parcours de démo doit charger");
        assert_eq!(course.holes.len(), 1);
        assert_eq!(course.holes[0].meta.par, 4);
        assert!((1..=4).contains(&course.difficulty));
    }

    #[test]
    fn discovers_demo_course_on_disk() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("courses");
        let courses = Course::discover(&root).expect("la découverte doit réussir");
        assert!(courses
            .iter()
            .any(|(_, c)| c.name == "Parcours de démonstration"));
    }

    #[test]
    fn rejects_out_of_range_difficulty() {
        let dir = std::env::temp_dir().join(format!(
            "divotty_test_difficulty_{}",
            std::process::id()
        ));
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(
            dir.join("course.yaml"),
            "name: \"Test\"\ndifficulty: 5\nholes:\n  - hole_01.course\n",
        )
        .unwrap();
        std::fs::write(dir.join("hole_01.course"), build_valid_raw()).unwrap();

        let err = Course::load_from_dir(&dir).unwrap_err();
        assert!(matches!(err, CourseError::InvalidDifficulty { .. }));

        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn rejects_bad_width() {
        let mut raw = build_valid_raw();
        raw = raw.replacen(&sample_grid_line('.'), &sample_grid_line('.')[..COURSE_WIDTH - 1], 1);
        let err = Hole::parse(&raw).unwrap_err();
        assert!(matches!(err, CourseError::BadDimensions { .. }));
    }
}
