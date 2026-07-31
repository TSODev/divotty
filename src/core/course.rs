use crate::core::terrain::TerrainKind;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use thiserror::Error;

pub const COURSE_WIDTH: usize = 100;
pub const COURSE_HEIGHT: usize = 60;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Dimensions de la grille ASCII déclarée dans ce fichier, si le trou
    /// n'utilise pas le canevas 100x60 complet. `None` (valeur par défaut,
    /// absente du frontmatter) signifie "grille 100x60 pleine" — tous les
    /// fichiers `.course` existants restent donc valides sans modification.
    /// La petite grille est centrée dans le canevas complet une fois parsée
    /// (voir `Hole::parse`), donc `shot.rs`/`render.rs` ne voient jamais que
    /// des `Hole` toujours 100x60.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub width: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub height: Option<usize>,
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
    #[error("grille invalide: attendu {expected_h} lignes, {actual_h} trouvées")]
    BadRowCount { expected_h: usize, actual_h: usize },
    #[error(
        "taille déclarée invalide pour le trou '{name}': {width}x{height} dépasse le \
         format maximal {max_w}x{max_h}"
    )]
    DeclaredSizeTooLarge {
        name: String,
        width: usize,
        height: usize,
        max_w: usize,
        max_h: usize,
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

        let declared_w = meta.width.unwrap_or(COURSE_WIDTH);
        let declared_h = meta.height.unwrap_or(COURSE_HEIGHT);
        if declared_w > COURSE_WIDTH || declared_h > COURSE_HEIGHT {
            return Err(CourseError::DeclaredSizeTooLarge {
                name: meta.name.clone(),
                width: declared_w,
                height: declared_h,
                max_w: COURSE_WIDTH,
                max_h: COURSE_HEIGHT,
            });
        }

        let mut local_tiles = Vec::with_capacity(declared_h);
        let mut local_tee = None;
        let mut local_hole_pos = None;

        let lines: Vec<&str> = grid_str
            .lines()
            .map(|l| l.trim_end_matches('\r'))
            .filter(|l| !l.is_empty())
            .collect();

        if lines.len() != declared_h {
            return Err(CourseError::BadRowCount {
                expected_h: declared_h,
                actual_h: lines.len(),
            });
        }

        for (y, line) in lines.iter().enumerate() {
            if line.chars().count() != declared_w {
                return Err(CourseError::BadDimensions {
                    expected_w: declared_w,
                    expected_h: declared_h,
                    line: y,
                    actual_w: line.chars().count(),
                });
            }
            let mut row = Vec::with_capacity(declared_w);
            for (x, c) in line.chars().enumerate() {
                let terrain = TerrainKind::from_char(c)
                    .ok_or(CourseError::UnknownTerrainChar(c, y, x))?;
                match terrain {
                    TerrainKind::Tee => {
                        if local_tee.is_some() {
                            return Err(CourseError::MultipleTee(meta.name.clone()));
                        }
                        local_tee = Some(Pos { x, y });
                    }
                    TerrainKind::Hole => {
                        if local_hole_pos.is_some() {
                            return Err(CourseError::MultipleHole(meta.name.clone()));
                        }
                        local_hole_pos = Some(Pos { x, y });
                    }
                    _ => {}
                }
                row.push(terrain);
            }
            local_tiles.push(row);
        }

        let local_tee = local_tee.ok_or_else(|| CourseError::NoTee(meta.name.clone()))?;
        let local_hole_pos = local_hole_pos.ok_or_else(|| CourseError::NoHole(meta.name.clone()))?;

        // Centre la petite grille dans le canevas 100x60 complet, entouré de
        // hors-limites — le reste du moteur (shot.rs, render.rs, Viewport) ne
        // voit donc jamais qu'un `Hole` toujours 100x60, quelle que soit la
        // taille déclarée. Le reste éventuel d'une différence impaire va en
        // bas/à droite plutôt qu'en haut/à gauche (détail cosmétique, sans
        // conséquence sur le jeu).
        let offset_x = (COURSE_WIDTH - declared_w) / 2;
        let offset_y = (COURSE_HEIGHT - declared_h) / 2;

        let mut tiles = vec![vec![TerrainKind::OutOfBounds; COURSE_WIDTH]; COURSE_HEIGHT];
        for (y, row) in local_tiles.into_iter().enumerate() {
            for (x, terrain) in row.into_iter().enumerate() {
                tiles[offset_y + y][offset_x + x] = terrain;
            }
        }
        let tee = Pos {
            x: local_tee.x + offset_x,
            y: local_tee.y + offset_y,
        };
        let hole_pos = Pos {
            x: local_hole_pos.x + offset_x,
            y: local_hole_pos.y + offset_y,
        };

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

    /// Sérialise ce trou vers le format `.course` (frontmatter YAML + grille
    /// ASCII) — inverse de `Hole::parse`. Si `meta.width`/`meta.height` sont
    /// déclarés, seule la sous-grille correspondante (au même offset de
    /// centrage que `Hole::parse`) est réécrite ; sinon la grille 100x60
    /// complète l'est. Utilisée par le builder de trous (voir `ROADMAP.md`)
    /// pour produire un fichier valide par construction avant de l'écrire
    /// sur disque : sauvegarder, c'est appeler cette fonction puis vérifier
    /// que `Hole::parse` du résultat réussit.
    pub fn to_course_string(&self) -> String {
        let mut meta_yaml =
            serde_yaml::to_string(&self.meta).expect("HoleMeta se sérialise toujours en YAML");
        if !meta_yaml.ends_with('\n') {
            meta_yaml.push('\n');
        }

        let declared_w = self.meta.width.unwrap_or(COURSE_WIDTH);
        let declared_h = self.meta.height.unwrap_or(COURSE_HEIGHT);
        let offset_x = (COURSE_WIDTH - declared_w) / 2;
        let offset_y = (COURSE_HEIGHT - declared_h) / 2;

        let lines: Vec<String> = (0..declared_h)
            .map(|y| {
                (0..declared_w)
                    .map(|x| self.tiles[offset_y + y][offset_x + x].to_char())
                    .collect()
            })
            .collect();

        format!("{}---\n{}\n", meta_yaml, lines.join("\n"))
    }
}

#[derive(Deserialize)]
struct CourseIndex {
    name: String,
    difficulty: u8,
    holes: Vec<String>,
}

impl CourseIndex {
    fn parse(yaml: &str) -> Result<Self, CourseError> {
        let index: CourseIndex = serde_yaml::from_str(yaml)?;
        if !(1..=4).contains(&index.difficulty) {
            return Err(CourseError::InvalidDifficulty {
                name: index.name,
                difficulty: index.difficulty,
            });
        }
        Ok(index)
    }
}

impl Course {
    /// Charge un parcours depuis un dossier contenant un `course.yaml`
    /// (nom du parcours + ordre des fichiers de trous) et les fichiers `.course`
    /// correspondants.
    pub fn load_from_dir(dir: &Path) -> Result<Self, CourseError> {
        let index_raw = std::fs::read_to_string(dir.join("course.yaml"))?;
        let index = CourseIndex::parse(&index_raw)?;

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

    /// Construit un parcours à partir de contenu déjà en mémoire (`course.yaml`
    /// + un fichier `.course` par trou, dans l'ordre déclaré par `course.yaml`),
    /// sans passer par le disque. Utilisé pour les parcours embarqués dans le
    /// binaire (`include_str!` dans `main.rs`) : un joueur qui lance `divotty`
    /// après un `cargo install`, sans le dossier `courses/` à côté (voir
    /// `CLAUDE.md`), voit ainsi les vrais parcours plutôt qu'un unique trou
    /// générique de secours.
    pub fn from_embedded(course_yaml: &str, hole_raws: &[&str]) -> Result<Self, CourseError> {
        let index = CourseIndex::parse(course_yaml)?;

        let mut holes = Vec::with_capacity(hole_raws.len());
        for raw in hole_raws {
            holes.push(Hole::parse(raw)?);
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
    fn loads_quick3_course_from_disk() {
        // Vérifie que le parcours à 3 trous (courses/quick3/) parse
        // correctement — sert de non-régression sur l'enchaînement
        // multi-trous (voir ROADMAP v0.2, phase 4).
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("courses/quick3");
        let course = Course::load_from_dir(&dir).expect("le parcours Quick 3 doit charger");
        assert_eq!(course.holes.len(), 3);
        assert_eq!(course.holes.iter().map(|h| h.meta.par as u32).sum::<u32>(), 12);
        assert!((1..=4).contains(&course.difficulty));
    }

    #[test]
    fn discovers_quick3_course_on_disk() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("courses");
        let courses = Course::discover(&root).expect("la découverte doit réussir");
        assert!(courses.iter().any(|(_, c)| c.name == "Quick 3"));
    }

    #[test]
    fn from_embedded_parses_content_held_in_memory() {
        let course_yaml = "name: \"Test embarqué\"\ndifficulty: 2\nholes:\n  - unused.course\n";
        let hole_raw = build_valid_raw();

        let course = Course::from_embedded(course_yaml, &[&hole_raw])
            .expect("le contenu embarqué valide doit parser");

        assert_eq!(course.name, "Test embarqué");
        assert_eq!(course.difficulty, 2);
        assert_eq!(course.holes.len(), 1);
        assert_eq!(course.holes[0].meta.par, 3);
    }

    #[test]
    fn from_embedded_rejects_out_of_range_difficulty() {
        let course_yaml = "name: \"Test\"\ndifficulty: 9\nholes:\n  - unused.course\n";
        let hole_raw = build_valid_raw();

        let err = Course::from_embedded(course_yaml, &[&hole_raw]).unwrap_err();
        assert!(matches!(err, CourseError::InvalidDifficulty { .. }));
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

    fn build_small_raw(width: usize, height: usize) -> String {
        let mut lines = Vec::with_capacity(height);
        for y in 0..height {
            let mut line: Vec<char> = std::iter::repeat('.').take(width).collect();
            if y == 0 {
                line[0] = 'D';
            }
            if y == height - 1 {
                line[width - 1] = 'H';
            }
            lines.push(line.into_iter().collect::<String>());
        }
        format!(
            "name: \"Petit trou\"\npar: 3\nwidth: {}\nheight: {}\n---\n{}\n",
            width,
            height,
            lines.join("\n")
        )
    }

    #[test]
    fn small_hole_is_centered_in_the_full_canvas() {
        let raw = build_small_raw(20, 10);
        let hole = Hole::parse(&raw).expect("un petit trou déclaré doit parser");

        let offset_x = (COURSE_WIDTH - 20) / 2;
        let offset_y = (COURSE_HEIGHT - 10) / 2;
        assert_eq!(hole.tee, Pos { x: offset_x, y: offset_y });
        assert_eq!(
            hole.hole_pos,
            Pos { x: offset_x + 19, y: offset_y + 9 }
        );
        // Le canevas final reste toujours 100x60, peu importe la taille déclarée.
        assert_eq!(hole.tiles.len(), COURSE_HEIGHT);
        assert_eq!(hole.tiles[0].len(), COURSE_WIDTH);
        // Une case hors de la petite grille est bien du hors-limites.
        assert_eq!(hole.terrain_at(Pos { x: 0, y: 0 }), Some(TerrainKind::OutOfBounds));
    }

    #[test]
    fn rejects_declared_size_larger_than_the_canvas() {
        let raw = build_small_raw(COURSE_WIDTH + 1, 10);
        let err = Hole::parse(&raw).unwrap_err();
        assert!(matches!(err, CourseError::DeclaredSizeTooLarge { .. }));
    }

    #[test]
    fn rejects_row_count_mismatching_the_declared_height() {
        let mut raw = build_small_raw(20, 10);
        // Retire la dernière ligne de la grille sans toucher au frontmatter,
        // pour que le nombre de lignes ne corresponde plus à `height: 10`.
        if let Some(idx) = raw.rfind('\n') {
            raw.truncate(idx);
        }
        if let Some(idx) = raw.rfind('\n') {
            raw.truncate(idx);
        }
        let err = Hole::parse(&raw).unwrap_err();
        assert!(matches!(err, CourseError::BadRowCount { .. }));
    }

    #[test]
    fn full_size_hole_without_declared_dimensions_is_unaffected() {
        // Non-régression : un fichier 100x60 sans `width`/`height` continue
        // de parser à l'identique (tee/trou non translatés).
        let raw = build_valid_raw();
        let hole = Hole::parse(&raw).expect("le trou plein format doit parser");
        assert_eq!(hole.tee, Pos { x: 0, y: 0 });
        assert_eq!(hole.hole_pos, Pos { x: COURSE_WIDTH - 1, y: COURSE_HEIGHT - 1 });
    }

    #[test]
    fn to_course_string_roundtrips_the_demo_hole() {
        let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("courses/demo");
        let course = Course::load_from_dir(&dir).expect("le parcours de démo doit charger");
        let hole = &course.holes[0];

        let reparsed = Hole::parse(&hole.to_course_string())
            .expect("la sortie de to_course_string doit re-parser");

        assert_eq!(reparsed.meta.name, hole.meta.name);
        assert_eq!(reparsed.meta.par, hole.meta.par);
        assert_eq!(reparsed.tee, hole.tee);
        assert_eq!(reparsed.hole_pos, hole.hole_pos);
        assert_eq!(reparsed.tiles, hole.tiles);
    }

    #[test]
    fn to_course_string_roundtrips_a_small_declared_hole() {
        let raw = build_small_raw(20, 10);
        let hole = Hole::parse(&raw).expect("le petit trou doit parser");

        let reparsed = Hole::parse(&hole.to_course_string())
            .expect("la sortie de to_course_string doit re-parser");

        assert_eq!(reparsed.tee, hole.tee);
        assert_eq!(reparsed.hole_pos, hole.hole_pos);
        assert_eq!(reparsed.tiles, hole.tiles);
    }
}
