use crate::core::{Hole, Pos, TerrainKind};
use crate::tui::lang::Lang;
use crate::tui::render::{terrain_style, Viewport};
use crate::tui::sidebar::{panel, panel_bottom_aligned};
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Direction as LayoutDirection, Layout, Rect},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, BorderType, Borders, Widget},
};
use std::path::{Path, PathBuf};

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

/// Étape d'interaction courante dans l'écran d'édition : dessin normal,
/// l'une des deux saisies de texte ponctuelles (nom du trou, nom de fichier
/// de sauvegarde) qui bloquent temporairement la frappe de terrain le temps
/// de la saisie, ou une confirmation d'écrasement.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BuilderMode {
    Drawing,
    EditingName,
    /// Saisie du nom de fichier (sans extension, sans chemin) pour la
    /// sauvegarde — toujours écrit dans `courses/_library/`, voir
    /// `HOLE_LIBRARY_DIR`/`sanitize_hole_filename` dans `main.rs`. Le
    /// joueur ne choisit jamais l'emplacement lui-même.
    EnteringSaveName,
    /// Le nom de fichier saisi correspond à un fichier déjà présent dans
    /// `courses/_library/` — demande confirmation avant d'écraser plutôt
    /// que d'ajouter un compteur automatique (qui empêcherait de mettre à
    /// jour un trou déjà sauvegardé : chaque sauvegarde créerait un
    /// nouveau fichier au lieu de remplacer l'ancien). `Entrée`/`Y` pour
    /// écraser, `Échap`/`N` pour revenir à la saisie du nom et le changer.
    ConfirmOverwrite,
    /// En attente d'une touche de terrain (voir `C`, "combler") pour
    /// remplacer d'un coup toutes les cases encore hors-limites par ce
    /// terrain — jamais les cases déjà peintes, pour rester sans risque à
    /// utiliser même après avoir commencé à détailler un trou. `Échap`
    /// annule sans rien changer.
    FillingBackground,
}

pub(crate) fn write_line(buf: &mut Buffer, area: Rect, y: u16, text: &str, style: Style) {
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

/// Découpe `text` en lignes d'au plus `width` caractères, à la limite des
/// mots — utilisé pour les messages de sauvegarde/erreur dans le panneau
/// Contrôles de `BuilderSidebarView`, trop étroit (~24 caractères utiles)
/// pour une seule ligne comme dans l'ancien bandeau pleine largeur.
pub(crate) fn wrap_text(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        if current.is_empty() {
            current.push_str(word);
        } else if current.len() + 1 + word.len() <= width {
            current.push(' ');
            current.push_str(word);
        } else {
            lines.push(std::mem::take(&mut current));
            current.push_str(word);
        }
    }
    if !current.is_empty() {
        lines.push(current);
    }
    lines
}

/// Panneau de gauche avant de dessiner un nouveau trou : le par visé (seul
/// champ obligatoire) et l'orientation, qui ensemble suggèrent une taille
/// de grille cohérente avec les distances de club calibrées ailleurs dans
/// le projet (voir `tools/make_hole_canvas.py`). Affiché à côté d'un aperçu
/// de la grille vierge à cette taille (voir `setup_builder` dans
/// `main.rs`, qui construit ce second panneau) — même esprit deux colonnes
/// que le reste du builder plutôt qu'un unique panneau plein écran.
pub struct BuilderSetupView {
    pub lang: Lang,
    pub par: u8,
    pub orientation: BuilderOrientation,
    pub grid_size: (usize, usize),
}

impl Widget for BuilderSetupView {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "New hole",
            Lang::Fr => "Nouveau trou",
        };
        let (w, h) = self.grid_size;
        let lines = vec![
            Line::styled(
                match self.lang {
                    Lang::En => format!("Par: {}", self.par),
                    Lang::Fr => format!("Par : {}", self.par),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Orientation: {}", self.orientation.label(self.lang)),
                    Lang::Fr => format!("Orientation : {}", self.orientation.label(self.lang)),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Grid: {w}x{h}"),
                    Lang::Fr => format!("Grille : {w}x{h}"),
                },
                Style::default().fg(Color::White),
            ),
            Line::from(""),
            Line::styled(
                match self.lang {
                    Lang::En => "Up/Down  change par",
                    Lang::Fr => "Haut/Bas  changer par",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Left/Right  orientation",
                    Lang::Fr => "Gauche/Droite  orientation",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Enter  start drawing",
                    Lang::Fr => "Entrée  dessiner",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "Esc  back to menu",
                    Lang::Fr => "Échap  retour menu",
                },
                Style::default().fg(Color::Gray),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => "L  language",
                    Lang::Fr => "L  langue",
                },
                Style::default().fg(Color::Gray),
            ),
        ];
        panel(area, buf, title, HOLE_ACCENT, lines);
    }
}

/// Écran d'entrée du builder : "+ Nouveau trou" en premier, puis chaque
/// fichier `.course` trouvé sous `courses/` — choisir un fichier existant
/// affiche ensuite une confirmation "Modifier ou Dupliquer ?" à la place de
/// la ligne d'aide habituelle (voir `pick_hole_to_build` dans `main.rs`).
pub struct HolePickerView<'a> {
    pub lang: Lang,
    pub files: &'a [PathBuf],
    pub selected: usize,
    /// `Some(chemin)` quand un fichier vient d'être choisi et qu'on attend
    /// la confirmation modifier/dupliquer.
    pub confirm_target: Option<&'a Path>,
    /// Racine `courses/` effective (préfixée par `data_root`, voir
    /// `resolve_data_root` dans `main.rs`) — dépouillée de l'affichage pour
    /// ne montrer que le chemin relatif (`demo/hole_01.course`), même
    /// quand la racine réelle est un chemin absolu du dossier de données
    /// de la plateforme.
    pub courses_root: &'a Path,
}

impl<'a> Widget for HolePickerView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Hole builder — choose",
            Lang::Fr => "Éditeur de trou — choisir",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        let new_hole_label = match self.lang {
            Lang::En => "+ New hole",
            Lang::Fr => "+ Nouveau trou",
        };
        let mut entries: Vec<String> = vec![new_hole_label.to_string()];
        entries.extend(self.files.iter().map(|path| {
            path.strip_prefix(self.courses_root)
                .unwrap_or(path)
                .display()
                .to_string()
        }));

        // Une ligne par information plutôt qu'une seule ligne longue
        // (~60-70 caractères) tronquée dans cette colonne étroite — signalé
        // après avoir constaté la troncature, corrigé comme les autres
        // panneaux d'aide du builder (une entrée par ligne).
        let hint_width = (inner.width as usize).saturating_sub(1);
        let hint_lines: Vec<String> = if let Some(path) = self.confirm_target {
            let label = path
                .strip_prefix(self.courses_root)
                .unwrap_or(path)
                .display()
                .to_string();
            match self.lang {
                Lang::En => {
                    let mut lines = wrap_text(&format!("'{label}'"), hint_width);
                    lines.push("M  modify in place".to_string());
                    lines.push("D  duplicate".to_string());
                    lines.push("Esc  back".to_string());
                    lines
                }
                Lang::Fr => {
                    let mut lines = wrap_text(&format!("« {label} »"), hint_width);
                    lines.push("M  modifier sur place".to_string());
                    lines.push("D  dupliquer".to_string());
                    lines.push("Échap  retour".to_string());
                    lines
                }
            }
        } else {
            match self.lang {
                Lang::En => vec![
                    "↑ ↓  select".to_string(),
                    "Enter  choose".to_string(),
                    "L  language".to_string(),
                    "Esc  back to menu".to_string(),
                ],
                Lang::Fr => vec![
                    "↑ ↓  choisir".to_string(),
                    "Entrée  valider".to_string(),
                    "L  langue".to_string(),
                    "Échap  retour menu".to_string(),
                ],
            }
        };
        // Fenêtre défilante sur `entries` : sans ça, une bibliothèque plus
        // grande que l'espace disponible dépassait silencieusement en bas
        // du panneau (les entrées hors champ n'étaient jamais dessinées,
        // sans indication ni moyen d'y accéder) — signalé, corrigé.
        let available_rows = (inner.height as usize).saturating_sub(hint_lines.len());
        let offset = list_scroll_offset(self.selected, entries.len(), available_rows);
        let mut y = inner.y;
        for (i, label) in entries.iter().enumerate().skip(offset).take(available_rows) {
            let is_selected = i == self.selected;
            let (prefix, style) = if is_selected {
                ("> ", Style::default().fg(Color::Black).bg(Color::White))
            } else if i == 0 {
                ("  ", Style::default().fg(Color::LightGreen))
            } else {
                ("  ", Style::default().fg(Color::White))
            };
            write_line(buf, inner, y, &format!("{prefix}{label}"), style);
            y += 1;
        }

        let start_y = (inner.y + inner.height).saturating_sub(hint_lines.len() as u16);
        for (i, line) in hint_lines.iter().enumerate() {
            write_line(buf, inner, start_y + i as u16, line, Style::default().fg(Color::DarkGray));
        }
    }
}

/// Décalage de défilement pour garder `selected` visible dans une fenêtre
/// de `available` lignes parmi `total` entrées (voir `HolePickerView`).
/// Recalculé à chaque rendu à partir de la seule sélection courante — pas
/// besoin d'état persistant entre les frames pour un défilement stable :
/// tant que `selected` tient dans les `available` premières entrées, la
/// fenêtre reste sur `0` ; au-delà, elle avance juste assez pour révéler
/// `selected` (comme dernière ligne visible), sans jamais dépasser
/// `total - available` (pas de zone vide en bas une fois tout défilé).
pub(crate) fn list_scroll_offset(selected: usize, total: usize, available: usize) -> usize {
    if available == 0 || total <= available {
        return 0;
    }
    let max_offset = total - available;
    if selected < available {
        0
    } else {
        (selected + 1 - available).min(max_offset)
    }
}

/// Aperçu du trou actuellement sélectionné dans `HolePickerView`, affiché à
/// côté de la liste (voir `pick_hole_to_build` dans `main.rs`) — recalculé
/// à chaque case sélectionnée. Parser un seul petit fichier `.course` à
/// chaque frame (repos de la boucle ~200ms) est négligeable, c'est la même
/// opération que celle déjà faite à l'ouverture réelle d'un fichier.
pub struct HolePreviewView<'a> {
    pub lang: Lang,
    /// `None` : "+ Nouveau trou" sélectionné, rien à prévisualiser.
    /// `Some(Err(message))` : le fichier n'a pas pu être lu ou parsé.
    /// `Some(Ok(hole))` : trou chargé avec succès, à afficher.
    pub preview: Option<&'a Result<Hole, String>>,
}

impl<'a> Widget for HolePreviewView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title = match self.lang {
            Lang::En => "Preview",
            Lang::Fr => "Aperçu",
        };
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        block.render(area, buf);

        match self.preview {
            None => {
                let msg = match self.lang {
                    Lang::En => "No preview — starts from a blank grid.",
                    Lang::Fr => "Aucun aperçu — grille vierge au départ.",
                };
                write_line(buf, inner, inner.y, msg, Style::default().fg(Color::DarkGray));
            }
            Some(Err(err)) => {
                let msg = match self.lang {
                    Lang::En => format!("Couldn't load preview: {err}"),
                    Lang::Fr => format!("Aperçu impossible : {err}"),
                };
                write_line(buf, inner, inner.y, &msg, Style::default().fg(Color::LightRed));
            }
            Some(Ok(hole)) => {
                let tiles = hole.local_tiles();
                let h = tiles.len();
                let w = tiles.first().map(|r| r.len()).unwrap_or(0);
                let info_line = format!("{}  ·  Par {}  ·  {w}x{h}", hole.meta.name, hole.meta.par);
                write_line(buf, inner, inner.y, &info_line, Style::default().fg(Color::White));

                if inner.height > 2 {
                    let map_area = Rect {
                        x: inner.x,
                        y: inner.y + 2,
                        width: inner.width,
                        height: inner.height - 2,
                    };
                    BuilderView {
                        tiles: &tiles,
                        cursor: None,
                        viewport_w: map_area.width as usize,
                        viewport_h: map_area.height as usize,
                    }
                    .render(map_area, buf);
                }
            }
        }
    }
}

/// Légende de terrain sous forme de lignes prêtes pour un panneau
/// (`panel`/`panel_bottom_aligned` dans `sidebar.rs`) — une entrée par
/// ligne (plus lisible qu'en tenir deux par ligne, repéré après un
/// signalement) plutôt que toute la légende sur une seule ligne de ~110
/// caractères. Chaque entrée reprend la couleur exacte de son terrain sur
/// la carte (`terrain_style` dans `render.rs`) plutôt qu'une seule couleur
/// uniforme, pour que la légende serve aussi de repère visuel cohérent
/// avec ce que le joueur dessine.
fn terrain_legend_lines(lang: Lang) -> Vec<Line<'static>> {
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
        (TerrainKind::OutOfBounds, "out of bounds", "hors-limites"),
    ];
    let separator = Style::default().fg(Color::DarkGray);
    let mut lines: Vec<Line<'static>> = ITEMS
        .iter()
        .map(|(kind, label_en, label_fr)| {
            let (_, color) = terrain_style(*kind);
            let key = kind.to_char();
            // Une seule ligne par terrain maintenant (voir plus haut) : la
            // touche espace se lit mieux écrite en toutes lettres ("Space")
            // qu'entre parenthèses collée à un "=", et le libellé complet
            // "out of bounds" plutôt que l'abréviation "OOB" évite d'avoir
            // à deviner ce qu'elle veut dire.
            let key_display = match (key, lang) {
                (' ', Lang::En) => "Space".to_string(),
                (' ', Lang::Fr) => "Espace".to_string(),
                (c, _) => c.to_string(),
            };
            let label = if lang == Lang::En { label_en } else { label_fr };
            // `.` et `~` sont les deux seuls caractères de terrain à exiger
            // une combinaison (Maj/AltGr) sur au moins un des claviers
            // US/UK/FR — le builder accepte donc des alias de frappe pour
            // eux (voir `terrain_from_builder_key` dans `main.rs`), affichés
            // ici entre parenthèses pour rester découvrables. La
            // translation ne touche que la frappe : le fichier `.course`
            // garde toujours le même encodage canonique (`.`, `~`...) quelle
            // que soit la touche pressée.
            let alias_suffix = match kind {
                TerrainKind::Fairway => " (F/;)",
                TerrainKind::Water => " (W/é)",
                _ => "",
            };
            Line::styled(
                format!("{key_display} = {label}{alias_suffix}"),
                Style::default().fg(color).add_modifier(Modifier::BOLD),
            )
        })
        .collect();
    lines.push(Line::styled(
        match lang {
            Lang::En => "letters case-insensitive",
            Lang::Fr => "lettres insensibles casse",
        },
        separator,
    ));
    lines
}

/// Nombre de lignes/colonnes restantes avant que l'avancée automatique de la
/// frappe (voir `BuilderState::advance_cursor`) n'atteigne la dernière case
/// et s'arrête net — sert à colorer l'indicateur de position en
/// avertissement un peu avant d'y arriver, pas seulement une fois dessus.
const NEAR_END_THRESHOLD: usize = 3;

const HOLE_ACCENT: Color = Color::LightGreen;
const POSITION_ACCENT: Color = Color::Cyan;
const LEGEND_ACCENT: Color = Color::Yellow;
const CONTROLS_ACCENT: Color = Color::DarkGray;

fn bold(color: Color) -> Style {
    Style::default().fg(color).add_modifier(Modifier::BOLD)
}

/// Colonne de gauche pendant le dessin d'un trou (voir `run_builder` dans
/// `main.rs`) — même esprit que la sidebar du jeu (`tui::SidebarState`) :
/// plusieurs petits panneaux empilés plutôt qu'un seul bandeau, la carte
/// occupant la colonne de droite. Quatre panneaux : "Hole" (nom/par/
/// orientation), "Position" (x/y + progression dans le sens de l'avancée
/// automatique), "Legend" (légende des terrains, deux entrées par ligne
/// pour tenir dans la colonne — voir `terrain_legend_lines`), "Controls"
/// (aide clavier ou saisie en cours, alignée en bas comme le panneau
/// Contrôles du jeu).
pub struct BuilderSidebarView<'a> {
    pub lang: Lang,
    pub name: &'a str,
    pub par: u8,
    pub orientation: BuilderOrientation,
    pub cursor: Pos,
    pub grid_width: usize,
    pub grid_height: usize,
    pub mode: BuilderMode,
    pub text_input: &'a str,
    /// Nom de fichier (sans extension) en attente de confirmation
    /// d'écrasement, affiché dans `BuilderMode::ConfirmOverwrite`.
    pub pending_save_name: Option<&'a str>,
    pub message: Option<&'a str>,
    pub quit_confirm: bool,
    /// Deuxième pression sur `Échap` en attente (retour au menu) — distinct
    /// de `quit_confirm` (quitter l'application entière via `qq`).
    pub exit_confirm: bool,
}

impl<'a> Widget for BuilderSidebarView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let chunks = Layout::default()
            .direction(LayoutDirection::Vertical)
            .constraints([
                Constraint::Length(5),
                Constraint::Length(4),
                Constraint::Length(13),
                Constraint::Min(0),
            ])
            .split(area);

        let hole_title = match self.lang {
            Lang::En => "Hole",
            Lang::Fr => "Trou",
        };
        // Nom affiché avec un espace réservé tant qu'il est vide — un trou
        // sans nom est un état normal (voir `BuilderState::new`) : le nom
        // du fichier tapé à la sauvegarde (`S`) sera alors transféré ici
        // automatiquement, plutôt que de laisser un nom générique trompeur.
        let display_name: &str = if self.name.trim().is_empty() {
            match self.lang {
                Lang::En => "(unnamed)",
                Lang::Fr => "(sans nom)",
            }
        } else {
            self.name
        };
        let hole_lines = vec![
            Line::styled(
                match self.lang {
                    Lang::En => format!("Name: {display_name}"),
                    Lang::Fr => format!("Nom : {display_name}"),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Par: {}", self.par),
                    Lang::Fr => format!("Par : {}", self.par),
                },
                Style::default().fg(Color::White),
            ),
            Line::styled(
                match self.lang {
                    Lang::En => format!("Orientation: {}", self.orientation.label(self.lang)),
                    Lang::Fr => format!("Orientation : {}", self.orientation.label(self.lang)),
                },
                Style::default().fg(Color::White),
            ),
        ];
        panel(chunks[0], buf, hole_title, HOLE_ACCENT, hole_lines);

        // Position courante + progression dans le sens de l'avancée
        // automatique de la frappe (voir `BuilderOrientation`) : c'est
        // cette dimension qui s'arrête net en fin de grille plutôt que de
        // boucler, donc celle qui mérite un avertissement à l'approche de
        // la fin. x/y explicites, 0-indexés — mêmes coordonnées que la
        // grille du fichier `.course` et que les axes imprimés sur
        // `tools/hole_design_canvas.pdf` (origine (0,0) en haut à gauche),
        // pour naviguer directement vers une case repérée sur un plan
        // dessiné à l'avance plutôt que de deviner sa position à l'écran.
        let position_title = match self.lang {
            Lang::En => "Position",
            Lang::Fr => "Position",
        };
        let (progress_line, remaining) = match self.orientation {
            BuilderOrientation::Horizontal => {
                let remaining = self.grid_height - 1 - self.cursor.y;
                let line = match self.lang {
                    Lang::En => format!("Row {}/{}", self.cursor.y + 1, self.grid_height),
                    Lang::Fr => format!("Ligne {}/{}", self.cursor.y + 1, self.grid_height),
                };
                (line, remaining)
            }
            BuilderOrientation::Vertical => {
                let remaining = self.grid_width - 1 - self.cursor.x;
                let line = match self.lang {
                    Lang::En => format!("Column {}/{}", self.cursor.x + 1, self.grid_width),
                    Lang::Fr => format!("Colonne {}/{}", self.cursor.x + 1, self.grid_width),
                };
                (line, remaining)
            }
        };
        let progress_style = if remaining == 0 {
            bold(Color::LightRed)
        } else if remaining <= NEAR_END_THRESHOLD {
            bold(Color::LightYellow)
        } else {
            Style::default().fg(Color::White)
        };
        let progress_line = if remaining == 0 {
            match self.lang {
                Lang::En => format!("{progress_line} (last!)"),
                Lang::Fr => format!("{progress_line} (dernière !)"),
            }
        } else {
            progress_line
        };
        let position_lines = vec![
            Line::styled(
                format!("x={} y={}", self.cursor.x, self.cursor.y),
                Style::default().fg(Color::White),
            ),
            Line::styled(progress_line, progress_style),
        ];
        panel(chunks[1], buf, position_title, POSITION_ACCENT, position_lines);

        let legend_title = match self.lang {
            Lang::En => "Legend",
            Lang::Fr => "Légende",
        };
        panel(chunks[2], buf, legend_title, LEGEND_ACCENT, terrain_legend_lines(self.lang));

        let controls_title = match self.lang {
            Lang::En => "Controls",
            Lang::Fr => "Contrôles",
        };
        let mut controls_lines: Vec<Line<'static>> = Vec::new();
        if let Some(message) = self.message {
            for line in wrap_text(message, 24) {
                controls_lines.push(Line::styled(line, bold(Color::LightRed)));
            }
        }
        if self.quit_confirm {
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "Press q again",
                    Lang::Fr => "Appuyez encore",
                },
                bold(Color::Red),
            ));
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "to quit (unsaved)",
                    Lang::Fr => "sur q pour quitter",
                },
                bold(Color::Red),
            ));
        } else if self.exit_confirm {
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "Esc again: discard",
                    Lang::Fr => "Échap : abandonner",
                },
                bold(Color::LightYellow),
            ));
            controls_lines.push(Line::styled(
                match self.lang {
                    Lang::En => "S: save first",
                    Lang::Fr => "S : sauver d'abord",
                },
                bold(Color::LightYellow),
            ));
        } else {
            match self.mode {
                BuilderMode::Drawing => {}
                BuilderMode::EditingName => {
                    controls_lines.push(Line::styled(
                        match self.lang {
                            Lang::En => "New name:",
                            Lang::Fr => "Nouveau nom :",
                        },
                        Style::default().fg(Color::Gray),
                    ));
                    controls_lines.push(Line::styled(
                        format!("{}_", self.text_input),
                        Style::default().fg(Color::White),
                    ));
                }
                BuilderMode::EnteringSaveName => {
                    controls_lines.push(Line::styled(
                        match self.lang {
                            Lang::En => "File name:",
                            Lang::Fr => "Nom de fichier :",
                        },
                        Style::default().fg(Color::Gray),
                    ));
                    controls_lines.push(Line::styled(
                        format!("{}_", self.text_input),
                        Style::default().fg(Color::White),
                    ));
                }
                BuilderMode::ConfirmOverwrite => {
                    let name = self.pending_save_name.unwrap_or("");
                    controls_lines.push(Line::styled(
                        format!("'{name}'"),
                        bold(Color::White),
                    ));
                    controls_lines.push(Line::styled(
                        match self.lang {
                            Lang::En => "already exists:",
                            Lang::Fr => "existe déjà :",
                        },
                        Style::default().fg(Color::White),
                    ));
                }
                BuilderMode::FillingBackground => {
                    controls_lines.push(Line::styled(
                        match self.lang {
                            Lang::En => "Fill with:",
                            Lang::Fr => "Combler avec :",
                        },
                        Style::default().fg(Color::Gray),
                    ));
                }
            }
            let base: &str = match self.mode {
                BuilderMode::Drawing => match self.lang {
                    Lang::En => {
                        "letters  paint\n↑↓←→  move\nU  undo\nR  rotate\nC  fill\nN  rename\n\
                         S  save\nEsc Esc  menu\nqq  quit"
                    }
                    Lang::Fr => {
                        "lettres  peindre\n↑↓←→  déplacer\nU  annuler\nR  pivoter\nC  combler\n\
                         N  renommer\nS  sauver\nÉchap Échap  menu\nqq  quitter"
                    }
                },
                BuilderMode::EditingName => match self.lang {
                    Lang::En => "Enter  confirm\nEsc  cancel",
                    Lang::Fr => "Entrée  valider\nÉchap  annuler",
                },
                BuilderMode::EnteringSaveName => match self.lang {
                    Lang::En => "(no extension)\nEnter  confirm\nEsc  cancel",
                    Lang::Fr => "(sans extension)\nEntrée  valider\nÉchap  annuler",
                },
                BuilderMode::ConfirmOverwrite => match self.lang {
                    Lang::En => "Enter/Y  overwrite\nEsc/N  rename",
                    Lang::Fr => "Entrée/Y  écraser\nÉchap/N  renommer",
                },
                BuilderMode::FillingBackground => match self.lang {
                    Lang::En => "letters  choose\nEsc  cancel",
                    Lang::Fr => "lettres  choisir\nÉchap  annuler",
                },
            };
            controls_lines.extend(
                base.lines()
                    .map(|l| Line::styled(l.to_string(), Style::default().fg(Color::Gray))),
            );
        }
        panel_bottom_aligned(chunks[3], buf, controls_title, CONTROLS_ACCENT, controls_lines);
    }
}

/// Widget affichant la grille en cours d'édition, avec le curseur surligné
/// à la place de la balle/du trou "réels" du jeu (rien n'est encore validé
/// tant que le fichier n'est pas sauvegardé) — calqué sur la boucle
/// cellule-par-cellule de `CourseView` (`render.rs`), mais sans zoom ni
/// aucune des surimpressions d'aperçu de coup, qui n'ont pas de sens ici.
pub struct BuilderView<'a> {
    pub tiles: &'a [Vec<TerrainKind>],
    /// `None` pour un aperçu en lecture seule (pas de case à surligner,
    /// voir `HolePreviewView`) — le centrage retombe alors sur le milieu
    /// de la grille plutôt que sur une position de curseur.
    pub cursor: Option<Pos>,
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

        let center = self.cursor.unwrap_or(Pos { x: grid_w / 2, y: grid_h / 2 });

        let view_w = (inner.width as usize).min(self.viewport_w).max(1);
        let view_h = (inner.height as usize).min(self.viewport_h).max(1);
        let viewport = Viewport { width: view_w, height: view_h };
        let (ox, oy) = viewport.top_left(center, grid_w, grid_h);

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
                let is_cursor = self.cursor == Some(Pos { x: gx, y: gy });

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wrap_text_keeps_short_lines_untouched() {
        assert_eq!(wrap_text("short line", 24), vec!["short line".to_string()]);
    }

    #[test]
    fn wrap_text_breaks_at_word_boundaries_within_width() {
        let wrapped = wrap_text("not yet part of any course", 15);
        for line in &wrapped {
            assert!(line.chars().count() <= 15, "line too long: {line:?}");
        }
        assert_eq!(wrapped.join(" "), "not yet part of any course");
    }

    #[test]
    fn wrap_text_lets_a_single_long_word_overflow_the_width() {
        // Un seul "mot" sans espace (ex. un chemin de fichier) ne peut pas
        // être coupé plus finement — reste sur sa propre ligne même si ça
        // dépasse `width` (mieux qu'une coupure au milieu d'un chemin).
        let wrapped = wrap_text("courses/_library/a_very_long_name.course", 15);
        assert_eq!(wrapped, vec!["courses/_library/a_very_long_name.course".to_string()]);
    }

    #[test]
    fn wrap_text_returns_empty_for_empty_input() {
        assert!(wrap_text("", 24).is_empty());
    }

    #[test]
    fn list_scroll_offset_stays_at_zero_while_selection_fits_on_screen() {
        assert_eq!(list_scroll_offset(0, 20, 5), 0);
        assert_eq!(list_scroll_offset(4, 20, 5), 0);
    }

    #[test]
    fn list_scroll_offset_advances_just_enough_to_reveal_the_selection() {
        // 20 entrées, 5 visibles à la fois : sélectionner la 6e (index 5)
        // doit faire défiler d'exactement une ligne pour qu'elle devienne
        // la dernière visible.
        assert_eq!(list_scroll_offset(5, 20, 5), 1);
        assert_eq!(list_scroll_offset(10, 20, 5), 6);
    }

    #[test]
    fn list_scroll_offset_never_scrolls_past_the_last_full_page() {
        // Sélectionner la toute dernière entrée ne doit jamais laisser de
        // zone vide en bas : le décalage plafonne à `total - available`.
        assert_eq!(list_scroll_offset(19, 20, 5), 15);
    }

    #[test]
    fn list_scroll_offset_is_always_zero_when_everything_fits() {
        assert_eq!(list_scroll_offset(2, 4, 10), 0);
    }
}
