# Block Coders

A blocky, Minecraft-style coding puzzle game for kids aged about 8–12, written in Rust.

Kids snap together code blocks (forward, turn, mine, place, repeat, if/else,
while, functions) to guide a miner through 60 levels. They collect diamonds and
emeralds, bridge lava, and reach the portal. Each world teaches one big
programming idea.

| World | Levels | Concept |
|---|---|---|
| 1. Grassy Plains | 1–9 | Sequences: order of steps, turning, mining, placing blocks |
| 2. Loop Caves | 10–20 | `repeat` loops, patterns, nested loops |
| 3. Function Forest | 21–28 | Functions: write once, call many times |
| 4. Decision Desert | 29–42 | `if` and `if / else`. Every level here changes each run 🎲 |
| 5. The Deep Dark | 43–54 | `while` loops and algorithms (road following, the right-hand maze rule) 🎲 |
| 6. Nether Fortress | 55–60 | Master challenges that mix everything 🎲 |

Each world starts gently and gets harder. Levels marked with a die on the level
screen (32 of the 60) change every time you press RUN, so kids can't memorize a
path. Their program has to look at the world and decide.

## Running it

```sh
cargo run --release
```

The first build downloads `macroquad`, the only dependency. The window can be
resized and the game scales to fit.

## Installing on other Linux computers

```sh
packaging/make-package.sh
```

This runs the tests, builds an optimized binary, checks that it will run on
older distros, and writes `dist/block-coders-<version>-linux-x86_64.tar.gz`
(under 1 MB). Copy that file to the other computer, unpack it, and run
`./install.sh`. Block Coders then shows up in the app menu with its own icon.
No root is needed (`sudo ./install.sh --system` installs it for all users). The
`README.txt` inside the package explains how to install and uninstall.

It works on any 64-bit Linux with glibc 2.34+: Ubuntu 22.04+, Debian 12+,
Fedora 35+, Mint 21+, and Arch-based distros. To move a kid's progress to another
computer, copy `~/.local/share/block-coders/`.

For the plan to make a web version, see [docs/WEB.md](docs/WEB.md).

## Features for learning

- **Bit the robot** explains each new idea when it first appears. The **Hint**
  button gives a nudge when a kid is stuck.
- **STEP** runs one block at a time, and the block that is running lights up.
  This is how real developers debug.
- **Kid-friendly errors.** When something goes wrong (walking into lava,
  mining air, an infinite loop), the block that caused it gets a red outline and
  Bit explains what happened.
- **Stars for short programs.** You get 3 stars for matching the best block
  count, which encourages loops and functions.
- **Some levels change each run** (levels 14+). After a win, the game quietly
  tests the program in many other versions of the level. If a hard-coded
  solution would fail in one of them, the game says so and can show that world.
  So the program really has to check its surroundings with `if` and `while`.
- **"Show Rust code"** shows the kid's block program as real Rust, with syntax
  colors. This bridges block coding and typed code.
- **Players:** up to 6 kids can each have their own player, with a name, a
  shirt color for their miner, and their own stars and saved programs. Pick a
  player on the "Who's playing?" screen. **Start over** wipes that player back
  to level 1, and **Delete** removes the player. Both ask for confirmation first.
- Progress saves automatically to `~/.local/share/block-coders/`: the player
  list is in `users.txt`, and each player's progress is in `user-<id>.txt`. A save from
  the older single-player version is kept as "Player 1".

## Controls

- Click a block in **BLOCKS** to insert it at the yellow cursor. Click between
  blocks in the program to move the cursor.
- Use **x** or right-click on a block to delete it. Backspace deletes the block
  above the cursor.
- Click `-`/`+` on a repeat block to change its count. Click the white label on
  an if/while block to change what it checks.
- **Enter** runs the program.

## Parent shortcuts

- **Ctrl+U** on the level select screen unlocks all levels.
- **Ctrl+Shift+S** inside a level loads a working solution. Use it if you need
  to help.

## For curious kids (and parents): how it's built

| File | What it does |
|---|---|
| `src/levels.rs` | All 60 levels as ASCII maps. Easy to edit or add your own! |
| `src/world.rs` | The game rules (what happens on forward, mine, place...) |
| `src/program.rs` | Code blocks, plus a tiny compiler and virtual machine that runs them |
| `src/editor.rs` | The block editor UI |
| `src/users.rs`, `src/save.rs` | The player list, and saving each player's progress |
| `src/play.rs` | The game screen, animations and popups |
| `src/art.rs` | All pixel art, drawn from code (there are no image files) |
| `src/sound.rs` | Sound effects, synthesized from code (there are no sound files) |

To make a new level, copy one in `src/levels.rs` and edit its map. The comment
on `World::from_level` in `src/world.rs` lists every map symbol. Then run
`cargo test`. The tests check that every level's `solution` really wins in every
random version of the level. For dynamic levels, they also check that copying
the exact moves from one version fails in another, which proves kids have to use
`if`/`while`. Levels are saved by name, so give each new level a unique name.
