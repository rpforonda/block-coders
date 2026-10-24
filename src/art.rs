//! All the pixel art, generated in code (no image files needed).

use crate::levels::BossKind;
use crate::world::{Dir, Floor, Tile};
use macroquad::prelude::*;

fn hash(x: i32, y: i32, s: u32) -> f32 {
    let mut h = (x as u32).wrapping_mul(374_761_393)
        ^ (y as u32).wrapping_mul(668_265_263)
        ^ s.wrapping_mul(2_246_822_519);
    h = (h ^ (h >> 13)).wrapping_mul(1_274_126_177);
    h ^= h >> 16;
    (h & 0xffff) as f32 / 65535.0
}

pub fn rgb(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

pub fn shade(c: Color, f: f32) -> Color {
    Color::new((c.r * f).min(1.0), (c.g * f).min(1.0), (c.b * f).min(1.0), c.a)
}

fn mix(a: Color, b: Color, t: f32) -> Color {
    let t = t.clamp(0.0, 1.0);
    Color::new(a.r + (b.r - a.r) * t, a.g + (b.g - a.g) * t, a.b + (b.b - a.b) * t, 1.0)
}

fn tex(f: impl Fn(i32, i32) -> Color) -> Texture2D {
    let mut img = Image::gen_image_color(16, 16, BLANK);
    for y in 0..16 {
        for x in 0..16 {
            img.set_pixel(x as u32, y as u32, f(x, y));
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

/// Builds a texture from ASCII pixel art, one char per pixel.
fn sprite(rows: &[&str], palette: &[(char, Color)]) -> Texture2D {
    let w = rows[0].len() as u16;
    let mut img = Image::gen_image_color(w, rows.len() as u16, BLANK);
    for (y, row) in rows.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some((_, c)) = palette.iter().find(|(k, _)| *k == ch) {
                img.set_pixel(x as u32, y as u32, *c);
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

fn noisy(base: Color, seed: u32) -> impl Fn(i32, i32) -> Color {
    move |x, y| {
        let n = hash(x, y, seed);
        if n < 0.13 {
            shade(base, 0.8)
        } else if n > 0.9 {
            shade(base, 1.13)
        } else {
            shade(base, 0.93 + 0.12 * hash(x, y, seed + 1))
        }
    }
}

fn gem(x: i32, y: i32, light: Color, mid: Color, dark: Color, outline: Color) -> Color {
    let d = (x as f32 - 7.5).abs() + (y as f32 - 7.5).abs() * 0.85;
    if d > 6.6 {
        BLANK
    } else if d > 5.5 {
        outline
    } else if (x, y) == (5, 5) || (x, y) == (6, 4) || (x, y) == (5, 4) {
        WHITE
    } else if x + y < 15 {
        if x + y < 11 { light } else { mid }
    } else {
        dark
    }
}

/// A heart for the health meter in boss fights. Half hearts draw its left half.
pub const HEART: [&str; 8] = [
    ".kk...kk.",
    "kRRk.kRRk",
    "kRWRkRRRk",
    "kRRRRRRRk",
    ".kRRRRRk.",
    "..kRRRk..",
    "...kRk...",
    "....k....",
];

/// Pixel art for a sprite, and the colour of each letter in it.
pub type PixelArt = ([&'static str; 16], &'static [(char, (u8, u8, u8))]);

/// The six bosses, in `BossKind` order.
pub const BOSS_ART: [PixelArt; 6] = [
    // Mossback: a huge mossy turtle.
    (
        [
            "................",
            "....oooooooo....",
            "..oosmssmsmsoo..",
            ".osmssmsssmssmo.",
            ".ossmsssmssmsso.",
            "osmssmssmsssmsso",
            "oooooooooooooooo",
            ".yyyyyyyyyyyyyy.",
            "HH..HHHHHHHH..HH",
            "HH.HwwHHHHwwH.HH",
            "HH.HwkHHHHwkH.HH",
            "...HHHHHHHHHH...",
            "....HHrrrrHH....",
            ".....HHHHHH.....",
            "................",
            "................",
        ],
        &[
            ('o', (30, 70, 30)),
            ('s', (60, 110, 45)),
            ('m', (110, 170, 70)),
            ('y', (200, 190, 120)),
            ('H', (110, 130, 70)),
            ('w', (255, 255, 255)),
            ('k', (15, 15, 15)),
            ('r', (150, 40, 40)),
        ],
    ),
    // Echo Bat: wings spread, glowing eyes.
    (
        [
            "................",
            "...e........e...",
            "...pe......ep...",
            "...pppppppppp...",
            "d..ppyyppyypp..d",
            "dd.pppppppppp.dd",
            "dddpppfkkfpppddd",
            "dddddppppppddddd",
            ".dddd.pppp.dddd.",
            "..ddd..pp..ddd..",
            "...d..p..p..d...",
            "................",
            "................",
            "................",
            "................",
            "................",
        ],
        &[
            ('p', (95, 55, 135)),
            ('d', (55, 30, 85)),
            ('e', (210, 130, 180)),
            ('y', (255, 230, 90)),
            ('k', (15, 10, 20)),
            ('f', (245, 245, 245)),
        ],
    ),
    // Thornroot: an angry old tree with thorns.
    (
        [
            "....gggggggg....",
            "..ggGggGgggGgg..",
            ".gGgggGggGggggg.",
            ".ggggggggggggGg.",
            "..gggbbbbbbggg..",
            "t...bBbbbbBb...t",
            ".t..byybbyyb..t.",
            "..tbbbbbbbbbbt..",
            "...bbbkkkkbbb...",
            "....bbbbbbbb....",
            "....bBbbbbBb....",
            "...bbb.bb.bbb...",
            "..bb...bb...bb..",
            ".bb....bb....bb.",
            "................",
            "................",
        ],
        &[
            ('g', (60, 140, 50)),
            ('G', (110, 190, 75)),
            ('b', (115, 78, 42)),
            ('B', (70, 45, 25)),
            ('y', (240, 220, 90)),
            ('k', (20, 12, 8)),
            ('t', (215, 205, 160)),
        ],
    ),
    // Dune Scorpion: claws out, stinger raised.
    (
        [
            "......ttt.......",
            "......sSs.......",
            ".......s........",
            "SS.....s......SS",
            "sSs....s.....sSs",
            ".sss..sss...sss.",
            "...sssSSSSsss...",
            "..ssSrsSSsrSss..",
            ".s.sSSSSSSSSs.s.",
            "s..ssSSSSSSss..s",
            ".s..ssSSSSss..s.",
            "s....ssssss....s",
            ".......ss.......",
            "................",
            "................",
            "................",
        ],
        &[
            ('s', (215, 175, 95)),
            ('S', (150, 110, 50)),
            ('r', (230, 40, 40)),
            ('t', (90, 55, 25)),
        ],
    ),
    // The Gloom: a shadow with glowing eyes.
    (
        [
            "................",
            ".....dddddd.....",
            "...dddddddddd...",
            "..dddddddddddd..",
            ".ddddeeddeedddd.",
            ".ddddeEddEedddd.",
            ".dddddddddddddd.",
            "ddddddmmmmdddddd",
            "dddddmDDDDmddddd",
            "ddddddmmmmdddddd",
            "dddddddddddddddd",
            "dddDdddDddddDddd",
            "dd.ddd.dd.ddd.dd",
            "d..d...dd...d..d",
            "................",
            "................",
        ],
        &[
            ('d', (40, 34, 62)),
            ('D', (18, 16, 30)),
            ('e', (80, 240, 220)),
            ('E', (225, 255, 250)),
            ('m', (100, 45, 130)),
        ],
    ),
    // Abacus Golem: a stone golem with counting beads on its chest.
    (
        [
            "....gggggggg....",
            "....gyyggyyg....",
            "....gggkkggg....",
            "..GGGGGGGGGGGG..",
            "ggGGGGGGGGGGGGgg",
            "gggGrrwwwbbwGggg",
            "gggGwooowrrwGggg",
            "gggGbbwwwwooGggg",
            "gggGGGGGGGGGGggg",
            "..gggggggggggg..",
            "..ggg......ggg..",
            "..ggg......ggg..",
            ".GGGG......GGGG.",
            "................",
            "................",
            "................",
        ],
        &[
            ('g', (135, 135, 140)),
            ('G', (85, 85, 92)),
            ('y', (255, 200, 60)),
            ('k', (20, 20, 20)),
            ('r', (220, 60, 60)),
            ('b', (60, 120, 230)),
            ('o', (240, 200, 50)),
            ('w', (205, 205, 205)),
        ],
    ),
];

/// Shirt colours players can pick for their miner.
pub const SHIRTS: [(u8, u8, u8); 8] = [
    (0, 168, 168),
    (210, 50, 50),
    (60, 160, 60),
    (230, 160, 20),
    (150, 80, 210),
    (230, 100, 170),
    (50, 90, 200),
    (240, 240, 240),
];

const BIT: &[&str] = &[
    ".....RR.....", ".....AA.....", "..GGGGGGGG..", ".GLLLLLLLLG.", ".GLEELLEELG.", ".GLEELLEELG.",
    ".GLLLLLLLLG.", ".GLMLLLLMLG.", ".GLLMMMMLLG.", ".GLLLLLLLLG.", "..GGGGGGGG..", "...gg..gg...",
];

fn bit_palette() -> [(char, Color); 7] {
    [
        ('R', rgb(255, 70, 70)),
        ('A', rgb(160, 160, 170)),
        ('G', rgb(120, 125, 140)),
        ('g', rgb(80, 85, 100)),
        ('L', rgb(120, 230, 220)),
        ('E', rgb(20, 40, 60)),
        ('M', rgb(20, 90, 100)),
    ]
}

/// The app icon (Bit on a grass block) as RGBA bytes, `size` x `size`.
/// Pure CPU work, so it can run before any window exists.
pub fn icon_rgba(size: usize) -> Vec<u8> {
    let grass = noisy(rgb(98, 163, 58), 1);
    let pal = bit_palette();
    let mut px = [[BLANK; 16]; 16];
    for (y, row) in px.iter_mut().enumerate() {
        for (x, p) in row.iter_mut().enumerate() {
            let edge = x == 0 || y == 0 || x == 15 || y == 15;
            *p = if edge { rgb(40, 70, 25) } else { grass(x as i32, y as i32) };
        }
    }
    for (y, row) in BIT.iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some((_, c)) = pal.iter().find(|(k, _)| *k == ch) {
                px[y + 3][x + 2] = *c;
            }
        }
    }
    let mut out = Vec::with_capacity(size * size * 4);
    for y in 0..size {
        for x in 0..size {
            let c = px[y * 16 / size][x * 16 / size];
            out.extend([c.r, c.g, c.b, c.a].map(|v| (v * 255.0).round() as u8));
        }
    }
    out
}

pub struct Art {
    grass: Texture2D,
    dirt: Texture2D,
    sand: Texture2D,
    deepslate: Texture2D,
    nether: Texture2D,
    stone: Texture2D,
    pub cobble: Texture2D,
    ore: Texture2D,
    tree: Texture2D,
    cactus: Texture2D,
    bedrock: Texture2D,
    water: Vec<Texture2D>,
    lava: Vec<Texture2D>,
    bridge: Texture2D,
    pub emerald: Texture2D,
    pub diamond: Texture2D,
    pub crystal: Texture2D,
    bosses: Vec<Texture2D>,
    pub heart_full: Texture2D,
    pub heart_empty: Texture2D,
    portal: Vec<Texture2D>,
    portal_off: Texture2D,
    player: Vec<[Texture2D; 4]>,
    pub bit: Texture2D,
    pub pickaxe: Texture2D,
}

impl Art {
    pub fn new() -> Art {
        let stone_base = rgb(127, 127, 127);
        let stone_noise = noisy(stone_base, 10);
        // Solid blocks get a dark rim so they stand out from the floor.
        let stone_px = move |x: i32, y: i32| {
            let edge = x == 0 || y == 0 || x == 15 || y == 15;
            if edge { shade(stone_noise(x, y), 0.6) } else { stone_noise(x, y) }
        };
        let water = (0..4)
            .map(|f| {
                tex(move |x, y| {
                    let p = std::f32::consts::TAU;
                    let v = ((x + 4 * f) as f32 * p / 16.0 + y as f32 * p * 2.0 / 16.0).sin() * 0.5
                        + 0.5
                        + (hash(x, y, 30) - 0.5) * 0.25;
                    if v > 0.92 {
                        rgb(150, 200, 255)
                    } else {
                        mix(rgb(38, 80, 185), rgb(70, 130, 230), v)
                    }
                })
            })
            .collect();
        let lava = (0..4)
            .map(|f| {
                tex(move |x, y| {
                    let p = std::f32::consts::TAU;
                    let v = ((x + 4 * f) as f32 * p / 16.0).sin() * 0.3
                        + ((y - 4 * f) as f32 * p / 16.0 + x as f32 * p / 8.0).sin() * 0.3
                        + 0.5
                        + (hash(x, y, 40) - 0.5) * 0.3;
                    if v > 0.9 {
                        rgb(255, 225, 100)
                    } else {
                        mix(rgb(190, 55, 10), rgb(250, 150, 20), v)
                    }
                })
            })
            .collect();
        let portal = (0..4)
            .map(|f| {
                tex(move |x, y| {
                    if x < 2 || x > 13 || y < 2 || y > 13 {
                        return if hash(x, y, 50) < 0.2 { rgb(70, 35, 100) } else { rgb(20, 12, 30) };
                    }
                    let (cx, cy) = (x as f32 - 7.5, y as f32 - 7.5);
                    let d = (cx * cx + cy * cy).sqrt();
                    let a = (cy as f64).atan2(cx as f64) as f32; // f64: f32 atan2 needs a very new glibc
                    let v = (d * 1.3 - f as f32 * std::f32::consts::FRAC_PI_2 + a * 2.0).sin() * 0.5 + 0.5;
                    mix(rgb(100, 25, 180), rgb(215, 130, 255), v)
                })
            })
            .collect();

        let skin = rgb(198, 140, 100);
        let pal = [
            ('H', rgb(59, 40, 25)),
            ('S', skin),
            ('W', WHITE),
            ('B', rgb(73, 70, 150)),
            ('N', rgb(150, 95, 65)),
            ('M', rgb(110, 60, 40)),
            ('C', rgb(0, 168, 168)),
            ('c', rgb(0, 130, 130)),
            ('P', rgb(60, 60, 160)),
            ('G', rgb(85, 85, 85)),
        ];
        let down = [
            "..HHHHHHHH..", "..HHHHHHHH..", "..HSSSSSSH..", "..SSSSSSSS..", "..SWBSSBWS..",
            "..SSSSSSSS..", "..SSSNNSSS..", "..SSMMMMSS..", ".CCCCCCCCCC.", ".SCCCCCCCCS.",
            ".SCCCCCCCCS.", ".ScccccccccS", "..PPPPPPPP..", "..PPP..PPP..", "..PPP..PPP..",
            "..GGG..GGG..",
        ];
        let up = [
            "..HHHHHHHH..", "..HHHHHHHH..", "..HHHHHHHH..", "..HHHHHHHH..", "..HHHHHHHH..",
            "..HHHHHHHH..", "..HHHHHHHH..", "..SHHHHHHS..", ".CCCCCCCCCC.", ".SCCCCCCCCS.",
            ".SCCCCCCCCS.", ".ScccccccccS", "..PPPPPPPP..", "..PPP..PPP..", "..PPP..PPP..",
            "..GGG..GGG..",
        ];
        let right = [
            "..HHHHHHHH..", "..HHHHHHHH..", "..HHHHSSSS..", "..HHHSSSSS..", "..HHSSSSBS..",
            "..HSSSSSSS..", "..SSSSSSSN..", "..SSSSSMMS..", "...CCCCCC...", "...CCCCCC...",
            "...CSSCCC...", "...cSSccc...", "...PPPPPP...", "...PPPPPP...", "...PPPPPP...",
            "...GGGGGG...",
        ];
        let left: Vec<String> = right.iter().map(|r| r.chars().rev().collect()).collect();
        let left: Vec<&str> = left.iter().map(|s| s.as_str()).collect();

        let bit = sprite(BIT, &bit_palette());
        let pickaxe = sprite(
            &[
                "...CCCCC....", "..CccccCC...", ".......cCC..", "......W.cC..", ".....W...cC.",
                "....W....cC.", "...W......C.", "..W.........", ".W..........", "W...........",
            ],
            &[('C', rgb(60, 220, 210)), ('c', rgb(30, 160, 160)), ('W', rgb(140, 100, 55))],
        );

        Art {
            grass: tex(noisy(rgb(98, 163, 58), 1)),
            dirt: tex(noisy(rgb(136, 98, 66), 3)),
            sand: tex(noisy(rgb(222, 208, 158), 5)),
            deepslate: tex(noisy(rgb(132, 140, 172), 7)),
            nether: tex(noisy(rgb(150, 70, 62), 9)),
            stone: tex(&stone_px),
            cobble: tex(|x, y| {
                let row = y / 4;
                let off = if row % 2 == 0 { 0 } else { 2 };
                let (lx, ly) = ((x + off) % 4, y % 4);
                if lx == 0 || ly == 0 {
                    rgb(72, 72, 72)
                } else {
                    let s = 0.8 + 0.35 * hash((x + off) / 4, row, 5) - if lx == 3 || ly == 3 { 0.1 } else { 0.0 };
                    shade(rgb(128, 128, 128), s)
                }
            }),
            ore: tex(|x, y| {
                for (sx, sy) in [(3, 2), (10, 3), (6, 7), (12, 10), (2, 11), (8, 13)] {
                    match (x - sx, y - sy) {
                        (0, 0) => return rgb(200, 255, 250),
                        (1, 0) => return rgb(90, 225, 215),
                        (0, 1) => return rgb(45, 175, 175),
                        (1, 1) => return rgb(25, 115, 125),
                        _ => {}
                    }
                }
                stone_px(x, y)
            }),
            tree: tex(|x, y| {
                let (cx, cy) = (x as f32 - 7.5, y as f32 - 7.5);
                let d = (cx * cx + cy * cy).sqrt();
                if d > 7.7 {
                    BLANK
                } else if d > 6.7 {
                    rgb(30, 70, 22)
                } else {
                    let light = 1.0 - (cx + cy) * 0.025;
                    let n = hash(x / 2, y / 2, 20);
                    shade(rgb(58, 125, 40), (0.75 + 0.35 * n) * light)
                }
            }),
            cactus: tex(|x, y| {
                let (ax, ay) = ((x as f32 - 7.5).abs(), (y as f32 - 7.5).abs());
                if ax > 6.0 || ay > 6.0 {
                    BLANK
                } else if ax > 5.0 || ay > 5.0 {
                    rgb(40, 85, 30)
                } else if hash(x, y, 60) < 0.08 {
                    rgb(25, 40, 15)
                } else if x % 3 == 0 {
                    rgb(110, 170, 70)
                } else {
                    rgb(80, 140, 55)
                }
            }),
            bedrock: tex(|x, y| {
                [rgb(16, 16, 18), rgb(42, 42, 46), rgb(66, 66, 70), rgb(28, 28, 30)]
                    [(hash(x, y, 70) * 3.99) as usize]
            }),
            water,
            lava,
            bridge: tex(|x, y| {
                let base = rgb(165, 128, 78);
                if y % 4 == 3 || (x + (y / 4) * 5) % 8 == 0 {
                    shade(base, 0.68)
                } else {
                    shade(base, 0.92 + 0.15 * hash(x, y, 80))
                }
            }),
            emerald: tex(|x, y| {
                gem(x, y, rgb(110, 240, 140), rgb(40, 190, 85), rgb(20, 130, 55), rgb(10, 70, 30))
            }),
            diamond: tex(|x, y| {
                gem(x, y, rgb(170, 250, 245), rgb(80, 220, 215), rgb(30, 150, 160), rgb(15, 80, 90))
            }),
            crystal: tex(|x, y| {
                // A power crystal on a dark rock base.
                let c = gem(x, y, rgb(255, 200, 255), rgb(225, 110, 245), rgb(160, 50, 200), rgb(80, 20, 110));
                if c == BLANK { shade(rgb(52, 36, 64), 0.85 + 0.3 * hash(x, y, 90)) } else { c }
            }),
            bosses: BOSS_ART
                .iter()
                .map(|(rows, pal)| {
                    let pal: Vec<(char, Color)> = pal.iter().map(|&(c, (r, g, b))| (c, rgb(r, g, b))).collect();
                    sprite(rows, &pal)
                })
                .collect(),
            heart_full: sprite(&HEART, &[('k', rgb(40, 10, 10)), ('R', rgb(220, 30, 40)), ('W', WHITE)]),
            heart_empty: sprite(&HEART, &[('k', rgb(40, 10, 10)), ('R', rgb(70, 60, 60)), ('W', rgb(100, 90, 90))]),
            portal,
            portal_off: tex(|x, y| {
                if x < 2 || x > 13 || y < 2 || y > 13 {
                    if hash(x, y, 50) < 0.2 { rgb(70, 35, 100) } else { rgb(20, 12, 30) }
                } else {
                    shade(rgb(45, 28, 65), 0.85 + 0.3 * hash(x, y, 51))
                }
            }),
            player: SHIRTS
                .iter()
                .map(|&(r, g, b)| {
                    let shirt = rgb(r, g, b);
                    let mut pal = pal;
                    pal[6].1 = shirt;
                    pal[7].1 = shade(shirt, 0.77);
                    [sprite(&up, &pal), sprite(&right, &pal), sprite(&down, &pal), sprite(&left, &pal)]
                })
                .collect(),
            bit,
            pickaxe,
        }
    }

    pub fn floor(&self, f: Floor) -> &Texture2D {
        match f {
            Floor::Grass => &self.grass,
            Floor::Dirt => &self.dirt,
            Floor::Sand => &self.sand,
            Floor::Deepslate => &self.deepslate,
            Floor::Nether => &self.nether,
        }
    }

    /// The texture drawn on top of the floor for a tile (None = just floor).
    pub fn tile(&self, t: Tile, floor: Floor, time: f64, portal_on: bool) -> Option<&Texture2D> {
        let frame = ((time * 4.0) as usize) % 4;
        Some(match t {
            Tile::Floor | Tile::Emerald => return None,
            Tile::Stone => &self.stone,
            Tile::Cobble => &self.cobble,
            Tile::Ore => &self.ore,
            Tile::Tree if floor == Floor::Sand => &self.cactus,
            Tile::Tree => &self.tree,
            Tile::Bedrock => &self.bedrock,
            Tile::Water => &self.water[frame],
            Tile::Lava => &self.lava[frame],
            Tile::Bridge => &self.bridge,
            Tile::Portal if portal_on => &self.portal[frame],
            Tile::Portal => &self.portal_off,
            Tile::Crystal => &self.crystal,
            // The boss is drawn once over all its squares (see `boss`).
            Tile::Boss => return None,
        })
    }

    /// A boss creature, drawn once over all of its squares.
    pub fn boss(&self, kind: BossKind) -> &Texture2D {
        &self.bosses[kind as usize]
    }

    /// The miner sprite, wearing shirt colour number `shirt` (see `SHIRTS`).
    pub fn player(&self, shirt: usize, d: Dir) -> &Texture2D {
        &self.player[shirt % SHIRTS.len()][d as usize]
    }
}

pub fn draw_tex(t: &Texture2D, x: f32, y: f32, w: f32, h: f32, tint: Color) {
    draw_texture_ex(t, x, y, tint, DrawTextureParams { dest_size: Some(vec2(w, h)), ..Default::default() });
}
