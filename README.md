# Divotty

[![CI](https://github.com/TSODev/divotty/actions/workflows/ci.yml/badge.svg)](https://github.com/TSODev/divotty/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](#license)

A terminal (TUI) golf game written in Rust. Dice + club choice + aimed
direction + course conditions (fairway, rough, bunker, water, trees, green)
→ shot resolution with random dispersion. Comes with two in-game builders
(hole and course) so you can draw your own layouts without leaving the
terminal.

## Project structure

A single binary crate (`divotty`), organized as internal modules:

```
divotty/
├── src/
│   ├── main.rs          # menu → GameState → game loop, save/resume, hole & course builders
│   ├── core/            # pure game logic module, zero UI dependency
│   │   ├── mod.rs
│   │   ├── terrain.rs      # terrain types + gameplay profiles (distance, dispersion, penalties)
│   │   ├── course.rs       # 100x60 grid, .course/course.yaml parsing, Course::discover
│   │   ├── shot.rs          # shot resolution (dice + club + terrain), preview, line sampling
│   │   └── scoring.rs        # per-hole score / scorecard (semantic labels, no display text)
│   └── tui/             # ratatui rendering module
│       ├── mod.rs
│       ├── render.rs      # course view, viewport following the ball, zoom levels, shot preview + path recap
│       ├── sidebar.rs      # info column (title, hole, score, club, last shot, aim, controls)
│       ├── menu.rs          # course selection screen
│       ├── scorecard.rs      # end-of-round scorecard screen (every hole, par/strokes/label, total)
│       ├── builder.rs         # hole builder screens (setup, drawing, file picker + preview)
│       ├── course_builder.rs   # course builder screens (picker, setup, hole assembly)
│       ├── lang.rs              # display language (English by default, French toggle)
│       └── format.rs             # shared display helpers (difficulty stars, compass arrows...)
└── courses/
    ├── demo/            # example course (1 hole)
    │   ├── course.yaml     # course index (name, difficulty, hole order)
    │   └── hole_01.course  # a hole: YAML frontmatter + ASCII grid
    ├── quick3/          # a 3-hole course (par 3/4/5) for exercising multi-hole chaining
    └── _library/        # holes saved from the builder that aren't part of a course yet
```

## Running the game

```sh
cargo run
```

If a `courses/` folder exists next to where you run it (the usual `cargo
run` workflow from the repository root), courses/saves/the builder's
library all live there. Otherwise — the common case for `cargo install
divotty` run from anywhere — the game falls back to a per-user data
directory (`~/.local/share/divotty` on Linux, and the platform equivalent
on macOS/Windows), and ships the demo and Quick 3 courses embedded in the
binary so there's always something to play.

A course selection screen shows up first (courses found under `courses/`,
with difficulty stars, total par and hole count). If a save exists, the
`[C]` option lets you resume it.

Menu controls:
- **↑ / ↓**: change selected course
- **Enter**: play the selected course
- **C**: resume the saved game (if available)
- **E**: open the hole builder
- **P**: open the course builder
- **L**: switch language (English / French)
- **qq**: quit (press twice to confirm)

In-game controls:
- **Left/right arrows**: adjust aim angle
- **Tab / Shift+Tab**: change club, forward or backward (Driver → Wood → Hybrid → Iron → Wedge → Putter)
- **+ / -**: raise/lower shot power (3-6, 6 = full power) — see below
- **Space**: play the shot (rolls the dice)
- **Z**: cycle map zoom — normal → zoomed in (x3, easier to read a short putt) → zoomed out (the whole hole shrunk to fit on screen) → back to normal
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
and compensating is on you. If the hole itself falls outside the visible
area (a long or tall layout), a compass arrow marks the nearest edge in
its actual direction, so you're never guessing which way to look.

Playing a shot doesn't teleport the ball straight to where it lands — it
visibly travels there a step at a time (capped so a long drive doesn't
take proportionally longer to watch than a short putt). Aiming, changing
club and saving are disabled while the ball is moving; any key
fast-forwards straight to the result instead of forcing a wait every time.

The die roll (1-6) is always uniform, but `+`/`-` let you dial back your
shot power (shown as a 4-slot bar, `+---` to `---+`, in the Club panel) so
a good roll can't send the ball flying past a nearby green — turning it
down to 3, for instance, means the die can only ever come up 1-3. Power
can't go below 3 (any lower would make even a Putter nearly unable to
reach the hole) and resets to full whenever you change club or start a
new hole, so it's a per-shot fine-tune rather than a standing preference.

Putting rewards a good approach shot: the Putter's accuracy scales with
how close the ball already is to the hole, so a short putt is close to
automatic while a long lag putt stays a real gamble. A putt that rolls
over the hole falls in even if the raw distance would have carried it
further — every other club still has to land exactly on the hole, since
it doesn't roll along the ground the rest of the way there. A ball hit
into water or out of bounds no longer resets all the way back to where
the shot was played from — it backtracks along the shot's own path and
drops at the last safe spot before the hazard (hopping over a tree if
there's one in the way), still costing the usual penalty stroke. Once you
hole out, the map swaps the aim preview for a recap of the whole hole: a
red ball at each stop the ball made along the way, connected by a trail of
yellow balls from tee to cup.

## Hole builder

Press **E** from the course menu to draw a hole entirely from the
keyboard — no external tool needed, and a saved file is always one the
game can already load, since saving validates it the same way loading
does.

You'll first see a picker: choose **+ New hole**, or pick any existing
`.course` file to open for editing, either "modify in place" (saves
straight back to that file) or "duplicate" (saves as a new file). Picking
a hole shows a live preview — name, par, dimensions, a small map — next
to the list.

For a new hole, pick a par (a grid size is suggested from it, scaled so a
par 7+ hole gets the full 100x60 canvas). Then draw:

- **Letters/symbols**: paint the current cell with that terrain and
  auto-advance to the next one (row by row, wrapping like text)
- **↑ ↓ ← →**: move the cursor freely
- **U**: undo the last change (a single cell, an entire fill, or an
  entire block — always one `U` per action, however many cells it
  touched)
- **C**: flood-fill every cell still out of bounds with a terrain of your
  choice — press `C`, then the terrain key; already-painted cells are
  left untouched
- **R**: block mode — anchor a rectangle on the current cell, resize the
  opposite corner with the arrow keys, press a terrain key to pick what to
  fill it with (repeatable if you change your mind — the rectangle
  previews live on the map in that terrain), then `Enter` to apply it.
  Unlike `C`, a block overwrites whatever terrain is already there —
  except the tee and the hole, which a block never overwrites even if
  they fall inside it
- **N**: rename — sets the hole's displayed name, and if it already has a
  file on disk, offers to rename that file too (a real rename, nothing
  left behind)
- **S**: save (to `courses/_library/`, a holding area for holes not yet
  part of a course)
- **qq** / **Esc Esc**: quit / back to menu (press twice to confirm)

Terrain character legend (case-insensitive, plus a couple of
layout-friendly aliases so painting doesn't need an awkward key combo on
every keyboard — `F`/`;` also paint fairway, `W`/`é` also paint water):

| Character | Terrain |
|---|---|
| `D` | Tee (start) — exactly one per hole |
| `H` | Hole (target) — exactly one per hole |
| `.` | Fairway (also `F` / `;`) |
| `=` | Rough |
| `B` | Bunker |
| `~` | Water (also `W` / `é`) |
| `T` | Tree |
| `G` | Green |
| ` ` (space) | Out of bounds |
| `X` | Penalty area (charges a stroke, unlike water/OOB it doesn't force a drop) |

The header bar shows this legend at all times, colored to match the map,
plus the cursor's exact (x, y) position and a row progress readout that
warns as you approach the last row (auto-advance stops there rather than
wrapping around).

## Course builder

Press **P** from the course menu to assemble holes into a playable
course. Choose **+ New course** (name + difficulty) or an existing course
to keep editing, then:

- **A**: add a hole — opens a picker over every `.course` file found
  under `courses/` (library included, with a live preview). Adding a hole
  always copies its file into the course's folder rather than referencing
  it, so the same hole can be reused independently across as many courses
  as you like; name collisions are resolved automatically
- **X**: remove the selected hole from the list (never deletes its file)
- **[ / ]**: reorder the selected hole
- **N**: rename — same as the hole builder, also offers to rename the
  course's folder on disk if it already exists (refused rather than
  overwritten on a collision, since a folder can hold several files)
- **←/→**: change difficulty (1-4 stars)
- **S**: save (`course.yaml`, plus copying in any newly added holes)
- **qq** / **Esc Esc**: quit / back to menu

The course list on the main menu refreshes as soon as you leave the
builder, so a course you just built or edited shows up immediately.

## `.course` file format

A hole is a YAML frontmatter (`name`, `par`, optional `description`)
followed by `---` then an ASCII grid. The grid is normally exactly **100
columns x 60 rows**, but a hole can declare a smaller size instead:

```yaml
name: "Small hole"
par: 3
width: 40
height: 24
---
```

The declared grid is then centered automatically inside the full 100x60
canvas, surrounded by out-of-bounds — existing files that don't declare a
size are completely unaffected, and the game/rendering never see anything
but a full 100x60 grid either way.

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
precision with fall-in-on-overshoot, obstacle fly-over, wind drift, a
backtracking drop on water/out-of-bounds), club distances calibrated as
realistic ratios of the Driver, course parser validated with unit tests,
range/dispersion preview before playing with a visible ball-travel
animation, three-level map zoom (normal/in/out) with an off-screen hole
indicator, two-column multilingual UI (English/French), course selection
menu with difficulty displayed, save/resume, full multi-hole chaining
with a complete end-of-round scorecard, and a proper end-of-hole state
(next hole, replay, or back to menu).

A full in-game hole builder (draw, undo, flood-fill, rectangle blocks,
rename/duplicate existing holes) and course builder (assemble holes from
a shared library into a playable course) round out content creation
without any external tooling. `cargo install divotty` works standalone:
the demo and Quick 3 courses are embedded in the binary as a fallback if
`courses/` isn't found on disk. See `ROADMAP.md` for what's next,
`CHANGELOG.md` for release history, and `CLAUDE.md` for handoff context.

## License

Dual-licensed, your choice: [MIT](LICENSE-MIT) or [Apache License 2.0](LICENSE-APACHE).
