//! Small immediate-mode UI helpers: buttons, panels and text.

use crate::art::shade;
use macroquad::prelude::*;
use std::sync::atomic::{AtomicU32, Ordering};

/// The game is drawn in a fixed 1280x800 space and scaled to fit the window.
pub const VW: f32 = 1280.0;
pub const VH: f32 = 800.0;

static SCALE: AtomicU32 = AtomicU32::new(0x3f80_0000); // 1.0

pub fn set_scale(s: f32) {
    SCALE.store(s.to_bits(), Ordering::Relaxed);
}

fn scale() -> f32 {
    f32::from_bits(SCALE.load(Ordering::Relaxed))
}

pub struct Ui {
    pub mouse: Vec2,
    pub click: bool,
    pub rclick: bool,
    /// The left button was let go this frame / is held down (for dragging).
    pub release: bool,
    pub down: bool,
    pub wheel: f32,
    /// When false (e.g. behind a popup), nothing reacts to the mouse.
    pub enabled: bool,
    pub clicked_any: bool,
}

impl Ui {
    pub fn new(mouse: Vec2) -> Ui {
        Ui {
            mouse,
            click: is_mouse_button_pressed(MouseButton::Left),
            rclick: is_mouse_button_pressed(MouseButton::Right),
            release: is_mouse_button_released(MouseButton::Left),
            down: is_mouse_button_down(MouseButton::Left),
            wheel: mouse_wheel().1,
            enabled: true,
            clicked_any: false,
        }
    }

    pub fn hover(&self, r: Rect) -> bool {
        self.enabled && r.contains(self.mouse)
    }

    /// True once if `r` was clicked this frame (the click is then used up).
    pub fn clicked(&mut self, r: Rect) -> bool {
        if self.click && self.hover(r) {
            self.click = false;
            self.clicked_any = true;
            true
        } else {
            false
        }
    }

    pub fn button(&mut self, r: Rect, label: &str, color: Color, size: f32) -> bool {
        let hover = self.hover(r);
        bevel(r, if hover { shade(color, 1.18) } else { color });
        let tc = if luminance(color) > 0.8 { rgb8(30, 30, 30) } else { WHITE };
        text_centered(label, r, size, tc, luminance(color) <= 0.8);
        self.clicked(r)
    }

    /// A button that can be greyed out.
    pub fn button_if(&mut self, on: bool, r: Rect, label: &str, color: Color, size: f32) -> bool {
        if on {
            self.button(r, label, color, size)
        } else {
            bevel(r, rgb8(90, 90, 90));
            text_centered(label, r, size, rgb8(150, 150, 150), false);
            false
        }
    }
}

pub fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

pub fn luminance(c: Color) -> f32 {
    0.299 * c.r + 0.587 * c.g + 0.114 * c.b
}

/// A chunky Minecraft-style block: light top-left edge, dark bottom-right edge.
pub fn bevel(r: Rect, c: Color) {
    draw_rectangle(r.x - 2.0, r.y - 2.0, r.w + 4.0, r.h + 4.0, BLACK);
    draw_rectangle(r.x, r.y, r.w, r.h, c);
    let e = 3.0_f32.min(r.h / 4.0);
    draw_rectangle(r.x, r.y, r.w, e, shade(c, 1.3));
    draw_rectangle(r.x, r.y, e, r.h, shade(c, 1.2));
    draw_rectangle(r.x, r.y + r.h - e, r.w, e, shade(c, 0.65));
    draw_rectangle(r.x + r.w - e, r.y, e, r.h, shade(c, 0.72));
}

pub fn panel(r: Rect) {
    draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, BLACK);
    draw_rectangle(r.x, r.y, r.w, r.h, rgb8(198, 198, 198));
    draw_rectangle(r.x, r.y, r.w, 3.0, WHITE);
    draw_rectangle(r.x, r.y, 3.0, r.h, WHITE);
    draw_rectangle(r.x, r.y + r.h - 3.0, r.w, 3.0, rgb8(85, 85, 85));
    draw_rectangle(r.x + r.w - 3.0, r.y, 3.0, r.h, rgb8(85, 85, 85));
}

pub fn dark_panel(r: Rect) {
    draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, BLACK);
    draw_rectangle(r.x, r.y, r.w, r.h, rgb8(40, 40, 48));
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, rgb8(90, 90, 105));
}

/// Draws text with its top-left corner at (x, y). Text is rendered at the real
/// screen resolution so it stays sharp when the window is scaled.
pub fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let sc = scale();
    draw_text_ex(
        s,
        x,
        y + size * 0.72,
        TextParams {
            font_size: (size * sc).round().max(1.0) as u16,
            font_scale: 1.0 / sc,
            color: c,
            ..Default::default()
        },
    );
}

pub fn shadow_text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let o = (size / 12.0).max(1.5);
    text(s, x + o, y + o, size, shade(c, 0.25));
    text(s, x, y, size, c);
}

pub fn text_width(s: &str, size: f32) -> f32 {
    let sc = scale();
    measure_text(s, None, (size * sc).round().max(1.0) as u16, 1.0 / sc).width
}

pub fn text_centered(s: &str, r: Rect, size: f32, c: Color, shadow: bool) {
    let x = r.x + (r.w - text_width(s, size)) / 2.0;
    let y = r.y + (r.h - size) / 2.0 + 1.0;
    if shadow { shadow_text(s, x, y, size, c) } else { text(s, x, y, size, c) }
}

/// Splits text into lines that fit `width`. `\n` starts a new paragraph.
pub fn wrap(s: &str, width: f32, size: f32) -> Vec<String> {
    let mut lines = Vec::new();
    for para in s.split('\n') {
        let mut line = String::new();
        for word in para.split_whitespace() {
            let candidate = if line.is_empty() { word.to_string() } else { format!("{line} {word}") };
            if text_width(&candidate, size) > width && !line.is_empty() {
                lines.push(std::mem::take(&mut line));
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        lines.push(line);
    }
    lines
}

/// Draws wrapped text and returns the height it used.
pub fn paragraph(s: &str, x: f32, y: f32, width: f32, size: f32, c: Color) -> f32 {
    let lh = size * 1.3;
    let lines = wrap(s, width, size);
    for (i, l) in lines.iter().enumerate() {
        text(l, x, y + i as f32 * lh, size, c);
    }
    lines.len() as f32 * lh
}

/// A five-pointed pixel star, filled or empty.
pub fn star(cx: f32, cy: f32, r: f32, filled: bool) {
    let pts: Vec<Vec2> = (0..10)
        .map(|i| {
            let a = -std::f32::consts::FRAC_PI_2 + i as f32 * std::f32::consts::PI / 5.0;
            let rr = if i % 2 == 0 { r } else { r * 0.45 };
            vec2(cx + a.cos() * rr, cy + a.sin() * rr)
        })
        .collect();
    let c = vec2(cx, cy);
    let fill = if filled { rgb8(255, 215, 40) } else { rgb8(70, 70, 70) };
    for i in 0..10 {
        draw_triangle(c, pts[i], pts[(i + 1) % 10], fill);
    }
    for i in 0..10 {
        let a = pts[i];
        let b = pts[(i + 1) % 10];
        draw_line(a.x, a.y, b.x, b.y, 2.5, if filled { rgb8(150, 100, 0) } else { rgb8(30, 30, 30) });
    }
}

/// A small icon for each kind of code block (drawn with shapes).
pub fn block_icon(kind: crate::program::Kind, x: f32, y: f32, s: f32, art: &crate::art::Art) {
    use crate::program::Kind::*;
    let c = WHITE;
    let m = vec2(x + s / 2.0, y + s / 2.0);
    match kind {
        Forward => {
            draw_triangle(vec2(m.x, y + 2.0), vec2(x + 2.0, m.y + 1.0), vec2(x + s - 2.0, m.y + 1.0), c);
            draw_rectangle(m.x - s * 0.15, m.y, s * 0.3, s / 2.0 - 2.0, c);
        }
        TurnLeft | TurnRight => {
            let left = kind == TurnLeft;
            let r = s * 0.32;
            draw_arc(m.x, m.y + 2.0, 20, r, if left { 270.0 } else { 0.0 }, 3.0, 270.0, c);
            let tip_x = if left { m.x - r - 1.5 } else { m.x + r + 1.5 };
            let ty = m.y + 2.0;
            draw_triangle(vec2(tip_x - 5.0, ty), vec2(tip_x + 5.0, ty), vec2(tip_x, ty + 6.0), c);
        }
        Mine => crate::art::draw_tex(&art.pickaxe, x + 1.0, y + 2.0, s - 2.0, (s - 2.0) * 10.0 / 12.0, WHITE),
        Place => {
            crate::art::draw_tex(&art.cobble, x + 3.0, y + 3.0, s - 6.0, s - 6.0, WHITE);
            draw_rectangle_lines(x + 3.0, y + 3.0, s - 6.0, s - 6.0, 2.0, BLACK);
        }
        Repeat | While => {
            draw_arc(m.x, m.y, 24, s * 0.33, 30.0, 3.0, 300.0, c);
            let a = 30f32.to_radians();
            let p = vec2(m.x + a.cos() * s * 0.38, m.y + a.sin() * s * 0.38);
            draw_triangle(vec2(p.x - 6.0, p.y - 1.0), vec2(p.x + 4.0, p.y - 3.0), vec2(p.x + 1.0, p.y + 6.0), c);
        }
        If | IfElse => {
            text("?", x + s * 0.2, y + s * 0.05, s * 1.0, c);
        }
        Call => {
            text("f()", x - 2.0, y + s * 0.25, s * 0.62, c);
        }
    }
}
