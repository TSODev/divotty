use crate::core::{Direction, Hole, Pos, ShotPreview, TerrainKind};
use crate::tui::format::{compass_arrow, compass_sector};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Borders, Widget},
};

/// Couleur + caractère affichés pour un type de terrain. Le fairway reste
/// volontairement terne (couleur discrète) pour ne pas rivaliser avec les
/// éléments de visée (guide/halo), rendus plus lumineux — voir `render()`.
///
/// Les glyphes doivent rester des caractères "étroits" (largeur 1 colonne).
/// Le rendu écrit case par case via `buf.get_mut(x,y).set_char(...)` sans
/// jamais tenir compte de la largeur Unicode — un caractère à présentation
/// emoji par défaut (ex: `⛳`, U+26F3) s'affiche sur 2 colonnes dans la
/// plupart des terminaux et désaligne tout ce qui suit sur la ligne,
/// surtout visible avec le zoom qui le répète plusieurs fois d'affilée.
fn terrain_style(kind: TerrainKind) -> (char, Color) {
    match kind {
        TerrainKind::Tee => ('D', Color::LightCyan),
        TerrainKind::Fairway => ('.', Color::Rgb(0, 90, 0)),
        TerrainKind::Rough => ('"', Color::LightGreen),
        TerrainKind::Bunker => ('°', Color::Yellow),
        TerrainKind::Water => ('~', Color::Blue),
        TerrainKind::Tree => ('♣', Color::Rgb(0, 100, 0)),
        TerrainKind::Green => ('O', Color::Rgb(0, 220, 0)),
        TerrainKind::Hole => ('⚑', Color::White),
        TerrainKind::OutOfBounds => (' ', Color::DarkGray),
        TerrainKind::PenaltyZone => ('X', Color::Red),
    }
}

/// Viewport qui suit la balle : la grille (25x50) est plus grande que la
/// plupart des terminaux, donc on n'affiche qu'une fenêtre centrée sur la
/// position courante de la balle plutôt que toute la carte à la fois.
pub struct Viewport {
    pub width: usize,
    pub height: usize,
}

impl Viewport {
    /// Calcule le coin haut-gauche de la fenêtre pour centrer `center` dans
    /// une grille de dimensions `grid_w` x `grid_h`.
    pub fn top_left(&self, center: Pos, grid_w: usize, grid_h: usize) -> (usize, usize) {
        let half_w = self.width / 2;
        let half_h = self.height / 2;
        let x = center.x.saturating_sub(half_w).min(grid_w.saturating_sub(self.width));
        let y = center.y.saturating_sub(half_h).min(grid_h.saturating_sub(self.height));
        (x, y)
    }
}

/// Widget ratatui affichant la portion visible du trou courant, avec la
/// position de la balle superposée.
pub struct CourseView<'a> {
    pub hole: &'a Hole,
    pub ball: Pos,
    pub viewport: Viewport,
    /// Aperçu de portée/dispersion pour le coup en préparation, affiché en
    /// surimpression tant que le joueur n'a pas joué. `None` si pas de coup
    /// en cours de visée (ex: entre deux trous).
    pub preview: Option<ShotPreview>,
    /// Zoom activé par le joueur (touche dédiée) — pas automatique.
    pub zoomed: bool,
}

/// Distance euclidienne discrète entre deux cases de la grille.
fn cell_distance(a: Pos, b: Pos) -> f32 {
    let dx = a.x as f32 - b.x as f32;
    let dy = a.y as f32 - b.y as f32;
    (dx * dx + dy * dy).sqrt()
}

/// Points échantillonnés sur le segment `from`→`to`, une case tous les
/// (grosso modo) 1 cran, pour tracer un guide de trajectoire.
fn sample_line(from: Pos, to: Pos) -> Vec<Pos> {
    let dx = to.x as f32 - from.x as f32;
    let dy = to.y as f32 - from.y as f32;
    let length = (dx * dx + dy * dy).sqrt();
    let steps = length.ceil().max(1.0) as usize;
    (1..steps)
        .map(|step| {
            let t = step as f32 / steps as f32;
            Pos {
                x: (from.x as f32 + dx * t).round() as usize,
                y: (from.y as f32 + dy * t).round() as usize,
            }
        })
        .collect()
}

/// Facteur de grossissement quand le zoom est activé (touche dédiée) :
/// chaque case de la grille est alors dessinée comme un bloc de
/// `ZOOM_FACTOR`x`ZOOM_FACTOR` caractères plutôt qu'un seul. Purement
/// visuel — le modèle de position (`Pos`, cases entières) ne change pas.
const ZOOM_FACTOR: usize = 3;

/// Décalage (colonne, ligne) dans le bloc zoomé où placer la flèche de
/// visée : à l'opposé du centre, dans le secteur de boussole de la
/// direction visée (même secteur que `compass_arrow`, pour que le glyphe et
/// sa position restent cohérents). Généralisé à `zoom` plutôt que figé sur
/// la valeur actuelle de `ZOOM_FACTOR`.
fn compass_cell_offset(direction: Direction, zoom: usize) -> (u16, u16) {
    let center = (zoom / 2) as i32;
    let radius = center.max(1);
    let angle = (compass_sector(direction) as f32) * 45.0_f32.to_radians();
    let dx = (angle.cos() * radius as f32).round() as i32;
    let dy = (angle.sin() * radius as f32).round() as i32;
    (
        (center + dx).clamp(0, zoom as i32 - 1) as u16,
        (center + dy).clamp(0, zoom as i32 - 1) as u16,
    )
}

impl<'a> Widget for CourseView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default()
            .border_type(BorderType::Rounded)
            .borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let grid_h = self.hole.tiles.len();
        let grid_w = self.hole.tiles.first().map(|r| r.len()).unwrap_or(0);

        let zoom = if self.zoomed { ZOOM_FACTOR } else { 1 };

        // Fenêtre visible, en cases de grille : la taille physique de la
        // zone (panneau) et celle demandée par l'appelant (`self.viewport`)
        // sont toutes deux divisées par le zoom, puisqu'une case occupe
        // désormais `zoom` caractères dans chaque direction.
        let view_w_cells = ((inner.width as usize) / zoom)
            .min(self.viewport.width / zoom)
            .max(1);
        let view_h_cells = ((inner.height as usize) / zoom)
            .min(self.viewport.height / zoom)
            .max(1);
        let scoped_viewport = Viewport {
            width: view_w_cells,
            height: view_h_cells,
        };
        let (ox, oy) = scoped_viewport.top_left(self.ball, grid_w, grid_h);

        // Si la grille (visible) tient entièrement dans la zone disponible,
        // on la centre plutôt que de la laisser collée en haut à gauche.
        let content_w_cells = grid_w.min(view_w_cells);
        let content_h_cells = grid_h.min(view_h_cells);
        let margin_x = (inner.width - (content_w_cells * zoom) as u16) / 2;
        let margin_y = (inner.height - (content_h_cells * zoom) as u16) / 2;

        let guide_cells = self
            .preview
            .map(|p| sample_line(self.ball, p.max_landing))
            .unwrap_or_default();

        for grow in 0..content_h_cells {
            for gcol in 0..content_w_cells {
                let gx = ox + gcol;
                let gy = oy + grow;
                let pos = Pos { x: gx, y: gy };
                let Some(terrain) = self.hole.terrain_at(pos) else {
                    continue;
                };

                let (mut ch, mut color) = terrain_style(terrain);
                let mut modifier = Modifier::empty();
                // Le tee et le trou sont des repères qu'on ne veut jamais
                // masquer derrière la surimpression de l'aperçu de coup
                // (guide, halo, repère d'atterrissage) — seule leur couleur
                // se teinte pour indiquer un chevauchement.
                let is_landmark = matches!(terrain, TerrainKind::Tee | TerrainKind::Hole);

                if let Some(preview) = self.preview {
                    if cell_distance(pos, preview.expected_landing) <= preview.dispersion_radius {
                        color = Color::LightMagenta;
                        modifier = Modifier::BOLD;
                    }
                    if guide_cells.contains(&pos) {
                        if !is_landmark {
                            ch = '·';
                        }
                        color = Color::White;
                        modifier = Modifier::BOLD;
                    }
                    if pos == preview.expected_landing {
                        if !is_landmark {
                            ch = '✛';
                        }
                        color = Color::LightYellow;
                        modifier = Modifier::BOLD;
                    }
                }

                let is_ball = gx == self.ball.x && gy == self.ball.y;
                let base_x = inner.x + margin_x + (gcol * zoom) as u16;
                let base_y = inner.y + margin_y + (grow * zoom) as u16;

                // En zoom, la balle et le trou n'occupent plus qu'une seule
                // sous-case du bloc `zoom`x`zoom` (au lieu de le remplir
                // entièrement) : sur un putt de 1-2 cases, deux blocs pleins
                // ne laissaient plus rien voir entre les deux. Le reste du
                // bloc montre le terrain réel (sous la balle) ou le vert
                // (autour du trou), et — pour la balle — une flèche de
                // visée dans le secteur de la direction visée, fiable même
                // quand `expected_landing` arrondit à la même case que le
                // départ (voir `ShotPreview::direction`).
                if zoom > 1 && (is_ball || terrain == TerrainKind::Hole) {
                    let (fill_ch, fill_color) = if is_ball {
                        (ch, color) // terrain réel déjà calculé sous la balle
                    } else {
                        terrain_style(TerrainKind::Green) // vert autour du trou
                    };
                    let center = ((zoom / 2) as u16, (zoom / 2) as u16);
                    let arrow = is_ball
                        .then(|| self.preview.map(|p| p.direction))
                        .flatten()
                        .map(|direction| (compass_cell_offset(direction, zoom), compass_arrow(direction)));

                    for dy in 0..zoom as u16 {
                        for dx in 0..zoom as u16 {
                            let x = base_x + dx;
                            let y = base_y + dy;
                            if x >= inner.x + inner.width || y >= inner.y + inner.height {
                                continue;
                            }
                            if (dx, dy) == center {
                                let (marker, marker_color) =
                                    if is_ball { ('●', Color::Red) } else { (ch, color) };
                                buf.get_mut(x, y).set_char(marker).set_style(Style::default().fg(marker_color));
                            } else if let Some((offset, glyph)) = arrow {
                                if (dx, dy) == offset {
                                    buf.get_mut(x, y)
                                        .set_char(glyph.chars().next().unwrap())
                                        .set_style(Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD));
                                    continue;
                                }
                                buf.get_mut(x, y).set_char(fill_ch).set_style(Style::default().fg(fill_color));
                            } else {
                                buf.get_mut(x, y).set_char(fill_ch).set_style(Style::default().fg(fill_color));
                            }
                        }
                    }
                    continue;
                }

                let style = Style::default().fg(color).add_modifier(modifier);
                for dy in 0..zoom as u16 {
                    for dx in 0..zoom as u16 {
                        let x = base_x + dx;
                        let y = base_y + dy;
                        if x < inner.x + inner.width && y < inner.y + inner.height {
                            buf.get_mut(x, y).set_char(ch).set_style(style);
                        }
                    }
                }
            }
        }
    }
}
