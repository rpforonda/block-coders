//! An on-screen keyboard for typing names on touch screens.

use crate::sound::{Sfx, Sounds};
use crate::ui::*;
use macroquad::prelude::*;
use std::sync::atomic::{AtomicBool, Ordering};

static TOUCH: AtomicBool = AtomicBool::new(false);

/// Works out once whether this computer has a touch screen.
/// BC_TOUCH=1 or BC_TOUCH=0 overrides the check (handy for testing).
pub fn detect_touch() {
    let found = match std::env::var("BC_TOUCH").as_deref() {
        Ok("1") => true,
        Ok("0") => false,
        _ => has_touch_screen(),
    };
    TOUCH.store(found, Ordering::Relaxed);
}

/// Call every frame: a real touch event also counts (e.g. on Wayland or Android).
pub fn note_touches() {
    if !touches().is_empty() {
        TOUCH.store(true, Ordering::Relaxed);
    }
}

pub fn is_touch() -> bool {
    TOUCH.load(Ordering::Relaxed)
}

/// On X11 a tap arrives as an ordinary mouse click, so look for the hardware
/// instead: a touch screen is an input device with absolute axes and the
/// INPUT_PROP_DIRECT flag (touchpads have INPUT_PROP_POINTER instead).
#[cfg(target_os = "linux")]
fn has_touch_screen() -> bool {
    let Ok(dir) = std::fs::read_dir("/sys/class/input") else { return false };
    let hex_word = |path: std::path::PathBuf| {
        std::fs::read_to_string(path)
            .ok()
            .and_then(|s| s.split_whitespace().last().and_then(|w| u64::from_str_radix(w, 16).ok()))
            .unwrap_or(0)
    };
    dir.flatten().filter(|e| e.file_name().to_string_lossy().starts_with("event")).any(|e| {
        let dev = e.path().join("device");
        const INPUT_PROP_DIRECT: u64 = 1 << 1;
        hex_word(dev.join("properties")) & INPUT_PROP_DIRECT != 0 && hex_word(dev.join("capabilities/abs")) != 0
    })
}

/// In a browser, ask whether the main pointer is a finger (web/storage.js).
#[cfg(target_arch = "wasm32")]
fn has_touch_screen() -> bool {
    unsafe extern "C" {
        fn bc_touch_screen() -> i32;
    }
    // SAFETY: takes no arguments and returns a plain number.
    unsafe { bc_touch_screen() == 1 }
}

#[cfg(not(any(target_os = "linux", target_arch = "wasm32")))]
fn has_touch_screen() -> bool {
    false
}

const ROWS: [&str; 4] = ["1234567890-", "qwertyuiop_", "asdfghjkl", "zxcvbnm"];
const KEY: f32 = 78.0;
const GAP: f32 = 8.0;
const KEY_H: f32 = 52.0;

/// The space the keyboard takes up at the bottom of the screen.
pub const AREA: Rect = Rect { x: 0.0, y: VH - 5.0 * (KEY_H + GAP) - 14.0, w: VW, h: 5.0 * (KEY_H + GAP) + 14.0 };

pub enum Key {
    Char(char),
    Backspace,
    Hide,
}

pub struct Keyboard {
    pub open: bool,
    shift: bool,
    /// When the erase key started being held, for key repeat.
    held_since: Option<f64>,
    last_repeat: f64,
}

impl Keyboard {
    pub fn new() -> Keyboard {
        Keyboard { open: is_touch(), shift: false, held_since: None, last_repeat: 0.0 }
    }

    /// Draws the keyboard and returns the key tapped this frame, if any.
    /// `empty` turns on capitals for the first letter, like a phone does.
    pub fn frame(&mut self, ui: &mut Ui, sfx: &Sounds, empty: bool) -> Option<Key> {
        let a = AREA;
        dark_panel(a);
        let upper = self.shift || empty;
        let mut out = None;
        let row_y = |r: usize| a.y + 14.0 + r as f32 * (KEY_H + GAP);
        let row_x = |n: f32| (VW - n * KEY - (n - 1.0) * GAP) / 2.0;

        // Letter rows. Row 0 gets the erase key on the end, row 3 has shift at both ends.
        for (r, keys) in ROWS.iter().enumerate() {
            let extra = match r {
                0 => 1.6,
                3 => 2.0 * 1.5,
                _ => 0.0,
            };
            let n = keys.len() as f32;
            let mut x = row_x(n + extra) + if r == 3 { 1.5 * KEY + GAP } else { 0.0 };
            if r == 2 {
                x += KEY * 0.3;
            }
            for c in keys.chars() {
                let shown = if upper { c.to_ascii_uppercase() } else { c };
                let b = Rect::new(x, row_y(r), KEY, KEY_H);
                if ui.button(b, &shown.to_string(), rgb8(225, 225, 232), 30.0) {
                    out = Some(Key::Char(shown));
                }
                x += KEY + GAP;
            }
            if r == 0 {
                let b = Rect::new(x, row_y(0), KEY * 1.6, KEY_H);
                let hit = ui.button(b, "", rgb8(150, 80, 70), 20.0);
                erase_icon(b);
                if hit {
                    out = Some(Key::Backspace);
                    self.held_since = Some(get_time());
                    self.last_repeat = get_time();
                } else if let Some(t) = self.held_since {
                    // Holding erase keeps erasing.
                    let now = get_time();
                    if !(ui.down && ui.hover(b)) {
                        self.held_since = None;
                    } else if now - t > 0.45 && now - self.last_repeat > 0.07 {
                        self.last_repeat = now;
                        out = Some(Key::Backspace);
                    }
                }
            }
            if r == 3 {
                for sx in [row_x(n + extra), x] {
                    let b = Rect::new(sx, row_y(3), KEY * 1.5, KEY_H);
                    let fill = if self.shift { rgb8(255, 215, 40) } else { rgb8(120, 120, 135) };
                    if ui.button(b, "", fill, 20.0) {
                        sfx.play(Sfx::Click);
                        self.shift = !self.shift;
                    }
                    shift_icon(b, if self.shift { rgb8(40, 30, 0) } else { WHITE });
                }
            }
        }

        // Bottom row: space bar and hide.
        let y = row_y(4);
        let space = Rect::new(row_x(8.0), y, 6.0 * KEY + 5.0 * GAP, KEY_H);
        if ui.button(space, "space", rgb8(225, 225, 232), 22.0) {
            out = Some(Key::Char(' '));
        }
        let hide = Rect::new(space.x + space.w + GAP, y, 2.0 * KEY + GAP, KEY_H);
        if ui.button(hide, "Hide", rgb8(90, 90, 110), 22.0) {
            out = Some(Key::Hide);
        }

        match out {
            Some(Key::Char(c)) => {
                sfx.play(Sfx::Click);
                if c != ' ' {
                    self.shift = false;
                }
            }
            Some(Key::Hide) => {
                sfx.play(Sfx::Click);
                self.open = false;
            }
            _ => {}
        }
        out
    }
}

/// A backspace symbol: an arrow-shaped tag with an X in it.
fn erase_icon(b: Rect) {
    let (cx, cy) = (b.x + b.w / 2.0 + 4.0, b.y + b.h / 2.0);
    let (w, h) = (40.0, 28.0);
    let l = cx - w / 2.0;
    let pts = [vec2(l, cy), vec2(l + 12.0, cy - h / 2.0), vec2(l + w, cy - h / 2.0), vec2(l + w, cy + h / 2.0), vec2(l + 12.0, cy + h / 2.0)];
    for i in 0..5 {
        let (p, q) = (pts[i], pts[(i + 1) % 5]);
        draw_line(p.x, p.y, q.x, q.y, 3.0, WHITE);
    }
    let (xc, s) = (l + 25.0, 6.0);
    draw_line(xc - s, cy - s, xc + s, cy + s, 3.0, WHITE);
    draw_line(xc - s, cy + s, xc + s, cy - s, 3.0, WHITE);
}

/// An up arrow for the shift key.
fn shift_icon(b: Rect, c: Color) {
    let (cx, cy) = (b.x + b.w / 2.0, b.y + b.h / 2.0);
    draw_triangle(vec2(cx, cy - 15.0), vec2(cx - 15.0, cy + 1.0), vec2(cx + 15.0, cy + 1.0), c);
    draw_rectangle(cx - 7.0, cy, 14.0, 14.0, c);
}

/// A small keyboard picture, for the button that brings the keyboard back.
pub fn keyboard_icon(b: Rect, c: Color) {
    let r = Rect::new(b.x + b.w / 2.0 - 20.0, b.y + b.h / 2.0 - 12.0, 40.0, 24.0);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 3.0, c);
    for row in 0..2 {
        for col in 0..5 {
            draw_rectangle(r.x + 5.0 + col as f32 * 6.5, r.y + 5.0 + row as f32 * 6.0, 4.0, 4.0, c);
        }
    }
    draw_rectangle(r.x + 10.0, r.y + 17.0, 20.0, 3.0, c);
}
