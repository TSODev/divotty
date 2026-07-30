use crate::core::{Hole, Pos, ShotPreview, TerrainKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{Block, Borders, Widget},
};

/// Couleur + caractère affichés pour un type de terrain. Le fairway reste
/// volontairement terne (couleur discrète) pour ne pas rivaliser avec les
/// éléments de visée (guide/halo), rendus plus lumineux — voir `render()`.
fn terrain_style(kind: TerrainKind) -> (char, Color) {
    match kind {
        TerrainKind::Tee => ('D', Color::LightCyan),
        TerrainKind::Fairway => ('.', Color::Rgb(0, 90, 0)),
        TerrainKind::Rough => ('"', Color::LightGreen),
        TerrainKind::Bunker => ('°', Color::Yellow),
        TerrainKind::Water => ('~', Color::Blue),
        TerrainKind::Tree => ('♣', Color::Rgb(0, 100, 0)),
        TerrainKind::Green => ('O', Color::Rgb(0, 220, 0)),
        TerrainKind::Hole => ('⛳', Color::White),
        TerrainKind::OutOfBounds => (' ', Color::DarkGray),
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

impl<'a> Widget for CourseView<'a> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        block.render(area, buf);

        let grid_h = self.hole.tiles.len();
        let grid_w = self.hole.tiles.first().map(|r| r.len()).unwrap_or(0);
        let (ox, oy) = self.viewport.top_left(self.ball, grid_w, grid_h);

        // Si la grille tient entièrement dans la zone disponible (panneau
        // plus grand que la carte), on la centre plutôt que de la laisser
        // collée en haut à gauche du cadre.
        let content_w = (grid_w as u16).min(inner.width);
        let content_h = (grid_h as u16).min(inner.height);
        let margin_x = (inner.width - content_w) / 2;
        let margin_y = (inner.height - content_h) / 2;

        let guide_cells = self
            .preview
            .map(|p| sample_line(self.ball, p.max_landing))
            .unwrap_or_default();

        for row in 0..content_h.min(self.viewport.height as u16) {
            for col in 0..content_w.min(self.viewport.width as u16) {
                let gx = ox + col as usize;
                let gy = oy + row as usize;
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

                if gx == self.ball.x && gy == self.ball.y {
                    ch = '●';
                    color = Color::Red;
                    modifier = Modifier::empty();
                }

                let x = inner.x + margin_x + col;
                let y = inner.y + margin_y + row;
                if x < inner.x + inner.width && y < inner.y + inner.height {
                    buf.get_mut(x, y)
                        .set_char(ch)
                        .set_style(Style::default().fg(color).add_modifier(modifier));
                }
            }
        }
    }
}
