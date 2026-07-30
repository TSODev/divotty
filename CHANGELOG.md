# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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
