# Divotty

[![CI](https://github.com/TSODev/divotty/actions/workflows/ci.yml/badge.svg)](https://github.com/TSODev/divotty/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A terminal (TUI) golf game written in Rust. Dice + club choice + aimed
direction + course conditions (fairway, rough, bunker, water, trees, green)
→ shot resolution with random dispersion.

## Project structure

A single binary crate (`divotty`), organized as internal modules:

```
divotty/
├── src/
│   ├── main.rs          # menu → GameState → game loop, save/resume
│   ├── core/            # pure game logic module, zero UI dependency
│   │   ├── mod.rs
│   │   ├── terrain.rs      # terrain types + gameplay profiles (distance, dispersion, penalties)
│   │   ├── course.rs       # 100x60 grid, .course file parsing, Course::discover
│   │   ├── shot.rs          # shot resolution (dice + club + terrain) + preview (ShotPreview)
│   │   └── scoring.rs        # per-hole score / scorecard (semantic labels, no display text)
│   └── tui/             # ratatui rendering module
│       ├── mod.rs
│       ├── render.rs      # course view, viewport following the ball + dispersion preview
│       ├── sidebar.rs      # info column (title, hole, score, club, last shot, aim, controls)
│       ├── menu.rs          # course selection screen
│       ├── lang.rs           # display language (English by default, French toggle)
│       └── format.rs          # shared display helpers (difficulty stars...)
└── courses/
    └── demo/            # example course (1 hole)
        ├── course.yaml     # course index (name, difficulty, hole order)
        └── hole_01.course  # a hole: YAML frontmatter + ASCII grid
```

## Running the game

From the repository root (the game looks up `courses/` and `save.yaml`
relative to the current directory):

```sh
cargo run
```

A course selection screen shows up first (courses found under `courses/`,
with difficulty stars, total par and hole count). If a save exists
(`save.yaml`), the `[C]` option lets you resume it.

Menu controls:
- **↑ / ↓**: change selected course
- **Enter**: play the selected course
- **C**: resume the saved game (if available)
- **L**: switch language (English / French)
- **qq**: quit (press twice to confirm)

In-game controls:
- **Left/right arrows**: adjust aim angle
- **Tab**: change club (Driver → Wood → Hybrid → Iron → Wedge → Putter)
- **Space**: play the shot (rolls the dice)
- **S**: save the current game
- **L**: switch language
- **qq**: quit (press twice to confirm)

The map constantly shows a preview of the shot being lined up: a dotted
trajectory guide up to maximum range, and a dispersion halo around the
average landing spot, before the dice are even rolled.

## `.course` file format

A hole is a YAML frontmatter (`name`, `par`, optional `description`)
followed by `---` then an ASCII grid of exactly **100 columns x 60 rows**.

Character legend:

| Character | Terrain |
|---|---|
| `D` | Tee (start) — exactly one per hole |
| `H` | Hole (target) — exactly one per hole |
| `.` | Fairway |
| `=` | Rough |
| `B` | Bunker |
| `~` | Water |
| `T` | Tree |
| `G` | Green |
| ` ` (space) | Out of bounds |

A course (1, 9, or 18 holes) is a folder containing a `course.yaml` that
gives its name, its difficulty (1 to 4, purely indicative), and the list
of `.course` files in play order:

```yaml
name: "My course"
difficulty: 2
holes:
  - hole_01.course
  - hole_02.course
```

See `courses/demo/` for a complete, working example.

## Current state

Shot resolution engine tested (dice + club + terrain + seedable random
dispersion, per-club terrain sensitivity, obstacle fly-over), course parser
validated with unit tests, range/dispersion preview before playing, two-column
multilingual UI (English/French), course selection menu with difficulty
displayed, save/resume. Multi-hole chaining and a full scorecard aren't in
yet. See `ROADMAP.md` for what's next, `CHANGELOG.md` for release history,
and `CLAUDE.md` for handoff context.

## License

Dual-licensed, your choice: [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).
