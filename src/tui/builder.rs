use crate::core::{Pos, TerrainKind};
use crate::tui::lang::Lang;
use crate::tui::render::{terrain_style, Viewport};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Widget},
};

/// Sens dans lequel la frappe directe de terrain (voir `ROADMAP.md`, builder
/// de trous) avance automatiquement le curseur : ligne par ligne pour un
/// trou pensé à dominante horizontale, colonne par colonne pour un trou
/// vertical. Choisi une fois à l'en-tête, avant de commencer à dessiner.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderOrientation {
    Horizontal,
    Vertical,
}

impl BuilderOrientation {
    pub fn toggled(self) -> Self {
        match self {
            BuilderOrientation::Horizontal => BuilderOrientation::Vertical,
            BuilderOrientation::Vertical => BuilderOrientation::Horizontal,
        }
    }

    pub fn label(self, lang: Lang) -> &'static str {
        match (self, lang) {
            (BuilderOrientation::Horizontal, Lang::En) => "Horizontal",
            (BuilderOrientation::Vertical, Lang::En) => "Vertical",
            (BuilderOrientation::Horizontal, Lang::Fr) => "Horizontal",
            (BuilderOrientation::Vertical, Lang::Fr) => "Vertical",
        }
    }
}

/// Étape d'interaction courante dans l'écran d'édition : dessin normal, ou
/// une des deux saisies de texte ponctuelles (nom, chemin de sauvegarde) qui
/// bloquent temporairement la frappe de terrain le temps de la saisie.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderMode {
    Drawing,
    EditingName,
    EnteringSavePath,
}

fn write_line(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
    if y >= area.y + area.height {
        return;
    }
    for (x, ch) in text.chars().enumerate() {
        if x as u16 >= area.width {
            break;
        }
        buf.get_mut(area.x + x as u16, y).set_char(ch).set_style(style);
    }
}

/// Comme `write_line`, mais chaque segment garde son propre style — utilisé
/// pour la légende de terrain, où chaque entrée doit reprendre la couleur
/// exacte de son terrain sur la carte (`terrain_style` dans `render.rs`)
/// plutôt qu'une seule couleur uniforme pour toute la ligne.
fn write_spans(buf: &mut Buffer, area: Rect, y: u16, segments: &[(String, Style)]) {
    if y >= area.y + area.height {
        return;
    }
    let mut x = area.x;
    let max_x = area.x + area.width;
    for (text, style) in segments {
        for ch in text.chars() {
            if x >= max_x {
                return;
            }
            buf.get_mut(x, y).set_char(ch).set_style(*style);
            x += 1;
        }
    }
}

/// Petit écran d'en-tête avant de dessiner un nouveau trou : le par visé
/// (seul champ obligatoire) et l'orientation, qui ensemble suggèrent une
/// taille de grille cohérente avec les distances de club calibrées ailleurs
/// dans le projet (voir `tools/make_hole_canvas.py`).
pub struct BuilderSetupView {
    pub lang: Lang,
    pub par: u8,
    pub orientation: BuilderOrientation,
    pub grid_size: (usize, usize),
}

impl Widget for BuilderSetupView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "New hole — setup",
            Lang::Fr => "Nouveau trou — en-tête",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let (w, h) = self.grid_size;
        let lines = [
            match self.lang {
                Lang::En => format!("Par: {}   (up/down to change)", self.par),
                Lang::Fr => format!("Par : {}   (haut/bas pour changer)", self.par),
            },
            match self.lang {
                Lang::En => format!(
                    "Orientation: {}   (left/right to change)",
                    self.orientation.label(self.lang)
                ),
                Lang::Fr => format!(
                    "Orientation : {}   (gauche/droite pour changer)",
                    self.orientation.label(self.lang)
                ),
            },
            match self.lang {
                Lang::En => format!("Suggested grid: {w}x{h} cells"),
                Lang::Fr => format!("Grille suggérée : {w}x{h} cases"),
            },
            String::new(),
            match self.lang {
                Lang::En => "Enter: start drawing    Esc: back to menu    L: language".to_string(),
                Lang::Fr => "Entrée : dessiner    Échap : retour menu    L : langue".to_string(),
            },
        ];
        for (i, line) in lines.iter().enumerate() {
            write_line(
                buf,
                inner,
                inner.y + i as u16,
                line,
                Style::default().fg(Color::White),
            );
        }
    }
}

/// Légende des caractères de terrain reconnus (voir `TerrainKind::from_char`)
/// — même légende que celle imprimée sur `tools/hole_design_canvas.pdf`,
/// pour ne pas avoir à s'en souvenir par cœur en dessinant. Les lettres
/// sont insensibles à la casse à la frappe (voir `run_builder` : le
/// caractère tapé est mis en majuscule avant d'être résolu), la légende ne
/// montre donc que la forme majuscule pour rester lisible.
/// Légende de terrain sous forme de segments colorés, un par
/// `TerrainKind`, chacun repris avec la couleur exacte que ce terrain
/// affiche sur la carte (`terrain_style` dans `render.rs`) plutôt qu'une
/// seule couleur uniforme — pour que la légende serve aussi de repère
/// visuel cohérent avec ce que le joueur voit en dessinant.
fn terrain_legend_segments(lang: Lang) -> Vec<(String, Style)> {
    const ITEMS: [(TerrainKind, &str, &str); 10] = [
        (TerrainKind::Tee, "tee", "départ"),
        (TerrainKind::Fairway, "fairway", "fairway"),
        (TerrainKind::Rough, "rough", "rough"),
        (TerrainKind::Bunker, "bunker", "bunker"),
        (TerrainKind::Water, "water", "eau"),
        (TerrainKind::Tree, "tree", "arbre"),
        (TerrainKind::Green, "green", "green"),
        (TerrainKind::Hole, "hole", "trou"),
        (TerrainKind::PenaltyZone, "penalty", "pénalité"),
        (TerrainKind::OutOfBounds, "OOB", "hors-limites"),
    ];
    let separator = Style::default().fg(Color::DarkGray);
    let mut segments = Vec::with_capacity(ITEMS.len() * 2 + 1);
    for (i, (kind, label_en, label_fr)) in ITEMS.into_iter().enumerate() {
        if i > 0 {
            segments.push((" ".to_string(), separator));
        }
        let (_, color) = terrain_style(kind);
        let key = kind.to_char();
        let key_display = match (key, lang) {
            (' ', Lang::En) => "(space)".to_string(),
            (' ', Lang::Fr) => "(espace)".to_string(),
            (c, _) => c.to_string(),
        };
        let label = if lang == Lang::En { label_en } else { label_fr };
        segments.push((
            format!("{key_display}={label}"),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ));
    }
    segments.push((
        match lang {
            Lang::En => "  — letters case-insensitive".to_string(),
            Lang::Fr => "  — lettres insensibles à la casse".to_string(),
        },
        separator,
    ));
    segments
}

/// Nombre de lignes/colonnes restantes avant que l'avancée automatique de la
/// frappe (voir `BuilderState::advance_cursor`) n'atteigne la dernière case
/// et s'arrête net — sert à colorer l'indicateur de position en
/// avertissement un peu avant d'y arriver, pas seulement une fois dessus.
const NEAR_END_THRESHOLD: usize = 3;

/// Bandeau d'information au-dessus de la grille en cours d'édition : nom,
/// par, orientation, légende des terrains, position courante dans le sens
/// de l'avancée automatique (avec avertissement à l'approche de la
/// dernière ligne/colonne), et la ligne d'instructions ou de saisie qui
/// dépend du `mode` courant (dessin, édition du nom, saisie du chemin de
/// sauvegarde).
pub struct BuilderHeaderView<'a> {
    pub lang: Lang,
    pub name: &'a str,
    pub par: u8,
    pub orientation: BuilderOrientation,
    pub cursor: Pos,
    pub grid_width: usize,
    pub grid_height: usize,
    pub mode: BuilderMode,
    pub text_input: &'a str,
    pub message: Option<&'a str>,
    pub quit_confirm: bool,
    /// Deuxième pression sur `Échap` en attente (retour au menu) — distinct
    /// de `quit_confirm` (quitter l'application entière via `qq`).
    pub exit_confirm: bool,
}

impl<'a> Widget for BuilderHeaderView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Hole builder",
            Lang::Fr => "Éditeur de trou",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let info_line = match self.lang {
            Lang::En => format!(
                "Name: {}    Par: {}    Orientation: {}",
                self.name,
                self.par,
                self.orientation.label(self.lang)
            ),
            Lang::Fr => format!(
                "Nom : {}    Par : {}    Orientation : {}",
                self.name,
                self.par,
                self.orientation.label(self.lang)
            ),
        };
        write_line(buf, inner, inner.y, &info_line, Style::default().fg(Color::White));

        write_spans(buf, inner, inner.y + 1, &terrain_legend_segments(self.lang));

        // Ligne/colonne courante dans le sens de l'avancée automatique de la
        // frappe (voir `BuilderOrientation`) : c'est cette dimension qui
        // s'arrête net en fin de grille plutôt que de boucler, donc celle
        // qui mérite un avertissement à l'approche de la fin.
        // x/y explicites, 0-indexés — mêmes coordonnées que la grille du
        // fichier `.course` et que les axes imprimés sur
        // `tools/hole_design_canvas.pdf` (origine (0,0) en haut à gauche),
        // pour naviguer directement vers une case repérée sur un plan
        // dessiné à l'avance plutôt que de deviner sa position à l'écran.
        let (position_line, remaining) = match self.orientation {
            BuilderOrientation::Horizontal => {
                let remaining = self.grid_height - 1 - self.cursor.y;
                let line = match self.lang {
                    Lang::En => format!(
                        "Position: x={} y={}   (row {}/{})",
                        self.cursor.x,
                        self.cursor.y,
                        self.cursor.y + 1,
                        self.grid_height
                    ),
                    Lang::Fr => format!(
                        "Position : x={} y={}   (ligne {}/{})",
                        self.cursor.x,
                        self.cursor.y,
                        self.cursor.y + 1,
                        self.grid_height
                    ),
                };
                (line, remaining)
            }
            BuilderOrientation::Vertical => {
                let remaining = self.grid_width - 1 - self.cursor.x;
                let line = match self.lang {
                    Lang::En => format!(
                        "Position: x={} y={}   (column {}/{})",
                        self.cursor.x,
                        self.cursor.y,
                        self.cursor.x + 1,
                        self.grid_width
                    ),
                    Lang::Fr => format!(
                        "Position : x={} y={}   (colonne {}/{})",
                        self.cursor.x,
                        self.cursor.y,
                        self.cursor.x + 1,
                        self.grid_width
                    ),
                };
                (line, remaining)
            }
        };
        let position_style = if remaining == 0 {
            Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD)
        } else if remaining <= NEAR_END_THRESHOLD {
            Style::default().fg(Color::LightYellow).add_modifier(Modifier::BOLD)
        } else {
            Style::default().fg(Color::White)
        };
        let position_line = if remaining == 0 {
            match self.lang {
                Lang::En => format!("{position_line} — last one, won't wrap around"),
                Lang::Fr => format!("{position_line} — dernière, pas de retour au début"),
            }
        } else {
            position_line
        };
        write_line(buf, inner, inner.y + 2, &position_line, position_style);

        let fourth_line = if self.quit_confirm {
            match self.lang {
                Lang::En => "Press q again to quit without saving".to_string(),
                Lang::Fr => "Appuyez encore sur q pour quitter sans sauvegarder".to_string(),
            }
        } else if self.exit_confirm {
            match self.lang {
                Lang::En => {
                    "Press Esc again to discard and return to menu (S to save first)".to_string()
                }
                Lang::Fr => {
                    "Appuyez encore sur Échap pour abandonner et revenir au menu \
                     (S pour sauvegarder d'abord)"
                        .to_string()
                }
            }
        } else {
            match self.mode {
                BuilderMode::Drawing => match self.lang {
                    Lang::En => {
                        "Type a terrain char to paint & advance   arrows: move   \
                         U: undo   N: rename   S: save   Esc Esc: menu   qq: quit"
                            .to_string()
                    }
                    Lang::Fr => {
                        "Tapez un caractère de terrain pour peindre et avancer   \
                         flèches : déplacer   U : annuler   N : renommer   \
                         S : sauvegarder   Échap Échap : menu   qq : quitter"
                            .to_string()
                    }
                },
                BuilderMode::EditingName => match self.lang {
                    Lang::En => format!("New name: {}_ (Enter to confirm, Esc to cancel)", self.text_input),
                    Lang::Fr => format!("Nouveau nom : {}_ (Entrée valider, Échap annuler)", self.text_input),
                },
                BuilderMode::EnteringSavePath => match self.lang {
                    Lang::En => format!("Save to: {}_ (Enter to confirm, Esc to cancel)", self.text_input),
                    Lang::Fr => format!("Sauvegarder sous : {}_ (Entrée valider, Échap annuler)", self.text_input),
                },
            }
        };
        write_line(
            buf,
            inner,
            inner.y + 3,
            &fourth_line,
            Style::default().fg(Color::DarkGray),
        );

        if let Some(message) = self.message {
            write_line(
                buf,
                inner,
                inner.y + 4,
                message,
                Style::default().fg(Color::LightRed).add_modifier(Modifier::BOLD),
            );
        }
    }
}

/// Widget affichant la grille en cours d'édition, avec le curseur surligné
/// à la place de la balle/du trou "réels" du jeu (rien n'est encore validé
/// tant que le fichier n'est pas sauvegardé) — calqué sur la boucle
/// cellule-par-cellule de `CourseView` (`render.rs`), mais sans zoom ni
/// aucune des surimpressions d'aperçu de coup, qui n'ont pas de sens ici.
pub struct BuilderView<'a> {
    pub tiles: &'a [Vec<TerrainKind>],
    pub cursor: Pos,
    pub viewport_w: usize,
    pub viewport_h: usize,
}

impl<'a> Widget for BuilderView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let grid_h = self.tiles.len();
        let grid_w = self.tiles.first().map(|r| r.len()).unwrap_or(0);
        if grid_w == 0 || grid_h == 0 {
            return;
        }

        let view_w = (inner.width as usize).min(self.viewport_w).max(1);
        let view_h = (inner.height as usize).min(self.viewport_h).max(1);
        let viewport = Viewport { width: view_w, height: view_h };
        let (ox, oy) = viewport.top_left(self.cursor, grid_w, grid_h);

        let content_w = grid_w.min(view_w);
        let content_h = grid_h.min(view_h);
        let margin_x = (inner.width - content_w as u16) / 2;
        let margin_y = (inner.height - content_h as u16) / 2;

        for row in 0..content_h {
            for col in 0..content_w {
                let gx = ox + col;
                let gy = oy + row;
                let terrain = self.tiles[gy][gx];
                let (ch, color) = terrain_style(terrain);
                let is_cursor = gx == self.cursor.x && gy == self.cursor.y;

                let x = inner.x + margin_x + col as u16;
                let y = inner.y + margin_y + row as u16;
                if x >= inner.x + inner.width || y >= inner.y + inner.height {
                    continue;
                }
                let style = if is_cursor {
                    Style::default()
                        .fg(Color::Black)
                        .bg(Color::LightYellow)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(color)
                };
                buf.get_mut(x, y).set_char(ch).set_style(style);
            }
        }
    }
}
