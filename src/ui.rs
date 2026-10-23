//! Small immediate-mode UI helpers: buttons, panels and text.

use crate::art::shade;
use macroquad::prelude::*;
use std::cell::{Cell, RefCell};
use std::collections::HashSet;
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

/// Which font to draw in.
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum Face {
    /// Atkinson Hyperlegible: everything kids read.
    Text,
    /// Atkinson Hyperlegible Mono: the Rust code view, which lines letters up.
    Code,
}

thread_local! {
    /// The (font, character, rasterized size) glyphs macroquad has cached so
    /// far. It never forgets one, and all text goes through `text`,
    /// `text_width`, `code_text` and `code_width`.
    static CACHED: RefCell<HashSet<(Face, char, u16)>> = RefCell::new(HashSet::new());
    /// The two fonts, once loaded (see `load_fonts`). They're leaked, never
    /// dropped: dropping one at exit deletes its texture after the window and
    /// its GL context are gone, which crashed the game every time it quit.
    static FONTS: Cell<Option<&'static (Font, Font)>> = const { Cell::new(None) };
}

/// Loads the fonts: Atkinson Hyperlegible (Braille Institute), designed so
/// letters never look alike, and its Mono for code. Both are SIL Open Font
/// License (assets/fonts/). Chosen with TypeSafe for legibility (docs/DESIGN.md).
/// Until this runs (and in tests), text uses macroquad's built-in font.
pub fn load_fonts() {
    let text = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/AtkinsonHyperlegible-Regular.ttf"));
    let code = load_ttf_font_from_bytes(include_bytes!("../assets/fonts/AtkinsonHyperlegibleMono.ttf"));
    if let (Ok(t), Ok(c)) = (text, code) {
        FONTS.set(Some(Box::leak(Box::new((t, c)))));
    }
}

fn with_font<R>(face: Face, f: impl FnOnce(Option<&Font>) -> R) -> R {
    f(FONTS.get().map(|(t, c)| if face == Face::Code { c } else { t }))
}

/// macroquad 0.4.16 keeps glyphs in one font atlas texture. When a new glyph
/// makes the atlas grow, it deletes the old texture straight away, while text
/// already queued this frame (even earlier letters of the same string) still
/// draws with it: "glBindTexture called with an already deleted texture".
/// On top of that, miniquad 0.4.11 doesn't forget a deleted texture's binding,
/// and creating the new atlas "restores" it, binding the deleted texture again.
/// So before text with a glyph it hasn't cached, draw everything queued so far
/// and clear miniquad's bindings, then cache the whole string before any of it
/// is drawn. Text that's already cached (nearly all of it, after the first
/// frames) costs a set lookup.
fn cache_glyphs(face: Face, s: &str, font_size: u16, font_scale: f32) {
    let px = (font_size as f32 * screen_dpi_scale()).ceil() as u16; // as macroquad rasterizes it
    let fresh = CACHED.with_borrow_mut(|cached| {
        let before = cached.len();
        cached.extend(s.chars().map(|c| (face, c, px)));
        cached.len() != before
    });
    if fresh {
        // SAFETY: only draws the batched calls (as set_camera does every frame)
        // and unbinds textures (as miniquad does at the end of every frame).
        let mut gl = unsafe { get_internal_gl() };
        gl.flush();
        gl.quad_context.commit_frame();
        with_font(face, |font| measure_text(s, font, font_size, font_scale));
    }
}

/// The font size and scale to rasterize text of logical `size` sharply.
fn font_params(size: f32) -> (u16, f32) {
    let sc = scale();
    ((size * sc).round().max(1.0) as u16, 1.0 / sc)
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
        let bg = readable_bg(color);
        let tc = text_on(bg);
        let light_text = tc == WHITE;
        bevel(r, if self.hover(r) { hover_shade(bg, light_text) } else { bg });
        text_centered(label, r, size, tc, light_text);
        self.clicked(r)
    }

    /// A button that can be greyed out.
    pub fn button_if(&mut self, on: bool, r: Rect, label: &str, color: Color, size: f32) -> bool {
        if on {
            self.button(r, label, color, size)
        } else {
            bevel(r, rgb8(90, 90, 90));
            text_centered(label, r, size, DISABLED_TEXT, false);
            false
        }
    }
}

/// Width of a scrollbar, wide enough to grab with a finger on a touch screen.
pub const SCROLLBAR_W: f32 = 14.0;

/// A vertical scrollbar for a list taller than the box it's shown in. Drag
/// the thumb, or click the track above or below it to move a page. This also
/// works where there's no mouse wheel (touch screens, some trackpads).
#[derive(Default)]
pub struct Scrollbar {
    /// While the thumb is held: the mouse y and the scroll when it was grabbed.
    grab: Option<(f32, f32)>,
}

impl Scrollbar {
    /// Handles the mouse and draws the bar in `track`. Call it before drawing
    /// the list, so a click on the bar never reaches the list. Does nothing
    /// (and draws nothing) when `content` fits in `view`.
    pub fn update(&mut self, ui: &mut Ui, track: Rect, content: f32, view: f32, scroll: &mut f32) {
        let Some(thumb) = self.handle(ui, track, content, view, scroll) else { return };
        draw_rectangle(track.x, track.y, track.w, track.h, Color::new(0.0, 0.0, 0.0, 0.35));
        let lit = self.grab.is_some() || ui.hover(thumb);
        draw_rectangle(thumb.x + 2.0, thumb.y, thumb.w - 4.0, thumb.h, if lit { rgb8(230, 230, 240) } else { rgb8(160, 160, 175) });
    }

    /// The mouse handling of `update`, without drawing. Returns where the
    /// thumb is, or None when there's nothing to scroll.
    pub fn handle(&mut self, ui: &mut Ui, track: Rect, content: f32, view: f32, scroll: &mut f32) -> Option<Rect> {
        let max = content - view;
        if max <= 0.0 {
            self.grab = None;
            *scroll = 0.0;
            return None;
        }
        let thumb_h = (track.h * view / content).max(36.0).min(track.h);
        let travel = track.h - thumb_h;
        let thumb = Rect::new(track.x, track.y + travel * (*scroll / max).clamp(0.0, 1.0), track.w, thumb_h);
        match self.grab {
            Some((y0, s0)) if ui.down => *scroll = s0 + (ui.mouse.y - y0) * max / travel.max(1.0),
            Some(_) => self.grab = None,
            None if ui.clicked(thumb) => self.grab = Some((ui.mouse.y, *scroll)),
            None if ui.clicked(track) => *scroll += if ui.mouse.y < thumb.y { -view } else { view } * 0.8,
            None => {}
        }
        *scroll = scroll.clamp(0.0, max);
        Some(Rect::new(track.x, track.y + travel * (*scroll / max), track.w, thumb_h))
    }
}

pub fn rgb8(r: u8, g: u8, b: u8) -> Color {
    Color::from_rgba(r, g, b, 255)
}

/// Titles on the light grey popups (`panel`): dark amber, 4.7:1.
pub const TITLE_ON_LIGHT: Color = Color { r: 125.0 / 255.0, g: 65.0 / 255.0, b: 0.0, a: 1.0 };
/// "Not quite" titles on the light popups: dark orange-red, 4.9:1.
pub const WARN_ON_LIGHT: Color = Color { r: 140.0 / 255.0, g: 45.0 / 255.0, b: 5.0 / 255.0, a: 1.0 };
/// Grey text on the dark grey of something locked, 5:1.
pub const LOCKED_TEXT: Color = Color { r: 180.0 / 255.0, g: 180.0 / 255.0, b: 180.0 / 255.0, a: 1.0 };
/// The light grey of `panel`.
pub const PANEL: Color = Color { r: 198.0 / 255.0, g: 198.0 / 255.0, b: 198.0 / 255.0, a: 1.0 };
/// The dark grey of `dark_panel`.
pub const DARK_PANEL: Color = Color { r: 40.0 / 255.0, g: 40.0 / 255.0, b: 48.0 / 255.0, a: 1.0 };

/// Near-black text, for light backgrounds.
pub const DARK_TEXT: Color = Color { r: 18.0 / 255.0, g: 16.0 / 255.0, b: 10.0 / 255.0, a: 1.0 };
/// Text on a greyed-out button. Inactive controls are exempt from the
/// contrast minimum, but it's kept as readable as it can be while still
/// looking switched off.
const DISABLED_TEXT: Color = Color { r: 170.0 / 255.0, g: 170.0 / 255.0, b: 170.0 / 255.0, a: 1.0 };
/// The contrast text must have against its background: WCAG AA for normal
/// text (4.5:1), plus a little margin.
pub const MIN_CONTRAST: f32 = 4.6;
/// How dark a colour may be made so white text reads on it. Colours that
/// would need more (bright yellows) keep their colour and get dark text.
const MAX_DARKEN: f32 = 0.78;

/// WCAG relative luminance.
fn rel_lum(c: Color) -> f32 {
    let lin = |v: f32| if v <= 0.03928 { v / 12.92 } else { ((v + 0.055) / 1.055).powf(2.4) };
    0.2126 * lin(c.r) + 0.7152 * lin(c.g) + 0.0722 * lin(c.b)
}

/// WCAG contrast ratio between two colours, from 1 (same) to 21 (black on white).
pub fn contrast(a: Color, b: Color) -> f32 {
    let (la, lb) = (rel_lum(a), rel_lum(b));
    (la.max(lb) + 0.05) / (la.min(lb) + 0.05)
}

/// `bg`, darkened just enough for white text to read on it, unless that would
/// change it a lot (bright yellows and golds), in which case it stays as it is
/// and gets dark text instead (see `text_on`).
pub fn readable_bg(bg: Color) -> Color {
    if contrast(bg, WHITE) >= MIN_CONTRAST {
        return bg;
    }
    let mut f = 1.0;
    while f > MAX_DARKEN && contrast(shade(bg, f), WHITE) < MIN_CONTRAST {
        f -= 0.01;
    }
    let darker = shade(bg, f);
    if contrast(darker, WHITE) >= MIN_CONTRAST || contrast(bg, DARK_TEXT) < MIN_CONTRAST {
        // Mid-tones that dark text can't fix either keep darkening until white reads.
        let mut f = f;
        while contrast(shade(bg, f), WHITE) < MIN_CONTRAST {
            f -= 0.01;
        }
        shade(bg, f)
    } else {
        bg
    }
}

/// White or near-black, whichever reads better on `bg`.
pub fn text_on(bg: Color) -> Color {
    if contrast(bg, WHITE) >= contrast(bg, DARK_TEXT) { WHITE } else { DARK_TEXT }
}

/// A hovered button: moved away from its text colour, so hovering never makes
/// the label harder to read.
pub fn hover_shade(bg: Color, light_text: bool) -> Color {
    if light_text { shade(bg, 0.86) } else { shade(bg, 1.12) }
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
    draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
    draw_rectangle(r.x, r.y, r.w, 3.0, WHITE);
    draw_rectangle(r.x, r.y, 3.0, r.h, WHITE);
    draw_rectangle(r.x, r.y + r.h - 3.0, r.w, 3.0, rgb8(85, 85, 85));
    draw_rectangle(r.x + r.w - 3.0, r.y, 3.0, r.h, rgb8(85, 85, 85));
}

pub fn dark_panel(r: Rect) {
    draw_rectangle(r.x - 3.0, r.y - 3.0, r.w + 6.0, r.h + 6.0, BLACK);
    draw_rectangle(r.x, r.y, r.w, r.h, DARK_PANEL);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, rgb8(90, 90, 105));
}

/// Draws text with its top-left corner at (x, y). Text is rendered at the real
/// screen resolution so it stays sharp when the window is scaled.
pub fn text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_in(Face::Text, s, x, y, size, c);
}

/// Like `text`, in the monospaced code font.
pub fn code_text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    draw_in(Face::Code, s, x, y, size, c);
}

/// How far below the top of a line its baseline sits, as a share of the size.
const BASELINE: f32 = 0.78;

fn draw_in(face: Face, s: &str, x: f32, y: f32, size: f32, c: Color) {
    let (font_size, font_scale) = font_params(size);
    cache_glyphs(face, s, font_size, font_scale);
    with_font(face, |font| {
        draw_text_ex(s, x, y + size * BASELINE, TextParams { font, font_size, font_scale, color: c, ..Default::default() });
    });
}

pub fn shadow_text(s: &str, x: f32, y: f32, size: f32, c: Color) {
    let o = (size / 12.0).max(1.5);
    text(s, x + o, y + o, size, shade(c, 0.25));
    text(s, x, y, size, c);
}

pub fn text_width(s: &str, size: f32) -> f32 {
    width_in(Face::Text, s, size)
}

pub fn code_width(s: &str, size: f32) -> f32 {
    width_in(Face::Code, s, size)
}

fn width_in(face: Face, s: &str, size: f32) -> f32 {
    let (font_size, font_scale) = font_params(size);
    cache_glyphs(face, s, font_size, font_scale);
    with_font(face, |font| measure_text(s, font, font_size, font_scale).width)
}

/// Draws `s` centred in `r`. `\n` starts a new line; the lines are centred together.
pub fn text_centered(s: &str, r: Rect, size: f32, c: Color, shadow: bool) {
    let lh = size * 1.1;
    let lines: Vec<&str> = s.split('\n').collect();
    let top = r.y + (r.h - size - (lines.len() - 1) as f32 * lh) / 2.0 + 1.0;
    for (i, line) in lines.iter().enumerate() {
        let x = r.x + (r.w - text_width(line, size)) / 2.0;
        let y = top + i as f32 * lh;
        if shadow { shadow_text(line, x, y, size, c) } else { text(line, x, y, size, c) }
    }
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

/// The biggest size from `size` down to `min` at which `s` fits in `width`
/// on one line (`min` if it never does).
pub fn fit_size(s: &str, width: f32, size: f32, min: f32) -> f32 {
    let w = text_width(s, size);
    if w <= width { size } else { (size * width / w).floor().max(min) }
}

/// `fit_size`, in the code font.
pub fn fit_code_size(s: &str, width: f32, size: f32, min: f32) -> f32 {
    let w = code_width(s, size);
    if w <= width { size } else { (size * width / w).floor().max(min) }
}

/// Like `paragraph`, but each line is centred in `width`.
pub fn centered_paragraph(s: &str, x: f32, y: f32, width: f32, size: f32, c: Color) -> f32 {
    let lh = size * 1.3;
    let lines = wrap(s, width, size);
    for (i, l) in lines.iter().enumerate() {
        text(l, x + (width - text_width(l, size)) / 2.0, y + i as f32 * lh, size, c);
    }
    lines.len() as f32 * lh
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
/// A block's icon in `ink` (the block's text colour, so it reads as well as the label).
pub fn block_icon(kind: crate::program::Kind, x: f32, y: f32, s: f32, ink: Color, art: &crate::art::Art) {
    use crate::program::Kind::*;
    let c = ink;
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
        // A little box: a variable is a box that holds a number.
        Set | Add => {
            draw_rectangle_lines(x + 2.0, y + 4.0, s - 4.0, s - 6.0, 2.0, c);
            let sign = if kind == Set { "=" } else { "+" };
            text(sign, m.x - s * 0.18, y + s * 0.12, s * 0.8, c);
        }
        RepeatCount => {
            draw_arc(m.x, m.y, 24, s * 0.33, 30.0, 3.0, 300.0, c);
            text("n", m.x - s * 0.13, m.y - s * 0.3, s * 0.5, c);
        }
    }
}
