# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- Once you hole out, the map replaces the (now pointless) aim preview with
  a recap of the whole hole: a red ball at each intermediate stop and a
  trail of yellow balls connecting them, from tee to cup.

### Changed
- Putting is less of a coin flip now: the Putter's accuracy improves the
  closer the ball already is to the hole, instead of a fixed dispersion
  regardless of distance. A short putt left by a good approach shot is
  now close to a sure thing (like a real-golf gimme); a long lag putt
  stays a genuine risk. Every other club is unaffected.
- Aiming a very short putt (1-2 cells) is easier to read in zoom mode:
  the ball and hole now each take up a single character in their zoomed
  block instead of filling it entirely, and a small arrow next to the
  ball points in the aimed direction — reliable even when the shot is too
  short for the landing markers to visibly move off the ball's own cell.
  The hole is also boxed in a small rounded frame (`╭─╮│⚑│╰─╯`, matching
  the rounded borders used everywhere else in the UI) so it stands out
  from whatever terrain surrounds it. The rest of the shot preview (the
  trajectory guide dots, the dispersion halo, the landing marker) got the
  same treatment: each now shows as a single character in its cell's
  zoomed block instead of filling all of it, so a guide dot reads as a
  dot again instead of a solid square. Guide dots specifically go one step
  further: each is a small 3-dot segment oriented with the shot (horizontal,
  vertical, or diagonal), lining up edge-to-edge with the next cell's
  segment so the trajectory reads as one continuous line.
- The end-of-round scorecard screen now has some breathing room instead of
  text touching the border directly.
- The 7 sidebar panels now have a subtle dark green background instead of
  the terminal's plain black, and a small colored tick (`▏`) between the
  left border and each line's text instead of it touching the frame
  directly. The title panel also dropped its own "Divotty" border label,
  since the icon and version are already shown inside.
- A ball hit into water or out of bounds no longer resets all the way back
  to where the shot was played from. It now backtracks along the shot's
  own path and stops at the first safe spot before the hazard — hopping
  back over a tree if there's one in the way too, so it doesn't just trade
  one obstacle for another. The penalty stroke is still charged either
  way. The last-shot panel now says where the ball ended up ("Dropped ·
  the fairway") instead of a generic message. A tree that blocks a shot
  directly is unaffected by this — the ball still stays put in the tree,
  no penalty, just a tougher next shot.

### Fixed
- The ball marker disappeared after every shot when zoom was off — a
  regression from the change above, which had moved where the marker gets
  drawn into a branch that only ran while zoomed.

## [0.2.0] - 2026-07-31

### Added
- Multi-hole courses now actually chain holes: once a hole is holed out,
  pressing `N` moves on to the next one (ball, aim, wind, strokes and club
  reset, same as replaying a hole) instead of being stuck on hole 1 of the
  course. `N` only appears once there's another hole to play. The running
  scorecard is now saved and resumed with the rest of the game state.
- A running total ("Total: N (±M)") appears in the Score panel once a
  course has more than one hole, alongside the current hole's own score.
- Finishing the last hole and pressing `Enter` ("finish round") now shows
  a full scorecard screen — every hole played, with its par, strokes and
  Birdie/Bogey-style label, plus the overall total — before returning to
  the course menu. This applies to every course, including single-hole
  ones (the demo course included), not just multi-hole rounds.
- New 3-hole course, `courses/quick3/` ("Quick 3", difficulty 2): a par 3,
  4 and 5 (par 12 total) with distinct orientations and hazards, meant as
  a fast way to exercise hole-to-hole chaining without playing a full
  9-hole round.
- `Shift+Tab` cycles clubs backward (Putter → Wedge → ... → Driver),
  complementing `Tab`'s forward cycle — no need to go all the way around
  to reach the previous club.
- Shot power (`+`/`-`, shown in the Club panel): caps the die roll at a
  player-chosen value from 3 to 6 (6 = full power, the original behavior)
  so a lucky high roll can't send the ball flying past a nearby green.
  Framed as "power" rather than exposing the die-cap mechanism, to stay in
  golf terms. Floored at 3 rather than 1, since a lower cap would leave
  even a Putter almost unable to reach the hole. Resets to 6 on every new
  hole and every club change, so it's a per-shot fine-tune rather than a
  standing preference. Shown as a small 4-slot slider (`+---` to `---+`)
  rather than plain text.
  The shot preview (guide/halo) reflects the current cap.
- Version number shown in the sidebar's title panel.
- Sidebar panels now have rounded borders, a distinct accent color each,
  and colored/bold text that reacts to what's happening: the score panel
  turns gold for an eagle or better, green for a birdie, orange/red for a
  bogey or worse; the last-shot message turns green when holed and red on
  a penalty; the wind reading turns from green (calm) to yellow to red
  (strong) with its strength.
- Visual zoom (x3) on the map, toggled manually with `Z` (off by default).
  Purely cosmetic — the underlying whole-cell position model is unchanged.
- Wind: a random direction and strength are rolled when a hole loads, and
  drift the ball's landing spot downwind (proportional to shot distance —
  putts are never affected). Shown in the Aim panel next to the player's
  own aim compass, as a direction arrow and a Calm/Moderate/Strong label
  (color-coded green/yellow/red) rather than a raw strength number. The
  shot preview deliberately ignores wind, though — it's on the player to
  read it and compensate when aiming, rather than have the preview quietly
  correct for it.
- New penalty area terrain (`X`, shown in red): unlike water or
  out-of-bounds, it charges a stroke but doesn't force a drop back to
  where the shot started — the ball stays put.
- Holing out now actually ends the hole: aiming, playing, and saving are
  disabled once the ball is in, and the Controls panel switches to
  `R` (replay this hole), `M` (back to the course menu), or `qq` (quit).
  Previously nothing happened on holing out and you could keep swinging
  indefinitely. Replaying a course from the menu no longer removes it
  from the list, so it can be played again in the same session.

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
- A `cargo install`'d game with no `courses/` folder next to it now falls
  back to the real demo and Quick 3 courses (embedded in the binary at
  compile time) instead of a single generic hole — previously a player
  who didn't also clone the repo would never see the multi-hole chaining
  this release adds, since the bundled fallback was always a single hole.
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
- Course view (the map) is now framed with a rounded border, like every
  other sidebar panel, and is centered within that panel when the terminal
  is wider than the course grid (it still follows the ball with no
  centering once the grid no longer fits on screen).
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
