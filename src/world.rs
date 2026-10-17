//! The block world: tiles, the miner, and the rules for every action.

use crate::levels::Level;
use crate::program::{Action, Cond};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tile {
    Floor,
    Stone,
    Cobble,
    Ore,
    Tree,
    Bedrock,
    Water,
    Lava,
    Bridge,
    Emerald,
    Portal,
}

impl Tile {
    pub fn walkable(self) -> bool {
        matches!(self, Tile::Floor | Tile::Bridge | Tile::Emerald | Tile::Portal)
    }
}

/// What the ground looks like in a level (purely cosmetic).
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Floor {
    Grass,
    Dirt,
    Sand,
    Deepslate,
    Nether,
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Right,
    Down,
    Left,
}

impl Dir {
    pub fn delta(self) -> (i32, i32) {
        match self {
            Dir::Up => (0, -1),
            Dir::Right => (1, 0),
            Dir::Down => (0, 1),
            Dir::Left => (-1, 0),
        }
    }
    pub fn left(self) -> Dir {
        match self {
            Dir::Up => Dir::Left,
            Dir::Left => Dir::Down,
            Dir::Down => Dir::Right,
            Dir::Right => Dir::Up,
        }
    }
    pub fn right(self) -> Dir {
        self.left().left().left()
    }
    /// Clockwise angle in degrees, with Up = 0.
    pub fn angle(self) -> f32 {
        match self {
            Dir::Up => 0.0,
            Dir::Right => 90.0,
            Dir::Down => 180.0,
            Dir::Left => 270.0,
        }
    }
}

/// Why an action went wrong. Each one has a kid-friendly explanation.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Failure {
    Bonk(Tile),
    Hazard(Tile),
    NothingToMine,
    CantMine(Tile),
    NoBlocks,
    CantPlace(Tile),
}

impl Failure {
    pub fn message(self) -> String {
        let s = match self {
            Failure::Bonk(Tile::Stone | Tile::Cobble) => {
                "Bonk! There's stone in the way. Try MINE first."
            }
            Failure::Bonk(Tile::Ore) => "Bonk! Diamond ore is in the way. MINE it to collect it!",
            Failure::Bonk(Tile::Tree) => "Bonk! You walked into a tree.",
            Failure::Bonk(_) => "Bonk! That's a bedrock wall - nobody can go through bedrock.",
            Failure::Hazard(Tile::Lava) => {
                "Ouch, lava! Never step in lava. PLACE a block on it first to make a bridge."
            }
            Failure::Hazard(_) => {
                "Splash! You can't swim here. PLACE a block on the water to build a bridge."
            }
            Failure::NothingToMine => {
                "Whoosh! Your pickaxe swung at thin air. There was nothing in front of you to mine."
            }
            Failure::CantMine(Tile::Tree) => {
                "Trees are too tough for your pickaxe. It only mines stone and diamond ore."
            }
            Failure::CantMine(Tile::Bedrock) => "Bedrock is unbreakable! Nobody can mine it.",
            Failure::CantMine(Tile::Portal) => "You can't mine the portal - walk into it instead!",
            Failure::CantMine(Tile::Bridge) => "That's a bridge! Better not break it.",
            Failure::CantMine(_) => "You can't mine water or lava!",
            Failure::NoBlocks => {
                "Your backpack is empty - there are no blocks to place! Mine some stone first."
            }
            Failure::CantPlace(_) => "You can't place a block there.",
        };
        s.to_string()
    }
}

pub enum Outcome {
    Ok,
    Won,
    Fail(Failure),
}

/// A tiny deterministic random number generator (xorshift), so a seed
/// always produces the same world.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Rng {
        let mut r = Rng(seed ^ 0x9E37_79B9_7F4A_7C15);
        r.next();
        r
    }
    pub fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    pub fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
    pub fn coin(&mut self) -> bool {
        self.next() & 1 == 1
    }
}

#[derive(Clone, Debug)]
pub struct World {
    pub w: i32,
    pub h: i32,
    pub tiles: Vec<Tile>,
    pub pos: (i32, i32),
    pub dir: Dir,
    pub blocks: u32,
    pub diamonds: u32,
    pub diamonds_total: u32,
    pub emeralds: u32,
    pub emeralds_total: u32,
    pub has_portal: bool,
}

impl World {
    /// Build a world from a level map. Some map symbols are random, so the
    /// `seed` picks which variation you get.
    ///
    /// Map symbols:
    /// `.` floor  `#` stone  `D` diamond ore  `T` tree  `B` bedrock
    /// `W` water  `L` lava   `E` emerald      `P` portal  `C` cobblestone
    /// `^ > v <` start position and direction
    /// `?` stone or floor   `!` lava or floor   `*` stone or lava
    /// `$` ore or floor     `~` water or floor   `&` stone, lava or floor
    /// `p` possible portal spot (one is chosen)
    pub fn from_level(level: &Level, seed: u64) -> World {
        let mut rng = Rng::new(seed);
        let map = level.maps[rng.below(level.maps.len())];
        let h = map.len() as i32;
        let w = map[0].len() as i32;
        let mut tiles = Vec::with_capacity((w * h) as usize);
        let mut pos = (0, 0);
        let mut dir = Dir::Right;
        let mut portal_spots = Vec::new();
        for (y, row) in map.iter().enumerate() {
            for (x, ch) in row.chars().enumerate() {
                let t = match ch {
                    '.' => Tile::Floor,
                    '#' => Tile::Stone,
                    'C' => Tile::Cobble,
                    'D' => Tile::Ore,
                    'T' => Tile::Tree,
                    'B' => Tile::Bedrock,
                    'W' => Tile::Water,
                    'L' => Tile::Lava,
                    'E' => Tile::Emerald,
                    'P' => Tile::Portal,
                    '^' | '>' | 'v' | '<' => {
                        pos = (x as i32, y as i32);
                        dir = match ch {
                            '^' => Dir::Up,
                            '>' => Dir::Right,
                            'v' => Dir::Down,
                            _ => Dir::Left,
                        };
                        Tile::Floor
                    }
                    '?' => if rng.coin() { Tile::Stone } else { Tile::Floor },
                    '!' => if rng.coin() { Tile::Lava } else { Tile::Floor },
                    '*' => if rng.coin() { Tile::Stone } else { Tile::Lava },
                    '$' => if rng.coin() { Tile::Ore } else { Tile::Floor },
                    '~' => if rng.coin() { Tile::Water } else { Tile::Floor },
                    '&' => [Tile::Stone, Tile::Lava, Tile::Floor][rng.below(3)],
                    'p' => {
                        portal_spots.push(tiles.len());
                        Tile::Floor
                    }
                    other => panic!("unknown map symbol {other:?} in level {}", level.name),
                };
                tiles.push(t);
            }
        }
        if !portal_spots.is_empty() {
            tiles[portal_spots[rng.below(portal_spots.len())]] = Tile::Portal;
        }
        let count = |t: Tile| tiles.iter().filter(|&&x| x == t).count() as u32;
        World {
            w,
            h,
            diamonds_total: count(Tile::Ore),
            emeralds_total: count(Tile::Emerald),
            has_portal: count(Tile::Portal) > 0,
            tiles,
            pos,
            dir,
            blocks: level.start_blocks,
            diamonds: 0,
            emeralds: 0,
        }
    }

    pub fn tile(&self, (x, y): (i32, i32)) -> Tile {
        if x < 0 || y < 0 || x >= self.w || y >= self.h {
            Tile::Bedrock
        } else {
            self.tiles[(y * self.w + x) as usize]
        }
    }

    fn set(&mut self, (x, y): (i32, i32), t: Tile) {
        self.tiles[(y * self.w + x) as usize] = t;
    }

    fn step(&self, d: Dir) -> (i32, i32) {
        let (dx, dy) = d.delta();
        (self.pos.0 + dx, self.pos.1 + dy)
    }

    pub fn front(&self) -> (i32, i32) {
        self.step(self.dir)
    }

    pub fn check(&self, c: Cond) -> bool {
        let ahead = self.tile(self.front());
        match c {
            Cond::PathAhead => ahead.walkable(),
            Cond::PathLeft => self.tile(self.step(self.dir.left())).walkable(),
            Cond::PathRight => self.tile(self.step(self.dir.right())).walkable(),
            Cond::StoneAhead => matches!(ahead, Tile::Stone | Tile::Cobble),
            Cond::DiamondAhead => ahead == Tile::Ore,
            Cond::LavaAhead => ahead == Tile::Lava,
            Cond::WaterAhead => ahead == Tile::Water,
            Cond::NotAtPortal => self.tile(self.pos) != Tile::Portal,
        }
    }

    pub fn collected_all(&self) -> bool {
        self.diamonds == self.diamonds_total && self.emeralds == self.emeralds_total
    }

    pub fn won(&self) -> bool {
        if self.has_portal {
            self.tile(self.pos) == Tile::Portal && self.collected_all()
        } else {
            self.collected_all()
        }
    }

    pub fn apply(&mut self, a: Action) -> Outcome {
        let f = self.front();
        let ahead = self.tile(f);
        match a {
            Action::Forward => {
                if ahead.walkable() {
                    self.pos = f;
                    if ahead == Tile::Emerald {
                        self.set(f, Tile::Floor);
                        self.emeralds += 1;
                    }
                } else if matches!(ahead, Tile::Water | Tile::Lava) {
                    return Outcome::Fail(Failure::Hazard(ahead));
                } else {
                    return Outcome::Fail(Failure::Bonk(ahead));
                }
            }
            Action::TurnLeft => self.dir = self.dir.left(),
            Action::TurnRight => self.dir = self.dir.right(),
            Action::Mine => match ahead {
                Tile::Stone | Tile::Cobble => {
                    self.set(f, Tile::Floor);
                    self.blocks += 1;
                }
                Tile::Ore => {
                    self.set(f, Tile::Floor);
                    self.diamonds += 1;
                }
                Tile::Floor | Tile::Emerald => return Outcome::Fail(Failure::NothingToMine),
                other => return Outcome::Fail(Failure::CantMine(other)),
            },
            Action::Place => {
                if self.blocks == 0 {
                    return Outcome::Fail(Failure::NoBlocks);
                }
                let new = match ahead {
                    Tile::Water | Tile::Lava => Tile::Bridge,
                    Tile::Floor => Tile::Cobble,
                    other => return Outcome::Fail(Failure::CantPlace(other)),
                };
                self.set(f, new);
                self.blocks -= 1;
            }
        }
        if self.won() { Outcome::Won } else { Outcome::Ok }
    }

    /// Explains what is still missing when a program ends without winning.
    pub fn unfinished_message(&self) -> String {
        let mut parts = Vec::new();
        let d = self.diamonds_total - self.diamonds;
        let e = self.emeralds_total - self.emeralds;
        if d > 0 {
            parts.push(format!("{d} more diamond{}", if d == 1 { "" } else { "s" }));
        }
        if e > 0 {
            parts.push(format!("{e} more emerald{}", if e == 1 { "" } else { "s" }));
        }
        let mut msg = String::from("Your program finished, but ");
        let on_portal = self.tile(self.pos) == Tile::Portal;
        if self.has_portal && !on_portal {
            msg.push_str("the miner isn't standing in the portal yet.");
            if !parts.is_empty() {
                msg.push_str(&format!(" You also still need {}.", parts.join(" and ")));
            }
        } else {
            msg.push_str(&format!("you still need {}.", parts.join(" and ")));
        }
        msg.push_str(" Add more blocks to finish the job!");
        msg
    }
}
