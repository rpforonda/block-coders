//! How the Wardrobe's items (see `items.rs`) look on the miner. Patterned
//! shirts and armor are see-through overlays built from the miner's own pixel
//! rows, so they line up in every facing. Tools are small sprites held in the
//! hand. Lights, sparkles and the Superstar glow are drawn live.

use super::{hash, miner_rows, mix, rgb, shade, sprite, DIRS, PICKAXE};
use crate::world::Dir;
use macroquad::prelude::*;
use std::f32::consts::TAU;

/// How an item is made, as plain data, so tests can check every item has one.
pub enum Design {
    /// The shirt's color at miner pixel (x, y) in animation frame `f`.
    Shirt { frames: usize, fps: f32, px: fn(i32, i32, usize) -> Color },
    Armor(Armor),
    Tool(Tool),
}

impl Design {
    #[cfg(test)]
    pub fn slot(&self) -> crate::items::Slot {
        use crate::items::Slot;
        match self {
            Design::Shirt { .. } => Slot::Shirt,
            Design::Armor(_) => Slot::Armor,
            Design::Tool(_) => Slot::Tool,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum Finish {
    Plain,
    /// A checkerboard of rings.
    Chain,
    /// Purple glints that pulse.
    Glint,
    /// A star on the chest, twinkles, and a golden glow.
    Stars,
}

pub struct Armor {
    light: (u8, u8, u8),
    mid: (u8, u8, u8),
    dark: (u8, u8, u8),
    finish: Finish,
}

#[derive(Clone, Copy)]
pub struct Tool {
    /// Pixel rows for animation frame `f`.
    rows: fn(usize) -> &'static [&'static str],
    palette: fn(usize) -> Vec<(char, Color)>,
    frames: usize,
    /// Where the hand holds it, in its own pixels.
    grip: (f32, f32),
    /// The size of its pixels, in the miner's.
    scale: f32,
    glow: Option<Light>,
    sparkles: bool,
}

/// The light a tool gives off.
#[derive(Clone, Copy)]
pub struct Light {
    color: (u8, u8, u8),
    /// In the tool's own pixels.
    center: (f32, f32),
    /// In the miner's pixels.
    radius: f32,
}

const fn light(color: (u8, u8, u8), center: (f32, f32), radius: f32) -> Option<Light> {
    Some(Light { color, center, radius })
}

const TORCH: [&[&str]; 4] = [
    &[".f.", "fYf", "fYf", ".k.", ".w.", ".w.", ".w.", ".w.", ".w."],
    &["f..", "fYf", ".Yf", ".k.", ".w.", ".w.", ".w.", ".w.", ".w."],
    &[".f.", "fY.", "fYf", ".k.", ".w.", ".w.", ".w.", ".w.", ".w."],
    &["..f", "fYf", "fY.", ".k.", ".w.", ".w.", ".w.", ".w.", ".w."],
];
const LANTERN: &[&str] = &["..k..", ".k.k.", "kkkkk", "kYWYk", "kYYYk", "kkkkk"];
const WAND: &[&str] = &["..Y..", ".YWY.", "YWWWY", ".YWY.", "..Y..", "..k..", "..k..", "..k..", "..k..", "..k..", "..k.."];
const STAFF: &[&str] = &[
    "..c..", ".cCc.", ".CWC.", ".cCc.", "..c..", ".g.g.", "..g..", "..w..", "..w..", "..w..", "..w..", "..w..",
    "..w..", "..w..", "..w..", "..w..", "..w..", "..w..",
];

/// Each item's design, by key. None for the classic shirts (body colors) and
/// unknown keys.
pub fn design(key: &str) -> Option<Design> {
    let armor = |light, mid, dark, finish| Some(Design::Armor(Armor { light, mid, dark, finish }));
    let pickaxe = |palette: fn(usize) -> Vec<(char, Color)>, sparkles: bool| Tool {
        rows: |_| PICKAXE,
        palette,
        frames: 1,
        grip: (1.0, 8.5),
        scale: 0.6,
        glow: if sparkles { light((150, 245, 240), (9.0, 2.0), 2.5) } else { None },
        sparkles,
    };
    Some(match key {
        "stripes" => Design::Shirt { frames: 1, fps: 0.0, px: |_, y, _| if y % 2 == 0 { rgb(35, 55, 135) } else { rgb(240, 240, 245) } },
        "plaid" => Design::Shirt {
            frames: 1,
            fps: 0.0,
            px: |x, y, _| match (x % 3 == 0, y % 3 == 0) {
                (true, true) => rgb(55, 15, 20),
                (true, false) | (false, true) => rgb(125, 25, 30),
                _ => rgb(205, 45, 45),
            },
        },
        "galaxy" => Design::Shirt {
            frames: 4,
            fps: 3.0,
            px: |x, y, f| {
                if hash(x, y, 90) < 0.2 {
                    if hash(x, y, 91 + f as u32) > 0.4 { WHITE } else { rgb(190, 170, 255) }
                } else {
                    mix(rgb(30, 15, 75), rgb(75, 35, 130), hash(x, y, 92))
                }
            },
        },
        "rainbow" => Design::Shirt { frames: 8, fps: 8.0, px: |x, y, f| hue((x + y + f as i32) as f32 / 8.0) },
        "leather" => return armor((176, 118, 66), (140, 88, 48), (96, 58, 32), Finish::Plain),
        "chainmail" => return armor((185, 190, 200), (140, 145, 155), (95, 100, 112), Finish::Chain),
        "iron" => return armor((240, 240, 245), (198, 198, 206), (128, 128, 140), Finish::Plain),
        "gold" => return armor((255, 240, 120), (240, 195, 45), (175, 125, 20), Finish::Plain),
        "diamond" => return armor((200, 255, 250), (90, 225, 220), (35, 155, 165), Finish::Plain),
        "netherite" => return armor((100, 90, 100), (70, 62, 72), (42, 36, 44), Finish::Glint),
        "superstar" => return armor((255, 255, 225), (255, 212, 70), (205, 140, 25), Finish::Stars),
        "torch" => Design::Tool(Tool {
            rows: |f| TORCH[f % 4],
            palette: |_| vec![('f', rgb(250, 140, 30)), ('Y', rgb(255, 235, 130)), ('k', rgb(70, 45, 25)), ('w', rgb(140, 95, 50))],
            frames: 4,
            grip: (1.5, 7.5),
            scale: 0.9,
            glow: light((255, 190, 80), (1.5, 1.5), 4.0),
            sparkles: false,
        }),
        "lantern" => Design::Tool(Tool {
            rows: |_| LANTERN,
            palette: |_| vec![('k', rgb(55, 55, 65)), ('Y', rgb(255, 200, 70)), ('W', rgb(255, 245, 190))],
            frames: 1,
            grip: (2.5, 0.5),
            scale: 0.75,
            glow: light((255, 200, 90), (2.5, 3.5), 4.5),
            sparkles: false,
        }),
        "gold-pick" => Design::Tool(pickaxe(|_| vec![('C', rgb(255, 225, 80)), ('c', rgb(195, 145, 25)), ('W', rgb(140, 100, 55))], false)),
        "diamond-pick" => Design::Tool(pickaxe(|_| vec![('C', rgb(150, 245, 240)), ('c', rgb(45, 175, 180)), ('W', rgb(140, 100, 55))], true)),
        "wand" => Design::Tool(Tool {
            rows: |_| WAND,
            palette: |_| vec![('Y', rgb(255, 215, 60)), ('W', rgb(255, 250, 210)), ('k', rgb(90, 50, 120))],
            frames: 1,
            grip: (2.5, 9.0),
            scale: 0.8,
            glow: light((255, 230, 120), (2.5, 2.5), 3.0),
            sparkles: true,
        }),
        "crystal-staff" => Design::Tool(Tool {
            rows: |_| STAFF,
            palette: |f| {
                let c = [rgb(190, 90, 255), rgb(150, 120, 255), rgb(90, 200, 255), rgb(150, 120, 255)][f % 4];
                vec![('C', c), ('c', shade(c, 0.6)), ('W', WHITE), ('g', rgb(230, 180, 50)), ('w', rgb(110, 70, 40))]
            },
            frames: 4,
            grip: (2.5, 13.0),
            scale: 0.9,
            glow: light((170, 120, 255), (2.5, 2.0), 5.0),
            sparkles: true,
        }),
        _ => return None,
    })
}

/// A bright color around the color wheel (`h` in turns).
fn hue(h: f32) -> Color {
    let h = h.rem_euclid(1.0) * 6.0;
    let x = 1.0 - (h % 2.0 - 1.0).abs();
    let (r, g, b) = match h as i32 {
        0 => (1.0, x, 0.0),
        1 => (x, 1.0, 0.0),
        2 => (0.0, 1.0, x),
        3 => (0.0, x, 1.0),
        4 => (x, 0.0, 1.0),
        _ => (1.0, 0.0, x),
    };
    Color::new(0.25 + 0.75 * r, 0.25 + 0.75 * g, 0.25 + 0.75 * b, 1.0)
}

/// Armor pixel (x, y) over miner pixel `ch` facing `d`, in animation frame `f`.
fn armor_px(a: &Armor, ch: char, x: i32, y: i32, d: Dir, f: usize) -> Option<Color> {
    let [light, mid, dark] = [a.light, a.mid, a.dark].map(|(r, g, b)| rgb(r, g, b));
    let front = matches!(d, Dir::Up | Dir::Down);
    let base = match ch {
        // A helmet over the top of the head.
        'H' if y <= 2 => [light, mid, dark][y as usize],
        // A chestplate, leaving the shirt's sleeves showing from the front.
        'C' | 'c' if !front || (3..=8).contains(&x) => {
            if ch == 'c' {
                dark
            } else if y == 8 || (front && x == 3) {
                light
            } else {
                mid
            }
        }
        'P' if y == 12 => dark,
        'P' => mid,
        'G' => dark,
        _ => return None,
    };
    Some(match a.finish {
        Finish::Chain if ch != 'G' => {
            if (x + y) % 2 == 0 { light } else { dark }
        }
        Finish::Glint if hash(x, y, 77) < 0.16 => mix(base, rgb(200, 110, 255), [0.35, 0.7, 1.0, 0.7][f % 4]),
        Finish::Stars if d == Dir::Down && (5..=6).contains(&x) && (9..=10).contains(&y) => WHITE,
        Finish::Stars if hash(x, y, 80 + f as u32) < 0.1 => WHITE,
        _ => base,
    })
}

/// A see-through texture over the miner facing `d`: `px` colors the pixels
/// it covers (given the miner's own pixel there) and leaves the rest clear.
fn overlay(d: Dir, px: impl Fn(char, i32, i32) -> Option<Color>) -> Texture2D {
    let mut img = Image::gen_image_color(12, 16, BLANK);
    for (y, row) in miner_rows(d).iter().enumerate() {
        for (x, ch) in row.chars().enumerate() {
            if let Some(c) = px(ch, x as i32, y as i32) {
                img.set_pixel(x as u32, y as u32, c);
            }
        }
    }
    let t = Texture2D::from_image(&img);
    t.set_filter(FilterMode::Nearest);
    t
}

/// An item's textures.
pub enum ItemArt {
    /// The classic shirts: just the body's color.
    None,
    /// Overlays on the miner: one per facing, per animation frame.
    Worn { frames: Vec<[Texture2D; 4]>, fps: f32, aura: bool },
    /// A sprite held in the hand, per animation frame, pointing right.
    Held { frames: Vec<Texture2D>, fps: f32, tool: Tool },
}

pub fn item_art(key: &str) -> ItemArt {
    match design(key) {
        None => ItemArt::None,
        Some(Design::Shirt { frames, fps, px }) => ItemArt::Worn {
            frames: (0..frames)
                .map(|f| {
                    DIRS.map(|d| {
                        overlay(d, |ch, x, y| match ch {
                            'C' => Some(px(x, y, f)),
                            'c' => Some(shade(px(x, y, f), 0.77)),
                            _ => None,
                        })
                    })
                })
                .collect(),
            fps,
            aura: false,
        },
        Some(Design::Armor(a)) => {
            let n = if matches!(a.finish, Finish::Glint | Finish::Stars) { 4 } else { 1 };
            ItemArt::Worn {
                frames: (0..n).map(|f| DIRS.map(|d| overlay(d, |ch, x, y| armor_px(&a, ch, x, y, d, f)))).collect(),
                fps: 4.0,
                aura: a.finish == Finish::Stars,
            }
        }
        Some(Design::Tool(tool)) => ItemArt::Held {
            frames: (0..tool.frames).map(|f| sprite((tool.rows)(f), &(tool.palette)(f))).collect(),
            fps: 6.0,
            tool,
        },
    }
}

fn frame(time: f64, fps: f32, n: usize) -> usize {
    (time * fps as f64) as usize % n.max(1)
}

/// A soft, gently pulsing light.
fn glow(p: Vec2, radius: f32, c: Color, tint: Color, time: f64) {
    let pulse = 0.88 + 0.12 * (time * 6.0).sin() as f32;
    for (k, a) in [(1.0, 0.1), (0.65, 0.14), (0.35, 0.2)] {
        draw_circle(p.x, p.y, radius * k * pulse, Color::new(c.r * tint.r, c.g * tint.g, c.b * tint.b, a * tint.a));
    }
}

/// Three twinkling specks circling `p`.
fn sparkles(p: Vec2, radius: f32, tint: Color, time: f64) {
    for i in 0..3 {
        let a = time as f32 * 2.0 + i as f32 * TAU / 3.0;
        let twinkle = ((time * 5.0 + i as f64 * 1.7).sin() * 0.5 + 0.5) as f32;
        let s = radius * 0.2;
        let q = p + vec2(a.cos(), a.sin()) * radius * 0.9;
        draw_rectangle(q.x - s / 2.0, q.y - s / 2.0, s, s, Color::new(tint.r, tint.g, tint.b * 0.85, twinkle * tint.a));
    }
}

/// How far past the miner's 12-pixel width a tool can reach, side on.
pub const REACH: f32 = 4.6;

/// What goes behind the miner: the Superstar Armor's golden glow.
pub fn draw_behind(art: &ItemArt, r: Rect, tint: Color, time: f64) {
    if let ItemArt::Worn { aura: true, .. } = art {
        let center = vec2(r.x + r.w / 2.0, r.y + r.h / 2.0);
        glow(center, r.w * 0.75, rgb(255, 220, 90), tint, time);
        sparkles(center, r.w * 0.7, tint, time + 1.0);
    }
}

/// A shirt or armor over the miner facing `d`.
pub fn draw_worn(art: &ItemArt, d: Dir, r: Rect, tint: Color, time: f64) {
    if let ItemArt::Worn { frames, fps, .. } = art {
        let tex = &frames[frame(time, *fps, frames.len())][d as usize];
        super::draw_tex(tex, r.x, r.y, r.w, r.h, tint);
    }
}

/// A tool in the miner's hand, facing `d`, with its light.
pub fn draw_held(art: &ItemArt, d: Dir, r: Rect, tint: Color, time: f64) {
    let ItemArt::Held { frames, fps, tool } = art else { return };
    let tex = &frames[frame(time, *fps, frames.len())];
    let (ux, uy) = (r.w / 12.0, r.h / 16.0);
    // Where the hand is, in the miner's pixels, and whether the tool points
    // left. Side on, it's held out in front, clear of the face.
    let (hand, flip) = match d {
        Dir::Down => ((1.5, 11.5), true),
        Dir::Up => ((10.5, 11.5), false),
        Dir::Right => ((10.0, 11.5), false),
        Dir::Left => ((2.0, 11.5), true),
    };
    let (w, h) = (tex.width(), tex.height());
    let gx = if flip { w - tool.grip.0 } else { tool.grip.0 };
    // From the front or back, wide tools (the pickaxes) slide across the body
    // rather than poke far outside the miner's rectangle, which cards and
    // buttons are sized for. Side on, they reach up to `REACH` out in front.
    let x = hand.0 - gx * tool.scale;
    let x = if matches!(d, Dir::Up | Dir::Down) { x.min(13.5 - w * tool.scale).max(-1.5) } else { x };
    let y = hand.1 - tool.grip.1 * tool.scale;
    let params = DrawTextureParams { dest_size: Some(vec2(w * tool.scale * ux, h * tool.scale * uy)), flip_x: flip, ..Default::default() };
    draw_texture_ex(tex, r.x + x * ux, r.y + y * uy, tint, params);
    if let Some(Light { color: (cr, cg, cb), center: (cx, cy), radius }) = tool.glow {
        let cx = if flip { w - cx } else { cx };
        let p = vec2(r.x + (x + cx * tool.scale) * ux, r.y + (y + cy * tool.scale) * uy);
        glow(p, radius * ux, rgb(cr, cg, cb), tint, time);
        if tool.sparkles {
            sparkles(p, radius * ux, tint, time);
        }
    }
}
