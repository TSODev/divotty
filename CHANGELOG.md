# Changelog

All notable changes to this project are documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added
- The hole builder can now paint a whole rectangle of terrain at once:
  press `R` to anchor a block on the current cell, move the opposite
  corner with the arrow keys, press a terrain key to pick what to fill it
  with (repeatable if you change your mind — the rectangle previews live
  on the map in that terrain the whole time), then `Enter` to apply it and
  jump back to normal drawing. `Esc` cancels at any point without changing
  anything. Unlike `C` (fill), which only ever touches remaining
  out-of-bounds cells, a block overwrites whatever terrain is already
  there, since that's the point of drawing one (a bunker or green over an
  existing fairway, say) — except the tee and the hole, which a block
  never overwrites even if they fall inside it, since those are usually
  placed first. Undoing (`U`) reverts the whole rectangle in one press,
  each cell restored to its own original terrain.
- Renaming a hole or a course used to mean duplicating it under a new name
  and deleting the old file by hand outside the game. `N` now also offers
  to rename the underlying file (for a hole) or folder (for a course)
  whenever one already exists on disk — a real rename, nothing left
  behind — right after confirming the new display name. Only kicks in once
  something's actually been saved; on a brand-new hole or course, `N`
  still just renames the display name as before. Renaming a course folder
  onto an existing one is refused outright rather than offered as an
  overwrite, since a course folder can hold several files.
- `Z` now cycles through three zoom levels instead of just toggling one:
  Normal, the existing zoomed-in view, and a new zoomed-out overview that
  shrinks the whole hole to fit on screen at once. Handy for a long or
  very vertical hole where the tee and the cup are too far apart to both
  show up on a normal screen — the map always follows the ball, so without
  an overview you'd only ever see whichever end is close by. In the
  overview, tee and hole markers are never lost even when several grid
  cells collapse into one character, since they're prioritized over
  background terrain.
- In the normal (non-zoomed) view, when the hole itself is outside the
  visible area, a compass arrow now appears on the nearest edge pointing
  in its real direction — no more guessing which way to look when a shot
  travels further than the screen can currently show.
- The hole builder can now flood-fill every remaining out-of-bounds cell
  with a terrain of your choice: press `C`, then the terrain key you want
  to use. Already-painted cells are left untouched, so it's safe to use
  even partway through detailing a hole — handy since a fresh grid can be
  well over a thousand cells to paint by hand otherwise. Undoing (`U`)
  reverts the whole fill in one press rather than one cell at a time.
- A course builder to assemble existing holes into a playable course:
  press `P` from the course menu, choose "+ New course" (name + difficulty)
  or an existing course to keep editing, then build the ordered hole list
  from a picker that browses every `.course` file found under `courses/`
  (library included) with a live preview. `A` adds the selected hole, `X`
  removes it from the list, `[`/`]` reorder it, `N` renames the course,
  `←`/`→` change its difficulty, `S` saves. Adding a hole to a course always
  copies its file rather than referencing it — the same hole can be reused,
  independently, across as many courses as you like, and name collisions
  are resolved automatically with a counter. Removing a hole from the list
  never deletes its file. The course list on the main menu refreshes as
  soon as you leave the builder, so a course you just built or edited shows
  up immediately.
- The hole builder now accepts a couple of extra keys so painting terrain
  doesn't require an awkward key combo on every keyboard layout: `.`
  (fairway) also accepts `F` or `;`, and `~` (water) also accepts `W` or
  `é` — `;` and `é` specifically match the unshifted key that produces
  `.`/`~` on a French AZERTY layout, while `F`/`W` work with no modifier
  on any layout. This only changes what you can type; saved `.course`
  files are unaffected and keep using the same `.`/`~` characters as
  always. The builder's terrain legend shows the accepted aliases next to
  the two affected entries.
- Once you hole out, the map replaces the (now pointless) aim preview with
  a recap of the whole hole: a red ball at each intermediate stop and a
  trail of yellow balls connecting them, from tee to cup.
- `.course` files can now declare a smaller grid than the full 100x60
  canvas (`width`/`height` in the hole's frontmatter). The declared grid is
  centered automatically in the full canvas, surrounded by out-of-bounds —
  no more forcing every short hole to fill 100x60 by hand. Existing files
  that don't declare a size are completely unaffected.
- A first version of the in-game hole builder: press `E` from the course
  menu, pick a par (a suggested grid size follows, scaled proportionally
  so a par 7+ hole gets the full 100x60 canvas), then draw the hole
  entirely from the keyboard — typing a terrain character
  (`.`/`=`/`B`/`~`/`T`/`G`/`X`/`D`/`H`, either letter case) paints the
  current cell and auto-advances to the next one, arrows move freely, `U`
  undoes the last cell, `N` renames, `S` saves. The header bar shows the
  terrain legend at all times, each entry colored to match its terrain on
  the map, plus the cursor's exact (x, y) position — same 0-indexed
  coordinates as the `.course` grid and the printable PDF canvas, so you
  can navigate straight to a cell you planned out on paper — and a
  row/column progress readout that turns yellow, then red, as you approach
  the last row/column, since auto-advance stops there instead of wrapping
  around. Leaving with `Esc` now asks for confirmation just like `qq`
  does, and any key other than a second `Esc` cancels it — including `S`,
  which switches to saving instead, so you can save your work before
  leaving rather than losing it.
  Saving only ever asks for a plain file name (no path, no extension) —
  every hole is saved to `courses/_library/`, a holding area for holes not
  yet part of a course, with the name cleaned up automatically (special
  characters become underscores). Saving under a name that's already taken
  asks to confirm the overwrite (or lets you change the name) instead of
  silently appending a counter, so re-saving under the same name actually
  updates that file rather than piling up copies. A saved file is always
  one the game can already load, since saving validates it the same way
  loading does. Pressing `E` now opens a picker instead of jumping
  straight to a blank hole: choose "+ New hole", or pick any existing
  `.course` file to open for editing — either "modify in place" (saves
  straight back to that same file) or "duplicate" (saves as a new file,
  same as a brand-new hole). The picker also shows a live preview of the
  highlighted hole — name, par, dimensions, and a small map — next to the
  list. Every builder screen now shares the same layout as the game
  itself: the drawing screen has a stacked left sidebar (Hole, Position,
  Legend, Controls) with the map filling the right side instead of one
  wide banner above the grid, and the par setup screen shows a live
  preview of the blank grid at its suggested size next to the form.
  The hole's name and its save file name are now linked: naming the hole
  first (`N`) pre-fills the save prompt, and saving an unnamed hole feeds
  the typed file name back in as its name, so a `.course` file never ends
  up with a blank name. The save/rename prompts also gained a label
  ("File name:", "New name:") instead of showing a bare cursor.

### Changed
- A putt that rolls over the hole now falls in, even if it would have kept
  going otherwise — previously only the ball's exact final resting cell
  counted, so a well-aimed but slightly too strong putt would just roll
  past. Only the Putter is affected; every other club still has to land
  exactly on the hole, since it doesn't roll along the ground the rest of
  the way there.
- Playing a shot no longer teleports the ball straight to where it lands:
  it now visibly travels there, one grid cell every couple hundred
  milliseconds. A long drive doesn't take proportionally longer to watch
  than a short putt — the number of steps shown is capped regardless of
  distance. Aiming, changing club, and saving are disabled while the ball
  is still moving, and pressing any key fast-forwards straight to the
  result instead of making you wait out the full animation every time.
- Courses, save files, and the hole builder's library no longer depend on
  which directory you happen to launch `divotty` from. If `./courses`
  exists next to where you run it, behavior is unchanged (the usual
  `cargo run` workflow); otherwise the game now uses a proper per-user data
  directory (`~/.local/share/divotty` on Linux, and the equivalent on
  macOS/Windows) instead of scattering a `save.yaml` and an empty
  `courses/_library/` into whatever folder you happened to be in — the
  common case for anyone using `cargo install divotty` and running it from
  anywhere.
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
- A `.course` file's row count is now actually checked against its
  expected height while parsing. Previously only each row's width was
  validated, so a hole file with too few or too many grid lines could
  parse without error.

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
