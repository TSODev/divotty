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
pub(crate) fn terrain_style(kind: TerrainKind) -> (char, Color) {
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
    /// Historique des points d'arrêt de la balle sur ce trou (départ inclus,
    /// position finale incluse) — affiché à la place de `preview` une fois
    /// le trou terminé, en rappel visuel du parcours : une balle rouge à
    /// chaque arrêt intermédiaire, un pointillé de balles jaunes reliant
    /// les points consécutifs. `None` tant que le trou n'est pas fini.
    pub path: Option<&'a [Pos]>,
    /// Niveau de zoom choisi par le joueur (touche dédiée) — pas automatique.
    pub zoom: ZoomLevel,
}

/// Niveau de zoom de la carte, cycle avec une touche dédiée : normal (une
/// case = un caractère) → avant (grossi, voir `ZOOM_FACTOR`, utile pour un
/// putt de 1-2 cases) → arrière (vue d'ensemble réduite pour voir tout le
/// trou d'un coup, voir `render_overview`) → retour à normal.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ZoomLevel {
    Out,
    #[default]
    Normal,
    In,
}

impl ZoomLevel {
    pub fn next(self) -> Self {
        match self {
            ZoomLevel::Normal => ZoomLevel::In,
            ZoomLevel::In => ZoomLevel::Out,
            ZoomLevel::Out => ZoomLevel::Normal,
        }
    }
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

/// Case (entière) où placer une flèche indiquant la direction de `to`
/// (le trou) sur le bord de la fenêtre visible `[x_min,x_max] x
/// [y_min,y_max]`, en projetant le rayon depuis `from` (la balle) jusqu'à
/// la première bordure rencontrée — suit la direction réelle plutôt qu'un
/// simple angle collé à un coin : un trou légèrement au sud-est finit près
/// du bord droit, pas pile au coin.
fn edge_indicator_pos(from: Pos, to: Pos, x_min: usize, x_max: usize, y_min: usize, y_max: usize) -> Pos {
    let (fx, fy) = (from.x as f32, from.y as f32);
    let (tx, ty) = (to.x as f32, to.y as f32);
    let (dx, dy) = (tx - fx, ty - fy);

    let t_x = if dx > 0.0 {
        (x_max as f32 - fx) / dx
    } else if dx < 0.0 {
        (x_min as f32 - fx) / dx
    } else {
        f32::INFINITY
    };
    let t_y = if dy > 0.0 {
        (y_max as f32 - fy) / dy
    } else if dy < 0.0 {
        (y_min as f32 - fy) / dy
    } else {
        f32::INFINITY
    };
    let t = t_x.min(t_y).max(0.0);

    Pos {
        x: (fx + dx * t).round().clamp(x_min as f32, x_max as f32) as usize,
        y: (fy + dy * t).round().clamp(y_min as f32, y_max as f32) as usize,
    }
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

/// Trois décalages (dx, dy) formant un petit segment dans le bloc zoomé,
/// orienté selon la direction du coup (horizontal, vertical ou diagonal
/// selon le secteur de boussole de `direction`) — remplace le simple point
/// centré pour un point de guide de trajectoire, afin qu'il se lise comme
/// un tronçon de ligne qui *traverse* la case plutôt qu'un point isolé.
/// Le point du milieu coïncide toujours avec le centre du bloc.
fn guide_segment_offsets(direction: Direction, zoom: usize) -> [(u16, u16); 3] {
    let center = (zoom / 2) as u16;
    let last = zoom as u16 - 1;
    match compass_sector(direction) {
        0 | 4 => [(0, center), (center, center), (last, center)], // ← / → : horizontal
        2 | 6 => [(center, 0), (center, center), (center, last)], // ↑ / ↓ : vertical
        1 | 5 => [(0, 0), (center, center), (last, last)],        // ↖ / ↘ : diagonale "\"
        _ => [(last, 0), (center, center), (0, last)],            // ↗ / ↙ : diagonale "/"
    }
}

/// Caractère de cadre (box-drawing) pour la sous-case (dx, dy) d'un bloc
/// zoomé de taille `zoom`, entourant le trou d'un cadre plutôt que de la
/// couleur du terrain environnant — fait ressortir la cible quel que soit
/// le terrain autour (vert, rough...). `None` pour le centre, où le
/// drapeau reste affiché.
fn hole_frame_glyph(dx: u16, dy: u16, zoom: usize) -> Option<char> {
    let last = zoom as u16 - 1;
    let is_left = dx == 0;
    let is_right = dx == last;
    let is_top = dy == 0;
    let is_bottom = dy == last;
    match (is_left, is_right, is_top, is_bottom) {
        // Coins arrondis (mêmes glyphes que `BorderType::Rounded` utilisé
        // partout ailleurs dans l'interface), pas des coins carrés.
        (true, _, true, _) => Some('╭'),
        (_, true, true, _) => Some('╮'),
        (true, _, _, true) => Some('╰'),
        (_, true, _, true) => Some('╯'),
        _ if is_top || is_bottom => Some('─'),
        _ if is_left || is_right => Some('│'),
        _ => None,
    }
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

        if self.zoom == ZoomLevel::Out {
            render_overview(self.hole, self.ball, inner, buf);
            return;
        }

        let zoom = if self.zoom == ZoomLevel::In { ZOOM_FACTOR } else { 1 };

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

        // Le trou est-il hors de la fenêtre visible ? Si oui (et seulement
        // en zoom normal, voir `ROADMAP.md`), une flèche de boussole prend
        // sa place sur la case de bord la plus proche de sa direction
        // réelle — calculée par projection du rayon balle→trou jusqu'à la
        // première bordure rencontrée (`edge_indicator_pos`), pas un simple
        // angle collé à un coin.
        let hole_pos = self.hole.hole_pos;
        let hole_visible = hole_pos.x >= ox
            && hole_pos.x < ox + content_w_cells
            && hole_pos.y >= oy
            && hole_pos.y < oy + content_h_cells;
        let edge_indicator: Option<(Pos, &'static str)> = (zoom == 1 && !hole_visible).then(|| {
            let pos = edge_indicator_pos(
                self.ball,
                hole_pos,
                ox,
                ox + content_w_cells.saturating_sub(1),
                oy,
                oy + content_h_cells.saturating_sub(1),
            );
            (pos, compass_arrow(Direction::towards(self.ball, hole_pos)))
        });

        let guide_cells = self
            .preview
            .map(|p| sample_line(self.ball, p.max_landing))
            .unwrap_or_default();

        // Arrêts intermédiaires (ni le départ ni l'atterrissage final, déjà
        // repérés par leurs propres glyphes) et cases interpolées entre
        // arrêts consécutifs, pour le rappel de parcours affiché une fois
        // le trou terminé (voir `CourseView::path`).
        let (path_stops, path_dots): (Vec<Pos>, Vec<Pos>) = match self.path {
            Some(positions) if positions.len() >= 2 => {
                let stops = positions[1..positions.len() - 1].to_vec();
                let dots = positions.windows(2).flat_map(|pair| sample_line(pair[0], pair[1])).collect();
                (stops, dots)
            }
            _ => (Vec::new(), Vec::new()),
        };

        for grow in 0..content_h_cells {
            for gcol in 0..content_w_cells {
                let gx = ox + gcol;
                let gy = oy + grow;
                let pos = Pos { x: gx, y: gy };
                let Some(terrain) = self.hole.terrain_at(pos) else {
                    continue;
                };

                let (base_ch, base_color) = terrain_style(terrain);
                let mut ch = base_ch;
                let mut color = base_color;
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

                if self.path.is_some() {
                    if path_dots.contains(&pos) {
                        if !is_landmark {
                            ch = '●';
                        }
                        color = Color::LightYellow;
                        modifier = Modifier::BOLD;
                    }
                    if path_stops.contains(&pos) {
                        if !is_landmark {
                            ch = '●';
                        }
                        color = Color::Red;
                        modifier = Modifier::BOLD;
                    }
                }

                if let Some((indicator_pos, glyph)) = edge_indicator {
                    if pos == indicator_pos {
                        ch = glyph.chars().next().unwrap();
                        color = Color::White;
                        modifier = Modifier::BOLD;
                    }
                }

                let is_ball = gx == self.ball.x && gy == self.ball.y;
                if is_ball {
                    ch = '●';
                    color = Color::Red;
                    modifier = Modifier::empty();
                }

                let base_x = inner.x + margin_x + (gcol * zoom) as u16;
                let base_y = inner.y + margin_y + (grow * zoom) as u16;

                if zoom == 1 {
                    // Un seul caractère par case, comportement d'origine.
                    if base_x < inner.x + inner.width && base_y < inner.y + inner.height {
                        buf.get_mut(base_x, base_y)
                            .set_char(ch)
                            .set_style(Style::default().fg(color).add_modifier(modifier));
                    }
                    continue;
                }

                // En zoom, le centre du bloc affiche ce qui aurait été
                // affiché sans zoom (`ch`/`color`/`modifier` ci-dessus, y
                // compris la balle et toute surimpression de l'aperçu :
                // guide pointillé, halo de dispersion, repère
                // d'atterrissage) — le reste du bloc montre le terrain réel
                // plutôt que de dupliquer le même marqueur sur les
                // `zoom`x`zoom` caractères. Sur un putt de 1-2 cases, deux
                // blocs pleins ne laissaient plus rien voir entre eux ; même
                // chose pour un guide pointillé qui devenait un bloc plein
                // plutôt qu'un pointillé. Exceptions : la balle garde une
                // flèche de visée dans le secteur visé, le trou un cadre à
                // la place du remplissage de terrain, et un point de guide
                // (pas le halo ni le repère d'atterrissage, qui restent des
                // points isolés) un petit segment de 3 points orienté selon
                // la direction du coup plutôt qu'un point unique — se lit
                // comme un tronçon de trajectoire qui traverse la case.
                let center = ((zoom / 2) as u16, (zoom / 2) as u16);
                let arrow = is_ball
                    .then(|| self.preview.map(|p| p.direction))
                    .flatten()
                    .map(|direction| (compass_cell_offset(direction, zoom), compass_arrow(direction)));
                let is_hole = terrain == TerrainKind::Hole;
                let guide_segment = (!is_ball && !is_hole && ch == '·')
                    .then(|| self.preview.map(|p| p.direction))
                    .flatten()
                    .map(|direction| guide_segment_offsets(direction, zoom));

                for dy in 0..zoom as u16 {
                    for dx in 0..zoom as u16 {
                        let x = base_x + dx;
                        let y = base_y + dy;
                        if x >= inner.x + inner.width || y >= inner.y + inner.height {
                            continue;
                        }
                        if (dx, dy) == center {
                            buf.get_mut(x, y)
                                .set_char(ch)
                                .set_style(Style::default().fg(color).add_modifier(modifier));
                            continue;
                        }
                        if let Some((offset, glyph)) = arrow {
                            if (dx, dy) == offset {
                                buf.get_mut(x, y)
                                    .set_char(glyph.chars().next().unwrap())
                                    .set_style(Style::default().fg(Color::LightCyan).add_modifier(Modifier::BOLD));
                                continue;
                            }
                        }
                        if let Some(offsets) = guide_segment {
                            if offsets.contains(&(dx, dy)) {
                                buf.get_mut(x, y)
                                    .set_char(ch)
                                    .set_style(Style::default().fg(color).add_modifier(modifier));
                                continue;
                            }
                        }
                        if is_hole {
                            if let Some(frame) = hole_frame_glyph(dx, dy, zoom) {
                                buf.get_mut(x, y).set_char(frame).set_style(Style::default().fg(Color::White));
                                continue;
                            }
                        }
                        buf.get_mut(x, y).set_char(base_ch).set_style(Style::default().fg(base_color));
                    }
                }
            }
        }
    }
}

/// Terrain "dominant" d'un bloc de cases `[x0,x1) x [y0,y1)`, pour la vue
/// d'ensemble dézoomée (`render_overview`) : priorité aux repères qui ne
/// doivent jamais disparaître même minoritaires dans un bloc (tee/trou),
/// puis aux dangers (eau/bunker/arbre/pénalité), puis au terrain "normal"
/// (green/rough/fairway), hors-limites en dernier recours. Une simple
/// majorité aurait pu diluer un tee/trou isolé au milieu d'un grand bloc de
/// fairway, le faisant disparaître de la vue d'ensemble.
fn dominant_terrain(hole: &Hole, x0: usize, y0: usize, x1: usize, y1: usize) -> TerrainKind {
    const PRIORITY: [TerrainKind; 10] = [
        TerrainKind::Tee,
        TerrainKind::Hole,
        TerrainKind::Water,
        TerrainKind::Bunker,
        TerrainKind::Tree,
        TerrainKind::PenaltyZone,
        TerrainKind::Green,
        TerrainKind::Rough,
        TerrainKind::Fairway,
        TerrainKind::OutOfBounds,
    ];
    let mut present = [false; PRIORITY.len()];
    for y in y0..y1 {
        for x in x0..x1 {
            if let Some(terrain) = hole.terrain_at(Pos { x, y }) {
                if let Some(idx) = PRIORITY.iter().position(|k| *k == terrain) {
                    present[idx] = true;
                }
            }
        }
    }
    PRIORITY
        .iter()
        .zip(present.iter())
        .find(|(_, found)| **found)
        .map(|(kind, _)| *kind)
        .unwrap_or(TerrainKind::OutOfBounds)
}

/// Vue d'ensemble dézoomée (`ZoomLevel::Out`) : montre tout le trou (canevas
/// complet) réduit pour tenir dans la zone disponible, chaque caractère
/// affiché représentant un bloc de plusieurs cases de la grille réelle
/// plutôt qu'une seule. Le facteur de réduction est calculé pour que tout
/// tienne (le plus petit entier tel que la grille entière rentre dans
/// `inner`), pas une valeur fixe. Simplifiée à dessein par rapport au rendu
/// normal : ni aperçu de coup, ni rappel de trajectoire, seulement le
/// terrain et la balle — cette vue sert à se repérer d'un coup d'œil, pas à
/// viser précisément.
fn render_overview(hole: &Hole, ball: Pos, inner: Rect, buf: &mut Buffer) {
    let grid_h = hole.tiles.len();
    let grid_w = hole.tiles.first().map(|r| r.len()).unwrap_or(0);
    if grid_w == 0 || grid_h == 0 || inner.width == 0 || inner.height == 0 {
        return;
    }

    let factor = ((grid_w as f32 / inner.width as f32).ceil())
        .max((grid_h as f32 / inner.height as f32).ceil())
        .max(1.0) as usize;

    let display_w = (grid_w + factor - 1) / factor;
    let display_h = (grid_h + factor - 1) / factor;
    let margin_x = (inner.width - display_w as u16) / 2;
    let margin_y = (inner.height - display_h as u16) / 2;

    for row in 0..display_h {
        for col in 0..display_w {
            let x0 = col * factor;
            let y0 = row * factor;
            let x1 = (x0 + factor).min(grid_w);
            let y1 = (y0 + factor).min(grid_h);

            let ball_here = ball.x >= x0 && ball.x < x1 && ball.y >= y0 && ball.y < y1;
            let (ch, color) = if ball_here {
                ('●', Color::Red)
            } else {
                terrain_style(dominant_terrain(hole, x0, y0, x1, y1))
            };

            let x = inner.x + margin_x + col as u16;
            let y = inner.y + margin_y + row as u16;
            if x < inner.x + inner.width && y < inner.y + inner.height {
                buf.get_mut(x, y).set_char(ch).set_style(Style::default().fg(color));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::HoleMeta;

    fn test_hole(tiles: Vec<Vec<TerrainKind>>) -> Hole {
        let height = tiles.len();
        let width = tiles.first().map(|r| r.len()).unwrap_or(0);
        Hole {
            meta: HoleMeta { name: "Test".to_string(), par: 4, description: None, width: None, height: None },
            tiles,
            tee: Pos { x: 0, y: 0 },
            hole_pos: Pos { x: width.saturating_sub(1), y: height.saturating_sub(1) },
        }
    }

    #[test]
    fn dominant_terrain_prefers_tee_and_hole_over_everything_else() {
        let hole = test_hole(vec![
            vec![TerrainKind::Fairway, TerrainKind::Fairway, TerrainKind::Tee],
            vec![TerrainKind::Fairway, TerrainKind::Fairway, TerrainKind::Fairway],
        ]);
        assert_eq!(dominant_terrain(&hole, 0, 0, 3, 2), TerrainKind::Tee);
    }

    #[test]
    fn dominant_terrain_prefers_hazards_over_plain_terrain() {
        let hole = test_hole(vec![
            vec![TerrainKind::Fairway, TerrainKind::Fairway, TerrainKind::Fairway],
            vec![TerrainKind::Fairway, TerrainKind::Bunker, TerrainKind::Fairway],
        ]);
        assert_eq!(dominant_terrain(&hole, 0, 0, 3, 2), TerrainKind::Bunker);
    }

    #[test]
    fn dominant_terrain_falls_back_to_out_of_bounds_when_the_block_is_empty_of_anything_else() {
        let hole = test_hole(vec![vec![TerrainKind::OutOfBounds; 2]; 2]);
        assert_eq!(dominant_terrain(&hole, 0, 0, 2, 2), TerrainKind::OutOfBounds);
    }

    #[test]
    fn dominant_terrain_picks_the_majority_terrain_when_no_priority_kind_is_present() {
        let hole = test_hole(vec![
            vec![TerrainKind::Rough, TerrainKind::Rough],
            vec![TerrainKind::Fairway, TerrainKind::Rough],
        ]);
        // Le rough est présent, la présence l'emporte même minoritaire —
        // seule sa présence compte, pas le décompte exact.
        assert_eq!(dominant_terrain(&hole, 0, 0, 2, 2), TerrainKind::Rough);
    }

    #[test]
    fn edge_indicator_pos_projects_towards_the_right_edge_when_target_is_due_east() {
        let from = Pos { x: 10, y: 10 };
        let to = Pos { x: 50, y: 10 };
        let pos = edge_indicator_pos(from, to, 5, 20, 5, 20);
        assert_eq!(pos, Pos { x: 20, y: 10 }, "droit à l'est : doit toucher le bord droit, même hauteur");
    }

    #[test]
    fn edge_indicator_pos_projects_towards_the_bottom_edge_when_target_is_due_south() {
        let from = Pos { x: 10, y: 10 };
        let to = Pos { x: 10, y: 50 };
        let pos = edge_indicator_pos(from, to, 5, 20, 5, 20);
        assert_eq!(pos, Pos { x: 10, y: 20 }, "droit au sud : doit toucher le bord bas, même colonne");
    }

    #[test]
    fn edge_indicator_pos_hits_a_corner_on_an_exact_diagonal() {
        let from = Pos { x: 10, y: 10 };
        let to = Pos { x: 50, y: 50 };
        let pos = edge_indicator_pos(from, to, 5, 20, 5, 20);
        assert_eq!(pos, Pos { x: 20, y: 20 }, "diagonale exacte : doit toucher le coin bas-droit");
    }

    #[test]
    fn edge_indicator_pos_stays_closer_to_the_edge_matching_the_shallower_slope() {
        // Direction majoritairement horizontale (peu de dy pour beaucoup de
        // dx) : doit toucher le bord droit avant le bord bas, avec un y
        // proche mais pas égal à celui du bord bas.
        let from = Pos { x: 10, y: 10 };
        let to = Pos { x: 50, y: 15 };
        let pos = edge_indicator_pos(from, to, 5, 20, 5, 20);
        assert_eq!(pos.x, 20, "doit toucher le bord droit en premier");
        assert!(pos.y > 10 && pos.y < 20, "y doit avoir légèrement bougé sans atteindre le bord bas");
    }

    #[test]
    fn edge_indicator_pos_clamps_within_bounds_when_the_ball_is_already_on_the_edge() {
        let from = Pos { x: 20, y: 10 };
        let to = Pos { x: 50, y: 10 };
        let pos = edge_indicator_pos(from, to, 5, 20, 5, 20);
        assert_eq!(pos, Pos { x: 20, y: 10 }, "déjà sur le bord : la flèche reste sur place");
    }
}
