# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Version number shown in the sidebar's title panel.
- Visual zoom (x3) on the map, toggled manually with `Z` (off by default).
  Purely cosmetic — the underlying whole-cell position model is unchanged.
- Wind: a random direction and strength are rolled when a hole loads, and
  drift the ball's landing spot downwind (proportional to shot distance —
  putts are never affected). Shown in the Aim panel next to the player's
  own aim compass, and factored into the shot preview so it stays accurate.
- New penalty area terrain (`X`, shown in red): unlike water or
  out-of-bounds, it charges a stroke but doesn't force a drop back to
  where the shot started — the ball stays put.

### Changed
- Club distances recalibrated as realistic ratios of the Driver (Wood
  90%, Hybrid 80%, Iron 62.5%, Wedge 35% — the putter is deliberately
  left out of this scaling, it's a different short-range regime). An
  average Driver now leaves a Wood/Iron-range approach on a typical par 4
  instead of needing two full Drivers to get near the green.
- Redesigned the demo course (`courses/demo/hole_01.course`): the fairway
  is now a proper corridor bordered by rough on both sides, instead of
  nearly the entire 100x60 canvas being uniform fairway with rough only
  at the very top/bottom edges (`Rough` gameplay effects already existed
  in the engine — this was purely a course-design gap, not a missing
  feature). Also added a penalty area patch on each side of the rough to
  exercise the new terrain in play, and a small out-of-bounds patch just
  behind the green (long approach shots can now fly the green into OB) —
  the `OutOfBounds` terrain existed in the engine but the demo course had
  never actually used it.

### Fixed
- The tee and hole markers no longer get hidden behind the shot preview
  overlay (trajectory guide dots, the expected-landing marker) when they
  happen to line up — the landmark's glyph stays visible, only its color
  tints to reflect the overlap.
- The hole marker no longer corrupts the rest of the row when zoomed in on
  the green: it used a double-width emoji glyph (⛳) that the low-level,
  cell-by-cell map renderer isn't equipped to handle, throwing off column
  alignment once the zoom repeated it several times in a row. Replaced
  with a single-width flag glyph (⚑).

### Changed
- Tee and hole are easier to spot on the map: the hole is now a flag (⛳)
  instead of a barely-visible black disc, and the tee's `D` marker uses a
  bright, distinct color instead of plain white.
- Course view (the map) is now framed with a border, like every other
  sidebar panel, and is centered within that panel when the terminal is
  wider than the course grid (it still follows the ball with no centering
  once the grid no longer fits on screen).
- Fairway is duller and the shot preview (trajectory guide, dispersion
  halo, landing marker) is brighter and bold, so aiming reads more clearly
  against the terrain. The green's `,` marker is now a bright `O`, clearly
  distinct from the rough's similar green tone.
- Controls panel content is anchored to the bottom of the panel instead of
  the top.
- Course grid size increased from 50x25 to 100x60 to support holes ranging
  from par 4 to par 8 (the old grid's ~56-cell diagonal couldn't fit a
  credible par 6-8, which needs ~80-100+ cells of cumulative travel). The
  demo course (`courses/demo/hole_01.course`) and the in-memory fallback
  course were both migrated to the new dimensions.

**Note:** this changes the `.course` file format's required dimensions —
any custom `.course` file written for 0.1.0 will need to be resized to
100x60 to keep parsing.

## [0.1.0] - 2026-07-30

Initial release. Published on [crates.io](https://crates.io/crates/divotty)
(`cargo install divotty`).

### Added
- `.course` file format: YAML frontmatter (name, par, description) followed
  by a 50x25 ASCII terrain grid, with a validating parser (tee/hole
  uniqueness, exact dimensions, known characters).
- Terrain types (tee, fairway, rough, bunker, water, tree, green,
  out-of-bounds), each with a gameplay profile (distance/dispersion
  multipliers, landing penalties, forced drops, trajectory blocking).
- Shot resolution engine: dice roll + club + terrain modifiers + random
  dispersion, deterministic under a seeded RNG.
- Six clubs (Driver, Wood, Hybrid, Iron, Wedge, Putter), each with its own
  base distance, dispersion, and **terrain sensitivity** — difficult
  terrain (rough, bunker) penalizes long clubs proportionally more than
  short ones.
- Obstacle fly-over rule: a tree blocks the shot only if it's within the
  first ~15% of the flight (low-altitude zone); farther obstacles are
  flown over, and water never blocks a trajectory (only landing matters).
- Shot preview before playing: dotted trajectory guide up to maximum
  range, dispersion halo around the average landing spot, and a landing
  marker, all computed without consuming the RNG.
- Two-column terminal UI: a 7-panel sidebar (title, hole, score, club,
  last shot, aim, controls) alongside a course view with a viewport that
  follows the ball and dynamically fills the available space.
- English/French UI localization (English by default, `L` to toggle) —
  game logic stays language-agnostic; only the UI layer translates.
- Per-course difficulty rating (1 to 4 stars), set in `course.yaml`,
  purely informational.
- Course selection menu, listing every course found under `courses/` with
  its name, difficulty, total par, and hole count.
- Save and resume an in-progress game (`S` to save, `[C]` to resume from
  the menu) — course, current hole, strokes, ball position, club, and aim.
- Double-press quit confirmation (`qq`) in both the menu and in-game, to
  avoid accidental exits.
- Dual MIT / Apache-2.0 license.
- GitHub Actions CI (build + test on push and pull request).

### Changed
- Collapsed the original 3-crate Cargo workspace (`divotty-core`,
  `divotty-tui`, `divotty`) into a single binary crate, since crates.io
  requires path dependencies to point at already-published crates —
  publishing just `divotty` on its own wasn't possible otherwise. `core`
  and `tui` are now internal modules instead of separate crates.
