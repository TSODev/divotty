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
│       ├── render.rs      # course view, viewport following the ball, shot preview + end-of-hole path recap
│       ├── sidebar.rs      # info column (title, hole, score, club, last shot, aim, controls)
│       ├── menu.rs          # course selection screen
│       ├── scorecard.rs      # end-of-round scorecard screen (every hole, par/strokes/label, total)
│       ├── lang.rs             # display language (English by default, French toggle)
│       └── format.rs            # shared display helpers (difficulty stars, compass arrows...)
└── courses/
    ├── demo/            # example course (1 hole)
    │   ├── course.yaml     # course index (name, difficulty, hole order)
    │   └── hole_01.course  # a hole: YAML frontmatter + ASCII grid
    └── quick3/          # a 3-hole course (par 3/4/5) for exercising multi-hole chaining
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
- **Tab / Shift+Tab**: change club, forward or backward (Driver → Wood → Hybrid → Iron → Wedge → Putter)
- **+ / -**: raise/lower shot power (3-6, 6 = full power) — see below
- **Space**: play the shot (rolls the dice)
- **Z**: toggle map zoom (x3, off by default)
- **S**: save the current game
- **L**: switch language
- **qq**: quit (press twice to confirm)

Once you hole out, aiming/playing/saving are disabled and the controls
switch to:
- **N**: next hole (only shown if the course has one left)
- **Enter**: finish the round on the last hole — shows a full scorecard
  (every hole's par/strokes/label plus the total) before returning to the
  menu
- **R**: replay the current hole
- **M**: back to the course menu (the course stays in the list, so you can
  play it again)
- **qq**: quit

The map constantly shows a preview of the shot being lined up: a dotted
trajectory guide up to maximum range, and a dispersion halo around the
average landing spot, before the dice are even rolled. Wind (random
direction and strength, rolled per hole) drifts non-putt shots and is
shown in the Aim panel alongside the player's own aim compass, as a
direction arrow and a color-coded Calm/Moderate/Strong label rather than
a raw number — the preview itself ignores wind on purpose, so reading it
and compensating is on you.

The die roll (1-6) is always uniform, but `+`/`-` let you dial back your
shot power (shown as a 4-slot bar, `+---` to `---+`, in the Club panel) so
a good roll can't send the ball flying past a nearby green — turning it
down to 3, for instance, means the die can only ever come up 1-3. Power
can't go below 3 (any lower would make even a Putter nearly unable to
reach the hole) and resets to full whenever you change club or start a
new hole, so it's a per-shot fine-tune rather than a standing preference.

Putting rewards a good approach shot: the Putter's accuracy scales with
how close the ball already is to the hole, so a short putt is close to
automatic while a long lag putt stays a real gamble. A ball hit into
water or out of bounds no longer resets all the way back to where the
shot was played from — it backtracks along the shot's own path and drops
at the last safe spot before the hazard (hopping over a tree if there's
one in the way), still costing the usual penalty stroke. Once you hole
out, the map swaps the aim preview for a recap of the whole hole: a red
ball at each stop the ball made along the way, connected by a trail of
yellow balls from tee to cup.

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
| `X` | Penalty area (charges a stroke, unlike water/OOB it doesn't force a drop) |

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
dispersion, per-club terrain sensitivity, distance-scaled putting
precision, obstacle fly-over, wind drift, a backtracking drop on
water/out-of-bounds), club distances calibrated as realistic ratios of the
Driver, course parser validated with unit tests, range/dispersion preview
before playing, two-column multilingual UI (English/French) with an
optional map zoom, course selection menu with difficulty displayed,
save/resume, full multi-hole chaining with a complete end-of-round
scorecard, and a proper end-of-hole state (next hole, replay, or back to
menu). `cargo install divotty` works standalone: the demo and Quick 3
courses are embedded in the binary as a fallback if `courses/` isn't found
on disk. See `ROADMAP.md` for what's next, `CHANGELOG.md` for release
history, and `CLAUDE.md` for handoff context.

## License

Dual-licensed, your choice: [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).
