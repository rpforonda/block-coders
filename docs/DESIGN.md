# Block Coders: Design

This doc explains how Block Coders works inside and why it's built the way it
is. It's for anyone changing the code or adding levels. For playing and
installing, see the [README](../README.md).

## Contents

1. [Goals](#goals)
2. [Teaching approach](#teaching-approach)
3. [Architecture](#architecture)
4. [The world model](#the-world-model)
5. [The program model](#the-program-model)
6. [Dynamic levels](#dynamic-levels)
7. [Curriculum and level design](#curriculum-and-level-design)
8. [The block editor](#the-block-editor)
9. [Presentation](#presentation)
10. [Players and saving](#players-and-saving)
11. [Packaging](#packaging)
12. [Testing](#testing)
13. [Key decisions](#key-decisions)
14. [Limitations and future work](#limitations-and-future-work)

## Goals

**Goals**

- Teach the core ideas of programming (sequence, loops, functions,
  conditionals, while loops, algorithms) to kids aged about 8–12, through play.
- Make the ideas *necessary*, not just available. A level that teaches `if`
  shouldn't be solvable without `if`.
- Spark interest in real development: show that blocks are code and that the
  game itself is code (*Show Rust code*, editable text-map levels).
- Be easy to install and run: one small binary, no asset files, no account,
  works offline.

**Non-goals**

- A free-form programming language. Kids get a small, fixed instruction set
  that's easy to discover.
- Typing code. The bridge to typed code is the read-only Rust view.
- Multiplayer, online features, or tracking.

## Teaching approach

The design follows a few principles:

| Principle | How the game applies it |
|---|---|
| **One new idea at a time** | Each world introduces one concept. New blocks appear in the palette only when their level arrives (`Level::palette`). |
| **Explain at the moment of need** | Bit's intro popup appears on the level that introduces something new, not in a tutorial up front. |
| **Make the concept necessary** | `max_blocks` limits force loops and functions. Dynamic levels force `if`/`while`. |
| **Mistakes are information** | Errors name the cause in kid language and outline the block responsible in red. Nothing is ever lost: RESET restores the world, and the program stays. |
| **Debugging is a skill** | STEP plus a highlighted running block make the program visible as it runs. |
| **Reward good code, not just working code** | Stars compare the block count with the level's shortest solution (`par`). |
| **Name the idea afterwards** | The victory popup's "What you learned" names the concept the kid just used (decomposition, nested loops, algorithm...). |
| **Connect to the real world** | *Show Rust code*, plus lessons that mention robots, mazes and apps. |

## Architecture

Single binary, ~5,400 lines of Rust, with one dependency:
[macroquad](https://macroquad.rs) for windowing, 2D drawing, input and audio.

```mermaid
flowchart TD
    main["main.rs<br/>screens: title, players, level select<br/>window scaling, CLI, autopilot"]
    users["users.rs<br/>Who's playing? screen"]
    play["play.rs<br/>gameplay screen: run controls,<br/>animation, popups"]
    editor["editor.rs<br/>block editor"]
    program["program.rs<br/>blocks, compiler, VM,<br/>Rust codegen, save format"]
    world["world.rs<br/>tiles, rules, conditions,<br/>random map generation"]
    levels["levels.rs<br/>60 levels as data"]
    save["save.rs<br/>players + progress files"]
    art["art.rs / sound.rs<br/>procedural pixel art + sfx"]
    ui["ui.rs<br/>buttons, text, panels"]

    main --> users & play & save
    play --> editor & program & world & levels & save
    editor --> program
    program --> world
    world --> levels
    users --> save
    play & editor & users & main --> ui & art
```

The **logic layer** (`world`, `program`, `levels`, `save`) doesn't draw anything.
That lets the tests and the in-game "try other worlds" check run whole programs
instantly with `program::simulate`. The **presentation layer** (`play`, `editor`,
`users`, `main`, `ui`, `art`, `sound`) uses an immediate-mode style: every frame
it reads input, updates, and redraws from the current state.

### Screens and the frame loop

`main.rs` holds a `Screen` enum (`Title`, `Users`, `Select`, `Play`). Each
frame:

1. It computes a scale factor and sets a camera, so all drawing happens in a
   fixed **1280×800** virtual space, letterboxed to the real window size (see
   [Presentation](#presentation)).
2. It builds a `Ui` value with the mouse position in virtual coordinates and
   this frame's clicks.
3. It calls the current screen. The screen returns a navigation result (for
   example `Nav::Goto(level)`), and `main` switches screens.

`Ui::clicked` *uses up* a click, so overlapping widgets can't both react. Popups
set `ui.enabled = false` while the screen behind them is drawn, then turn input
back on for the popup itself.

## The world model

`world.rs` holds the game's rules. The `World` struct is a grid of `Tile`s plus
the miner's position, facing direction, backpack (block count) and the
gems collected/total.

| Tile | Walkable | Mine → | Place on it → |
|---|---|---|---|
| Floor | ✓ | *error: nothing to mine* | Cobblestone (a wall!) |
| Stone / Cobblestone | | Floor, +1 block | *error* |
| Diamond ore | | Floor, +1 diamond | *error* |
| Water / Lava | *error: splash / ouch* | *error* | Bridge |
| Bridge | ✓ | *error* | *error* |
| Emerald | ✓ (picks it up) | *error* | *error* |
| Tree / Bedrock | | *error* | *error* |
| Portal | ✓ | *error* | *error* |

Actions: `Forward`, `TurnLeft`, `TurnRight`, `Mine`, `Place`.
`World::apply(action)` returns `Ok`, `Won`, or `Fail(Failure)`. Each `Failure`
has a kid-friendly message.

Some rules are deliberately strict, because strictness is what makes
conditionals necessary:

- **Mining empty floor is an error.** Otherwise "mine, forward" in a loop
  would solve every random-stone level without an `if`.
- **Placing on floor builds a wall.** Otherwise "place, forward" would solve
  every random-lava level.

**Win condition:** the miner stands on the portal *and* has collected every
diamond and emerald. Until then the portal is drawn dark. The win is checked
after every action, so a program stops as soon as it wins.

**Sensors** (`World::check(Cond)`): `path ahead/left/right` (is the tile
walkable), `stone ahead`, `diamond ahead`, `lava ahead`, `water ahead`,
`not at portal`. Everything outside the map counts as bedrock.

## The program model

```
blocks (tree)  ──compile──▶  bytecode  ──Vm::next──▶  one action at a time
                                         ▲
                             world sensors (World::check)
```

### Blocks

A program is two lists of `Node`s: `main` and `func` (one user-defined
function). Each node has a `kind` (forward, turn left/right, mine, place, call,
repeat, if, if/else, while), and also a `count` for repeat, a `cond` for
if/while, a `body`, an `els` (else part), and a unique `id`. The id is how the
running block gets highlighted.

### Compiler and VM

`Vm::new` flattens the tree into a small instruction set:

| Op | Meaning |
|---|---|
| `Act(action, id)` | Do an action (the VM pauses here and hands it to the game) |
| `Push(n)` / `Loop(top)` | Repeat: push a counter; at the end, count down and jump back |
| `JumpIfNot(cond, target, id)` | if / while test |
| `Jump(target)` | else-skip / loop back |
| `Call(id)` / `Ret` | Call the function / return |
| `Halt` | End of `main` |

`Vm::next(&world)` runs control instructions until it reaches the next
`Act`, then returns `Step::Act(action, block_id)`. The caller applies the
action, plays the animation, and asks for the next one. This **pause at every
action** design makes STEP, the highlight, animation pacing and instant
simulation all the same code path.

**Safety limits** (each gives a kid-readable error and outlines the block to
blame):

| Limit | Value | Catches |
|---|---|---|
| Actions per run | 600 | Walking in circles forever |
| Control steps without an action | 5,000 | `while` loops with nothing (or no movement) inside |
| Call depth | 100 | A function that calls itself forever |

### Rust code view

`Program::to_rust` prints the tree as Rust (`for _ in 0..3 { mine(); }`,
`while !at_portal() { ... }`, `fn my_function()`). The view colors keywords,
function calls and numbers.

### Save format

`Program::serialize` produces a compact token string, which is also the format
of each level's `solution` field:

```
[R4 F R F L ] F            repeat 4 { forward, right, forward, left } forward
[E3 M | P ]                if stone ahead { mine } else { place }
C C F / F L M R            main: call, call, forward   function: forward, left, mine, right
```

The tokens are `F L R M P C`, and `[R<n>`, `[I<c>`, `[E<c>`, `[W<c>` open a
repeat, if, if/else or while, where `<c>` is an index into `ALL_CONDS`. `|`
separates if/else branches, `]` closes a block, and `/` separates main from
the function.

## Dynamic levels

A level is **dynamic** (`Level::is_random`) if it has several `maps` or uses
random map symbols (`? ! * ~ & $ p`). `World::from_level(level, seed)` uses a
small xorshift RNG, so **a seed always produces the same world**. Seeds make
bugs reproducible and let the game show the exact world where something failed.

When a dynamic level is won, `play.rs` runs the program again with
`simulate` on 80 other seeds. If any of them fails, the win doesn't count. Bit
shows the failure message, and **"Show me that world"** loads that seed so the
kid can watch the bug happen. This is what makes the game "ungameable": a
program that wins by luck doesn't get the stars.

The **[dice] New world** button rerolls the seed. **RESET** keeps the same world,
so a kid can debug against a world that stays put.

## Curriculum and level design

Levels are data (`levels.rs`). They're grouped into six chapters. Each chapter
starts with an intro level (enforced by a test) and ramps up:

```
Sequences (9) → Loops (11) → Functions (8) → If/Else (14) 🎲 → While & algorithms (12) 🎲 → Master (6) 🎲
```

Inside a chapter, difficulty grows by: more steps → mixing two concepts →
irregular layouts that defeat the previous chapter's tricks → several random
features at once. Examples:

- **Loops:** one loop → two loops → loops with two instructions inside →
  stairs patterns → nested loops (pond, snake tunnel, lawnmower).
- **If/Else:** one maybe-stone → two → a whole corridor → lava → stone *or*
  water (two `if`s) → stone *or* lava (`if/else`) → forks (path sensing) →
  diamonds on both sides → `if` inside functions.
- **While:** random-length wall → random-length tunnels and lava → roads that
  only turn right → spirals → roads that turn both ways → mazes → mazes with
  hazards.

### Level design rules

These come from bugs found while building the levels.

1. **`par` equals the block count of `solution`**, and `solution` must stay
   within `max_blocks` and only use palette blocks and selectable conditions
   (all enforced by tests).
2. **`max_blocks` forces the concept.** Set it below the straight-line
   solution so that a loop or function is required. The functions levels use
   irregular spacing so that a plain loop can't beat a function.
3. **`start_blocks` covers the worst case.** If every random `!`/`*`/`&` turned
   out to be lava, the backpack must still have enough blocks.
4. **Never put a random tile right after a corner** on roads where the
   solution uses `path to left/right`. A stone or lava there makes the
   side-path check see a wall, and the miner turns the wrong way.
5. **In mazes solved with the right-hand rule, put obstacles only in straight
   corridors.** A wall-follower walks straight into obstacles there and deals
   with them. An obstacle next to a corner gets passed sideways, and the miner
   can circle forever ("Rubble Maze" originally failed this way).
6. **Win-on-arrival allows overshooting.** Because the game stops the moment
   the miner reaches the portal, `repeat 20 { forward }` wins any straight
   hallway that ends at the portal. To force `while`, put a *turn* after the
   variable-length part (as "How Far?" does).
7. **Level names are save keys.** Keep them unique and never rename one that
   kids may already have finished. If you must rename, update
   `LEGACY_ORDER`.

The maze layouts were generated by a search (the ignored test `find_mazes`).
It looks for mazes that the right-hand rule solves but the simpler
road-following programs don't, which guarantees the maze levels teach something
new.

## The block editor

Kids can work either way: **click-to-insert** (click a palette block and it
goes in at the yellow cursor) or **drag and drop** (drag from the palette, or
drag blocks inside the program to reorder them).

- **Slots.** Each frame, `layout()` walks the tree and makes a list of rows
  (block, `else`, end cap) and **slots**: the gaps where a block could go.
  Every slot has a unique y position, so both clicks and drops simply pick the
  nearest slot. Empty bodies get their own visible gap.
- **Press, then decide.** Pressing on a palette block or a program block
  starts a `Drag` that isn't active yet. If the mouse moves more than 6 pixels,
  it becomes a real drag. If the button is let go first, it counts as a click
  (insert at the cursor, or move the cursor). So clicking keeps working exactly
  as before, and younger kids never *have* to drag.
- **While dragging:** a ghost block follows the mouse, a green line marks the
  drop slot, the original block is dimmed, and the list scrolls when you drag
  near its top or bottom edge. Slots inside the dragged block are skipped, so a
  container can't be dropped into itself. Hovering a MAIN/FUNCTION tab switches
  to it, so blocks can move between them. Dropping a program block outside the
  program panel deletes it (the ghost turns red and says "let go to delete").
- **Moving safely.** `Editor::move_node` first inserts a placeholder node at the
  target slot, then removes the dragged block from wherever it was, then
  replaces the placeholder with it. Removing a block shifts the positions after
  it, which would make a target path computed beforehand point at the wrong
  spot. The placeholder doesn't care. Unit tests in `editor.rs` cover moving
  down in the same list, into and out of containers and else parts, between
  tabs, and moving a container with its children.
- Inline controls (the `x`, `−`/`+`, the condition label) are handled before a
  press can start a drag, and their edits are collected as an `Edit` value and
  applied after drawing, so the tree is never changed while it's being read.

### Undo and redo

`Play` keeps two stacks of whole-program snapshots (programs are tiny, so
copies are cheap). At the start of every frame it remembers the program. At
the end, if the program changed (checked by comparing the save-format string),
the old version goes onto the undo stack and the redo stack is cleared. This
catches every kind of edit in one place, with no per-action bookkeeping: adds,
drags, deletes, count and condition changes, Clear, and the parent "load
solution" shortcut. Undo and Redo swap versions between the stacks and set a
flag so the swap itself isn't recorded. History holds 100 steps and lasts while
the level is open.

**Following the running block:** while a program runs, the editor switches to
the tab (MAIN or FUNCTION) that holds the highlighted block. It does this
**only when the highlighted block changes**. An earlier version switched every
frame, which locked kids out of the MAIN tab after an error inside the function.

## Presentation

- **Fixed virtual canvas.** Everything is laid out in 1280×800 and scaled with a
  `Camera2D` plus a letterbox viewport. Any window size, tiling window managers
  and odd aspect ratios all work. Mouse positions are converted back into
  virtual coordinates.
- **Sharp text when scaled.** `ui::text` renders fonts at the real on-screen
  pixel size (`font_size × scale`, `font_scale = 1/scale`), so text stays
  sharp at any window size.
- **Procedural art** (`art.rs`). Every texture is a 16×16 image built by a
  function of (x, y): hashed noise for grass and stone, sine patterns for
  animated water, lava and portal frames, and ASCII pixel-art for the miner,
  Bit, the pickaxe and the app icon. The miner is generated in eight shirt
  colors, one per player choice.
- **Procedural sound** (`sound.rs`). Square, triangle and noise waves with a
  short fade-in/out are written into in-memory WAV files at startup.
- **Animation.** Each action makes an `Anim` (move, turn, mine, place, bump,
  hazard). Effects such as crack overlays, particles and "+1 diamond" text fire
  partway through. The run speed setting (Slow/Normal/Fast/Turbo) sets the
  animation length, which is also the time between steps.

## Players and saving

Files live in `$XDG_DATA_HOME/block-coders/` (usually
`~/.local/share/block-coders/`):

```
users.txt        id;shirt;name           one line per player
user-<id>.txt    level name;stars;program  one line per level
```

- **Saved by level name, not number**, so levels can be inserted or reordered
  freely. Older saves that numbered levels 0–19 are converted on load using
  `LEGACY_ORDER`. An even older single-player `progress.txt` becomes "Player 1".
- **Unlocking:** level *i* is open if it's level 1, or if *i* ≤ (furthest level
  with stars) + 1. So beating a level opens everything before it, and new
  levels added early in the list never lock out a player who already went
  further.
- **Writes happen on every program edit and every win.** The files are tiny, and
  this means closing the window never loses work.
- Players have up to 12 characters (letters, digits, space, `-`, `_`), and there
  can be at most 6 players. "Start over" rewrites the player's file empty, and
  "Delete" removes it. Both ask for confirmation.

## Packaging

`packaging/make-package.sh` runs the tests, builds the release binary (with
symbols stripped), checks portability, writes the icon (`block-coders
--write-icon`), and creates a tarball with `install.sh`/`uninstall.sh` and a
`.desktop` entry.

- **Portability limit is glibc 2.34.** The script reads the binary's required
  glibc symbol versions (`objdump -T`) and fails the build if anything needs
  more. Lesson learned: on a very new distro, a single `f32::atan2` call pulled
  in `atan2f@GLIBC_2.43`, which made the binary unusable on Ubuntu and Debian.
  It now uses f64 `atan2`.
- macroquad loads OpenGL/X11/Wayland at runtime (`dlopen`). The only hard link
  dependency is `libasound.so.2`. `install.sh` warns if it's missing.
- The window class is set to `block-coders`, matching `StartupWMClass` in the
  `.desktop` file, so desktops show the right icon.
- A user install needs no root (`~/.local/bin` and `~/.local/share`).
  `--system` installs to `/usr/local`.

## Testing

`cargo test` (well under a second):

| Test | Guarantees |
|---|---|
| `every_solution_wins_in_every_world` | Every level's `solution` wins in 300 random worlds, and `par` matches its block count |
| `dynamic_levels_cannot_be_hardcoded` | For each dynamic level, replaying the exact moves that won one world fails in some other world |
| `shortcuts_fail_somewhere` | Specific tempting shortcuts fail (e.g. "mine everything", road-following on a maze) |
| `solutions_only_use_available_conditions` | Solutions only use conditions the kid can pick in that level |
| `level_list_is_sane` | 60 levels, unique names (save keys), chapters in order, intro on each chapter's first level, hint and lesson present |
| `maps_are_well_formed` | Rectangular maps, exactly one start, fits the screen |
| `serialize_round_trips` | The save format round-trips |
| `editor::tests::*` | Moving blocks: reordering, into/out of containers and else parts, between tabs, with children, and unique ids afterwards |
| `players_can_be_created_reset_and_deleted` | Player files, legacy migration, unlock rules |
| `empty_while_is_caught` | The VM's infinite-loop guard |

**Screenshot autopilot** (development only). Setting `BC_SHOT=<dir>` makes the
game drive itself: the title, the player screens, a level (`BC_LEVEL=<index>`)
loaded with its solution (or with `BC_PROGRAM=<program string>`), the Rust view,
a run, and the result. It saves a PNG at each step. `BC_CLICK=x,y` injects a
click after the run, which was used to reproduce and verify the tab-switching
bug. `BC_INPUT="x,y,act;..."` plays one scripted mouse step per frame (`move`,
`down`, `up`, `shot`, `dump`), which tests drag-and-drop and undo with real
input handling. `dump` prints the program after each step. Set `XDG_DATA_HOME` to a temporary folder so test runs never touch real
saves.

## Key decisions

| Decision | Alternatives | Why |
|---|---|---|
| macroquad | Bevy, ggez, egui | Small, fast to compile, simple immediate-mode 2D, built-in audio, and it builds for the web. Bevy's ECS is overkill for a grid game. |
| Click-to-insert *and* drag-and-drop | Only one of them | Clicking is easier for young kids and trackpads. Dragging is faster and is what Scratch users expect. Both use the same slot system. |
| Undo by whole-program snapshots | An undo entry per edit type | One generic check at the end of the frame can't miss a kind of edit, and programs are tiny. |
| Fixed 1280×800 virtual canvas | Responsive layout | One layout to design and test, and it works at any window size via letterboxing. |
| Procedural assets | Image/sound files | A single self-contained binary, nothing to lose or load, and the art is itself code kids can explore. |
| Bytecode VM that pauses at actions | Walking the tree with recursion | Stepping, highlighting, animation and instant simulation all share one path. Infinite loops are easy to detect. |
| One function (no parameters) | Several named functions | Enough to teach reuse at this age. More would need naming UI. |
| Strict rules (mining air fails, placing on floor walls you in) | Forgiving rules | Without them, random levels could be solved without conditionals. |
| Win-on-arrival | Win only when the program ends | Feels better ("I made it!") and avoids "why didn't it win?" confusion. Levels work around the overshoot loophole (rule 6). |
| Verify wins on 80 other seeds | Trust a single run | Without it, kids could pass a dynamic level by luck and learn the wrong lesson. |
| Plain-text save files | JSON/serde | Zero dependencies, human-readable, easy for a parent to back up or inspect. |

## Limitations and future work

- **Linux only so far.** macroquad supports Windows and macOS, but no packages
  exist yet. A GitHub Actions matrix could build them.
- **Web version.** Planned; see [WEB.md](WEB.md). The main change is moving
  saves to browser storage.
- **Touch screens.** Taps work as clicks, but typing a player name needs a
  keyboard, and the 1280×800 layout is small on phones.
- **Accessibility.** No screen reader support. Colors are bold and paired with
  icons and text labels, but they haven't been checked for color blindness.
- **One function, no parameters, no variables.** Possible next concepts:
  counters ("repeat until you have 3 blocks"), parameters ("dig(n)"), or a
  second function.
- **Level authoring needs Rust.** Levels are compiled in. Loading levels from
  text files would let kids make and share levels without rebuilding.
