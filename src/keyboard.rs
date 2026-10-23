//! An on-screen keyboard for typing names on touch screens.

use crate::sound::{Sfx, Sounds};
use crate::layout::{self, Canvas, KeyCap};
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
    pub fn frame(&mut self, ui: &mut Ui, sfx: &Sounds, empty: bool, c: Canvas) -> Option<Key> {
        let (area, keys) = layout::keyboard(c);
        dark_panel(area);
        let upper = self.shift || empty;
        let mut out = None;
        for (cap, b) in keys {
            match cap {
                KeyCap::Char(ch) => {
                    let shown = if upper { ch.to_ascii_uppercase() } else { ch };
                    if ui.button(b, &shown.to_string(), rgb8(225, 225, 232), (b.h * 0.58).min(30.0)) {
                        out = Some(Key::Char(shown));
                    }
                }
                KeyCap::Erase => {
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
                KeyCap::Shift => {
                    let fill = if self.shift { rgb8(255, 215, 40) } else { rgb8(120, 120, 135) };
                    if ui.button(b, "", fill, 20.0) {
                        sfx.play(Sfx::Click);
                        self.shift = !self.shift;
                    }
                    let bg = readable_bg(fill);
                    shift_icon(b, text_on(bg));
                }
                KeyCap::Space => {
                    if ui.button(b, "space", rgb8(225, 225, 232), 22.0) {
                        out = Some(Key::Char(' '));
                    }
                }
                KeyCap::Hide => {
                    if ui.button(b, "Hide", rgb8(90, 90, 110), 22.0) {
                        out = Some(Key::Hide);
                    }
                }
            }
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
