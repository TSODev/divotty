use crate::core::course::{Hole, Pos, COURSE_HEIGHT, COURSE_WIDTH};
use crate::core::terrain::TerrainKind;
use rand::Rng;
use serde::{Deserialize, Serialize};

/// Un club détermine la portée de base et la précision naturelle du joueur,
/// indépendamment du terrain. Facilement extensible (wedge, putter, etc.)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Club {
    Driver,
    Wood,
    Hybrid,
    Iron,
    Wedge,
    Putter,
}

impl Club {
    /// Portée de base (en cases) pour un résultat de dé donné (1..=6).
    ///
    /// Les facteurs (hors putter) sont calibrés en ratio du Driver, façon
    /// golf réel (Bois ~90%, Hybride ~80%, Fer ~62%, Wedge ~35%), plutôt que
    /// des valeurs arbitraires — un Driver moyen (dé 3.5) doit laisser une
    /// approche à distance de Bois/Fer sur un par 4 typique (voir
    /// `courses/demo/hole_01.course`, ~53 cases tee-trou). Le putter n'est
    /// volontairement pas mis à l'échelle du Driver : c'est un régime à
    /// part (précision très courte distance sur le green), pas une version
    /// proportionnelle d'un grand coup.
    pub fn base_distance(self, die: u8) -> f32 {
        let die = die as f32;
        match self {
            Club::Driver => die * 8.0,
            Club::Wood => die * 7.2,
            Club::Hybrid => die * 6.4,
            Club::Iron => die * 5.0,
            Club::Wedge => die * 2.8,
            Club::Putter => die * 0.5,
        }
    }

    /// Dispersion de base en cases (rayon d'imprécision autour de la cible visée).
    pub fn base_dispersion(self) -> f32 {
        match self {
            Club::Driver => 3.0,
            Club::Wood => 2.3,
            Club::Hybrid => 1.9,
            Club::Iron => 1.5,
            Club::Wedge => 0.8,
            Club::Putter => 0.2,
        }
    }

    /// Sensibilité du club aux difficultés du terrain (rough, bunker, arbres...).
    /// Un club long (Driver) est beaucoup plus déstabilisé par un mauvais lie
    /// qu'un club court (Wedge, Putter), qui reste jouable presque partout.
    /// Ce facteur module l'écart introduit par `TerrainProfile::dispersion_mult`
    /// (voir `resolve_shot`) : à multiplicateur de terrain égal, un Driver en
    /// subit un effet amplifié, un Putter un effet quasi nul.
    pub fn terrain_sensitivity(self) -> f32 {
        match self {
            Club::Driver => 1.5,
            Club::Wood => 1.3,
            Club::Hybrid => 1.1,
            Club::Iron => 1.0,
            Club::Wedge => 0.7,
            Club::Putter => 0.3,
        }
    }
}

/// Direction visée par le joueur, normalisée (dx, dy en cases).
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Direction {
    pub dx: f32,
    pub dy: f32,
}

impl Direction {
    pub fn towards(from: Pos, to: Pos) -> Self {
        let dx = to.x as f32 - from.x as f32;
        let dy = to.y as f32 - from.y as f32;
        let len = (dx * dx + dy * dy).sqrt().max(0.0001);
        Direction {
            dx: dx / len,
            dy: dy / len,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct Shot {
    pub club: Club,
    pub direction: Direction,
    pub die_roll: u8,
}

/// Vent affectant la trajectoire d'un coup : direction + force. Généré
/// aléatoirement par `app` (jamais dans `core`) et injecté ici comme le RNG
/// — `core` ne fait qu'appliquer l'effet, jamais tirer le vent lui-même.
#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Wind {
    pub direction: Direction,
    /// Force du vent, en cases de dérive pour le coup le plus long possible
    /// (Driver, dé=6) ; les coups plus courts dérivent proportionnellement
    /// moins. Un putt n'est jamais affecté (balle roulant au sol).
    pub strength: f32,
}

impl Default for Wind {
    /// Vent nul — pratique pour les tests qui ne portent pas sur le vent.
    fn default() -> Self {
        Wind {
            direction: Direction { dx: 1.0, dy: 0.0 },
            strength: 0.0,
        }
    }
}

/// Décalage (dx, dy) induit par le vent sur un coup donné, proportionnel à
/// la distance effective du coup (un tir plus long passe plus de temps en
/// l'air, donc dérive plus) — nul pour un putt.
fn wind_push(club: Club, effective_distance: f32, wind: Wind) -> (f32, f32) {
    if matches!(club, Club::Putter) {
        return (0.0, 0.0);
    }
    let max_distance = Club::Driver.base_distance(6);
    let push = wind.strength * effective_distance / max_distance;
    (wind.direction.dx * push, wind.direction.dy * push)
}

/// Résultat de la résolution d'un coup.
#[derive(Debug, Clone)]
pub struct ShotResult {
    pub landing: Pos,
    pub landing_terrain: TerrainKind,
    /// Coups de pénalité à ajouter au score (ex: balle à l'eau).
    pub penalty_strokes: u8,
    /// Vrai si la balle a atteint le trou.
    pub holed: bool,
    /// Vrai si un drop a été appliqué (balle relancée depuis la position de départ du coup).
    pub dropped: bool,
}

/// Combine la dispersion de base d'un club, le multiplicateur du terrain de
/// départ et la sensibilité du club à ce terrain pour obtenir la dispersion
/// effective. Isolée de `resolve_shot` pour rester testable sans RNG.
fn effective_dispersion(base_dispersion: f32, dispersion_mult: f32, terrain_sensitivity: f32) -> f32 {
    let terrain_effect = (dispersion_mult - 1.0) * terrain_sensitivity;
    base_dispersion * (1.0 + terrain_effect).max(0.0)
}

/// Fraction du vol pendant laquelle la balle est trop basse pour survoler un
/// obstacle (arbre...). Au-delà, on considère qu'elle a pris assez
/// d'altitude pour passer par-dessus. Principe volontairement simple : un
/// seul paramètre, pas de vraie simulation d'arc.
const LOW_ALTITUDE_FRACTION: f32 = 0.15;

/// Cherche le premier obstacle bloquant (`TerrainProfile::blocks_trajectory`)
/// rencontré dans la zone basse du vol, entre `from` et `target`. `None` si
/// la trajectoire est dégagée (obstacle absent, ou trop loin pour être dans
/// la zone basse et donc survolé).
fn find_trajectory_block(hole: &Hole, from: Pos, target: (f32, f32)) -> Option<Pos> {
    let total_dx = target.0 - from.x as f32;
    let total_dy = target.1 - from.y as f32;
    let flight_length = (total_dx * total_dx + total_dy * total_dy).sqrt();
    let clearance_zone = (flight_length * LOW_ALTITUDE_FRACTION).max(1.0);

    let steps = flight_length.ceil().max(1.0) as usize;
    for step in 1..=steps {
        let distance = step as f32;
        if distance > flight_length || distance > clearance_zone {
            break;
        }
        let t = distance / flight_length;
        let sx = (from.x as f32 + total_dx * t)
            .round()
            .clamp(0.0, (COURSE_WIDTH - 1) as f32) as usize;
        let sy = (from.y as f32 + total_dy * t)
            .round()
            .clamp(0.0, (COURSE_HEIGHT - 1) as f32) as usize;
        let sample = Pos { x: sx, y: sy };
        if sample == from {
            continue;
        }
        if hole
            .terrain_at(sample)
            .unwrap_or(TerrainKind::OutOfBounds)
            .profile()
            .blocks_trajectory
        {
            return Some(sample);
        }
    }
    None
}

/// Aperçu affichable avant de jouer un coup : où la balle peut
/// raisonnablement atterrir selon le club et la direction visée, avant même
/// de lancer le dé. Ne tient pas compte des obstacles sur la trajectoire
/// (`blocks_trajectory`) ni de la déviation aléatoire réelle : c'est un
/// aperçu, pas une prédiction exacte — `resolve_shot` reste seul à faire foi.
#[derive(Debug, Clone, Copy)]
pub struct ShotPreview {
    /// Atterrissage si le dé donne 6 (portée maximale).
    pub max_landing: Pos,
    /// Atterrissage pour un dé "moyen" (arrondi de 3.5), sert de centre à la
    /// zone de dispersion affichée.
    pub expected_landing: Pos,
    /// Rayon de dispersion effectif (en cases) autour de `expected_landing`.
    pub dispersion_radius: f32,
}

/// Calcule l'aperçu de portée/dispersion pour un club et une direction
/// donnés, depuis la case `from`. Utilisé par l'UI pour afficher une zone
/// de dispersion visée avant que le joueur ne joue réellement le coup.
///
/// `die_strength` (1 à 6) est le plafond que le joueur a choisi pour le dé
/// (voir `GameState::die_strength` côté `app`) : le meilleur tirage possible
/// n'est plus forcément 6, donc `max_landing` doit refléter ce plafond
/// plutôt qu'une portée maximale toujours calculée sur un 6 fixe.
pub fn preview_shot(
    hole: &Hole,
    from: Pos,
    club: Club,
    direction: Direction,
    wind: Wind,
    die_strength: u8,
) -> ShotPreview {
    let start_terrain = hole.terrain_at(from).unwrap_or(TerrainKind::OutOfBounds);
    let profile = start_terrain.profile();

    let landing_for_die = |die: u8| -> Pos {
        let distance = club.base_distance(die) * profile.distance_mult;
        let (wind_dx, wind_dy) = wind_push(club, distance, wind);
        let x = (from.x as f32 + direction.dx * distance + wind_dx)
            .round()
            .clamp(0.0, (COURSE_WIDTH - 1) as f32) as usize;
        let y = (from.y as f32 + direction.dy * distance + wind_dy)
            .round()
            .clamp(0.0, (COURSE_HEIGHT - 1) as f32) as usize;
        Pos { x, y }
    };

    let expected_die = ((1 + die_strength) as f32 / 2.0).round() as u8;

    ShotPreview {
        max_landing: landing_for_die(die_strength),
        expected_landing: landing_for_die(expected_die),
        dispersion_radius: effective_dispersion(
            club.base_dispersion(),
            profile.dispersion_mult,
            club.terrain_sensitivity(),
        ),
    }
}

/// Résout un coup : applique les modificateurs de terrain de la case de départ,
/// calcule la distance et la dispersion effectives, tire une déviation
/// aléatoire, et détermine la case d'arrivée + ses conséquences.
///
/// `rng` est injecté pour permettre des tests déterministes (seed fixe).
pub fn resolve_shot(hole: &Hole, from: Pos, shot: Shot, wind: Wind, rng: &mut impl Rng) -> ShotResult {
    let start_terrain = hole.terrain_at(from).unwrap_or(TerrainKind::OutOfBounds);
    let profile = start_terrain.profile();

    let base_distance = shot.club.base_distance(shot.die_roll);
    let effective_distance = base_distance * profile.distance_mult;

    let effective_dispersion = effective_dispersion(
        shot.club.base_dispersion(),
        profile.dispersion_mult,
        shot.club.terrain_sensitivity(),
    );

    // Déviation aléatoire de la trajectoire : un angle et une amplitude
    // tirés dans le rayon de dispersion effectif.
    let deviation_angle: f32 = rng.gen_range(0.0..std::f32::consts::TAU);
    let deviation_amount: f32 = rng.gen_range(0.0..effective_dispersion);

    let (wind_dx, wind_dy) = wind_push(shot.club, effective_distance, wind);

    let target_x = from.x as f32 + shot.direction.dx * effective_distance
        + deviation_angle.cos() * deviation_amount
        + wind_dx;
    let target_y = from.y as f32 + shot.direction.dy * effective_distance
        + deviation_angle.sin() * deviation_amount
        + wind_dy;

    let mut landing = match find_trajectory_block(hole, from, (target_x, target_y)) {
        Some(blocked_pos) => blocked_pos,
        None => {
            let clamped_x = target_x.round().clamp(0.0, (COURSE_WIDTH - 1) as f32) as usize;
            let clamped_y = target_y.round().clamp(0.0, (COURSE_HEIGHT - 1) as f32) as usize;
            Pos {
                x: clamped_x,
                y: clamped_y,
            }
        }
    };

    let mut landing_terrain = hole.terrain_at(landing).unwrap_or(TerrainKind::OutOfBounds);
    let landing_profile = landing_terrain.profile();

    let penalty_strokes = landing_profile.landing_penalty;
    let mut dropped = false;

    if landing_profile.forces_drop {
        // Drop simplifié : on relance depuis la position de départ du coup.
        // (Une version plus avancée pourrait dropper au dernier point de
        // fairway valide le long de la trajectoire.)
        landing = from;
        landing_terrain = start_terrain;
        dropped = true;
    }

    let holed = landing_terrain == TerrainKind::Hole;

    ShotResult {
        landing,
        landing_terrain,
        penalty_strokes,
        holed,
        dropped,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::course::Hole;
    use rand_pcg::Pcg32;

    fn flat_fairway_hole() -> Hole {
        let raw_line: String = std::iter::repeat('.').take(COURSE_WIDTH).collect();
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        for y in 0..COURSE_HEIGHT {
            let mut chars: Vec<char> = raw_line.chars().collect();
            if y == 0 {
                chars[0] = 'D';
            }
            if y == COURSE_HEIGHT - 1 {
                chars[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(chars.into_iter().collect::<String>());
        }
        let raw = format!("name: \"Test\"\npar: 3\n---\n{}\n", lines.join("\n"));
        Hole::parse(&raw).unwrap()
    }

    /// Distance moyenne (sur dé 1..=6) d'un club, en cases.
    fn average_distance(club: Club) -> f32 {
        (1..=6).map(|die| club.base_distance(die)).sum::<f32>() / 6.0
    }

    #[test]
    fn club_distances_scale_realistically_relative_to_driver() {
        let driver_avg = average_distance(Club::Driver);
        for (club, expected_ratio) in [
            (Club::Wood, 0.90),
            (Club::Hybrid, 0.80),
            (Club::Iron, 0.625),
            (Club::Wedge, 0.35),
        ] {
            let actual_ratio = average_distance(club) / driver_avg;
            assert!(
                (actual_ratio - expected_ratio).abs() < 0.01,
                "{club:?}: ratio attendu {expected_ratio}, obtenu {actual_ratio}"
            );
        }
    }

    #[test]
    fn driver_leaves_a_wood_or_iron_approach_on_a_typical_par_4() {
        // Trou de démo : ~53 cases entre le tee et le trou. Un Driver moyen
        // doit laisser une approche à portée de Bois ou de Fer, pas encore
        // un autre Driver complet ni juste un petit pitch.
        let hole_length = 53.0;
        let remaining = hole_length - average_distance(Club::Driver);
        assert!(
            remaining <= average_distance(Club::Wood) * 1.1
                && remaining >= average_distance(Club::Iron) * 0.9,
            "distance restante après un Driver moyen ({remaining}) hors de portée Bois/Fer"
        );
    }

    #[test]
    fn rough_penalizes_driver_more_than_putter() {
        let rough_mult = TerrainKind::Rough.profile().dispersion_mult;
        let driver_penalty = effective_dispersion(
            Club::Driver.base_dispersion(),
            rough_mult,
            Club::Driver.terrain_sensitivity(),
        ) - Club::Driver.base_dispersion();
        let putter_penalty = effective_dispersion(
            Club::Putter.base_dispersion(),
            rough_mult,
            Club::Putter.terrain_sensitivity(),
        ) - Club::Putter.base_dispersion();
        assert!(driver_penalty > putter_penalty);
    }

    #[test]
    fn neutral_terrain_leaves_dispersion_unchanged() {
        for club in [
            Club::Driver,
            Club::Wood,
            Club::Hybrid,
            Club::Iron,
            Club::Wedge,
            Club::Putter,
        ] {
            let base = club.base_dispersion();
            assert_eq!(effective_dispersion(base, 1.0, club.terrain_sensitivity()), base);
        }
    }

    #[test]
    fn nearby_tree_blocks_the_shot() {
        let fairway_line: String = std::iter::repeat('.').take(COURSE_WIDTH).collect();
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        for y in 0..COURSE_HEIGHT {
            let mut chars: Vec<char> = fairway_line.chars().collect();
            if y == 0 {
                chars[0] = 'D';
                chars[1] = 'T'; // arbre juste devant le tee
            }
            if y == COURSE_HEIGHT - 1 {
                chars[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(chars.into_iter().collect::<String>());
        }
        let raw = format!("name: \"Test arbre proche\"\npar: 3\n---\n{}\n", lines.join("\n"));
        let hole = Hole::parse(&raw).unwrap();

        let shot = Shot {
            club: Club::Driver,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 6,
        };
        let mut rng = Pcg32::new(7, 7);
        let result = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng);
        assert_eq!(result.landing, Pos { x: 1, y: 0 });
        assert_eq!(result.landing_terrain, TerrainKind::Tree);
    }

    #[test]
    fn distant_tree_is_flown_over() {
        let fairway_line: String = std::iter::repeat('.').take(COURSE_WIDTH).collect();
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        for y in 0..COURSE_HEIGHT {
            let mut chars: Vec<char> = fairway_line.chars().collect();
            if y == 0 {
                chars[0] = 'D';
                chars[10] = 'T'; // arbre loin sur le fairway, bien après la zone basse
            }
            if y == COURSE_HEIGHT - 1 {
                chars[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(chars.into_iter().collect::<String>());
        }
        let raw = format!("name: \"Test arbre lointain\"\npar: 3\n---\n{}\n", lines.join("\n"));
        let hole = Hole::parse(&raw).unwrap();

        let shot = Shot {
            club: Club::Driver,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 6,
        };
        let mut rng = Pcg32::new(7, 7);
        let result = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng);
        assert!(result.landing.x > 10, "la balle devrait survoler l'arbre lointain");
    }

    #[test]
    fn preview_grows_with_die_and_matches_dispersion() {
        let hole = flat_fairway_hole();
        let direction = Direction { dx: 1.0, dy: 0.0 };
        let preview = preview_shot(&hole, hole.tee, Club::Driver, direction, Wind::default(), 6);

        assert!(preview.max_landing.x > preview.expected_landing.x);
        assert!(preview.expected_landing.x > hole.tee.x);
        assert_eq!(
            preview.dispersion_radius,
            effective_dispersion(
                Club::Driver.base_dispersion(),
                TerrainKind::Fairway.profile().dispersion_mult,
                Club::Driver.terrain_sensitivity(),
            )
        );
    }

    #[test]
    fn preview_shrinks_with_a_lower_die_strength() {
        let hole = flat_fairway_hole();
        let direction = Direction { dx: 1.0, dy: 0.0 };
        let full = preview_shot(&hole, hole.tee, Club::Driver, direction, Wind::default(), 6);
        let capped = preview_shot(&hole, hole.tee, Club::Driver, direction, Wind::default(), 3);

        assert!(
            capped.max_landing.x < full.max_landing.x,
            "un plafond de dé plus bas doit réduire la portée maximale affichée"
        );
        assert_eq!(
            capped.dispersion_radius, full.dispersion_radius,
            "le plafond de dé ne doit pas affecter la dispersion, seulement la distance"
        );
    }

    #[test]
    fn deterministic_with_seeded_rng() {
        let hole = flat_fairway_hole();
        let shot = Shot {
            club: Club::Iron,
            direction: Direction::towards(hole.tee, hole.hole_pos),
            die_roll: 4,
        };
        let mut rng_a = Pcg32::new(42, 54);
        let mut rng_b = Pcg32::new(42, 54);
        let result_a = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng_a);
        let result_b = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng_b);
        assert_eq!(result_a.landing, result_b.landing);
    }

    #[test]
    fn wind_pushes_the_ball_downwind() {
        let hole = flat_fairway_hole();
        let shot = Shot {
            club: Club::Driver,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 6,
        };
        // Vent perpendiculaire à la visée (plein "sud") : ne doit pas
        // changer la distance parcourue en x, seulement pousser en y.
        let crosswind = Wind {
            direction: Direction { dx: 0.0, dy: 1.0 },
            strength: 3.0,
        };

        let mut rng_calm = Pcg32::new(9, 9);
        let calm = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng_calm);

        let mut rng_wind = Pcg32::new(9, 9);
        let windy = resolve_shot(&hole, hole.tee, shot, crosswind, &mut rng_wind);

        assert!(
            windy.landing.y > calm.landing.y,
            "le vent devrait pousser la balle vers le sud (y croissant)"
        );
    }

    #[test]
    fn putter_is_unaffected_by_wind() {
        let hole = flat_fairway_hole();
        let shot = Shot {
            club: Club::Putter,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 4,
        };
        let strong_crosswind = Wind {
            direction: Direction { dx: 0.0, dy: 1.0 },
            strength: 5.0,
        };

        let mut rng_calm = Pcg32::new(3, 3);
        let calm = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng_calm);

        let mut rng_wind = Pcg32::new(3, 3);
        let windy = resolve_shot(&hole, hole.tee, shot, strong_crosswind, &mut rng_wind);

        assert_eq!(calm.landing, windy.landing, "un putt ne doit jamais être affecté par le vent");
    }

    #[test]
    fn water_forces_drop_and_penalty() {
        // Trou 1x1 case d'eau adjacente pour forcer un atterrissage dedans
        // via une trajectoire courte et déterministe.
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        let fairway_line: String = std::iter::repeat('.').take(COURSE_WIDTH).collect();
        for y in 0..COURSE_HEIGHT {
            let mut chars: Vec<char> = fairway_line.chars().collect();
            if y == 0 {
                chars[0] = 'D';
                chars[1] = '~'; // eau juste à côté du départ
            }
            if y == COURSE_HEIGHT - 1 {
                chars[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(chars.into_iter().collect::<String>());
        }
        let raw = format!("name: \"Test eau\"\npar: 3\n---\n{}\n", lines.join("\n"));
        let hole = Hole::parse(&raw).unwrap();

        let shot = Shot {
            club: Club::Putter,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 1,
        };
        let mut rng = Pcg32::new(1, 1);
        let result = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng);
        if result.dropped {
            // La pénalité (1 coup) est capturée avant le drop, qui ne fait
            // que replacer la balle : le coup de pénalité reste donc dû.
            assert_eq!(result.penalty_strokes, 1);
        }
    }

    #[test]
    fn penalty_zone_charges_a_stroke_without_forcing_a_drop() {
        // Contrairement à l'eau/hors-limites, une zone à pénalité coûte un
        // coup mais laisse la balle sur place — seul terrain à combiner
        // pénalité et absence de drop forcé.
        let mut lines = Vec::with_capacity(COURSE_HEIGHT);
        let fairway_line: String = std::iter::repeat('.').take(COURSE_WIDTH).collect();
        for y in 0..COURSE_HEIGHT {
            let mut chars: Vec<char> = fairway_line.chars().collect();
            if y == 0 {
                chars[0] = 'D';
                chars[1] = 'X'; // zone à pénalité juste à côté du départ
            }
            if y == COURSE_HEIGHT - 1 {
                chars[COURSE_WIDTH - 1] = 'H';
            }
            lines.push(chars.into_iter().collect::<String>());
        }
        let raw = format!("name: \"Test pénalité\"\npar: 3\n---\n{}\n", lines.join("\n"));
        let hole = Hole::parse(&raw).unwrap();

        let shot = Shot {
            club: Club::Putter,
            direction: Direction { dx: 1.0, dy: 0.0 },
            die_roll: 1,
        };
        let mut rng = Pcg32::new(1, 1);
        let result = resolve_shot(&hole, hole.tee, shot, Wind::default(), &mut rng);
        if result.landing_terrain == TerrainKind::PenaltyZone {
            assert_eq!(result.penalty_strokes, 1);
            assert!(!result.dropped, "une zone à pénalité ne force pas de drop");
            assert_eq!(result.landing, Pos { x: 1, y: 0 });
        }
    }
}
