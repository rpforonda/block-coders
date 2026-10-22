//! Where everything on the play screen goes, for any screen shape.
//!
//! The game draws on a virtual canvas. On a big enough screen that's the
//! desktop's 1280x800, scaled to fit. Shrinking it onto a phone would make
//! normal text 6-9pt, so below `MIN_SCALE` the canvas takes the screen's own
//! shape at a readable scale instead, and the play screen is laid out for it:
//!
//! - **Wide** (desktop, tablets, phones held sideways): the desktop's three
//!   columns, resized. At 1280x800 this is exactly the desktop layout.
//! - **Tall** (phones held upright): the world on top, then Bit, then blocks
//!   and program side by side, with RUN / STEP / RESET along the bottom. While
//!   a program runs, the blocks step aside and the program fills the width, so
//!   the running block is big and right under the miner.
//!
//! TypeSafe weighed these against tabs and a blocks drawer for 8-12 year olds
//! new to computers (see docs/DESIGN.md, "Small screens").

use crate::ui::{VH, VW};
use macroquad::math::Rect;

/// The smallest the desktop canvas may be shrunk. Below this, normal text
/// would be under about 15pt, so the canvas changes shape instead.
pub const MIN_SCALE: f32 = 0.8;
/// The smallest canvas the layouts are designed for. Smaller screens (phones
/// held sideways are the usual case: about 375-430 points tall) get this,
/// shrunk to fit, so their text is a little smaller (about 14pt) but
/// everything still fits.
pub const MIN_W: f32 = 440.0;
pub const MIN_H: f32 = 520.0;
/// Wide canvases narrower than this get slimmer blocks and program columns.
const SLIM_W: f32 = 1100.0;

/// Screens the layouts are tested on: (name, logical width, logical height).
#[cfg(test)]
pub const SAMPLE_SCREENS: &[(&str, f32, f32)] = &[
    ("desktop", 1280.0, 800.0),
    ("big monitor", 1920.0, 1080.0),
    ("laptop", 1366.0, 768.0),
    ("small window", 900.0, 600.0),
    ("iPhone SE upright", 375.0, 667.0),
    ("iPhone upright", 390.0, 844.0),
    ("Android upright", 412.0, 915.0),
    ("iPhone sideways", 844.0, 390.0),
    ("iPhone SE sideways", 667.0, 375.0),
    ("iPad mini upright", 744.0, 1133.0),
    ("small tablet", 1024.0, 600.0),
    ("tiny phone", 320.0, 568.0),
];

/// The canvas size and its scale for a screen of `sw` x `sh` logical pixels.
pub fn canvas(sw: f32, sh: f32) -> (f32, f32, f32) {
    let fit = (sw / VW).min(sh / VH);
    if fit >= MIN_SCALE {
        return (VW, VH, fit);
    }
    let scale = MIN_SCALE.min(sw / MIN_W).min(sh / MIN_H);
    (sw / scale, sh / scale, scale)
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Shape {
    Wide,
    Tall,
}

/// One text item: where it goes and how big.
#[derive(Clone, Copy, Debug)]
pub struct Label {
    pub x: f32,
    pub y: f32,
    pub size: f32,
}

#[derive(Clone, Debug)]
pub struct Layout {
    pub w: f32,
    pub h: f32,
    pub shape: Shape,
    /// Height of the top bar.
    pub top_h: f32,
    pub levels: Rect,
    pub code: Rect,
    pub help: Rect,
    pub sound: Rect,
    pub title: Label,
    pub subtitle: Label,
    /// Where the stars go, if there's room beside the title.
    pub stars_x: Option<f32>,
    pub world: Rect,
    pub hud: Rect,
    pub guide: Rect,
    pub hint: Rect,
    pub goal: Rect,
    /// The blocks palette (hidden while a program runs in the Tall layout).
    pub palette: Option<Rect>,
    pub controls: Rect,
    pub run: Rect,
    pub step: Rect,
    pub reset: Rect,
    pub speed: Rect,
    pub editor: Rect,
}

impl Layout {
    /// The layout for a `w` x `h` canvas. `running`: a program is running
    /// (or has just finished), which in the Tall layout gives the program the
    /// blocks' space.
    pub fn new(w: f32, h: f32, running: bool) -> Layout {
        if w >= h { Layout::wide(w, h) } else { Layout::tall(w, h, running) }
    }

    /// The desktop arrangement, stretched to the canvas. Every size here is
    /// the desktop's own number at 1280x800.
    fn wide(w: f32, h: f32) -> Layout {
        let (pal_w, ed_w) = if w >= SLIM_W { (196.0, 302.0) } else { (174.0, 270.0) };
        let guide_h = (h - 620.0).clamp(130.0, 180.0);
        let editor = Rect::new(w - 16.0 - ed_w, 68.0, ed_w, h - 84.0);
        let palette = Rect::new(editor.x - 14.0 - pal_w, 68.0, pal_w, h - 252.0);
        let world = Rect::new(16.0, 68.0, palette.x - 32.0, h - 140.0 - guide_h);
        let hud = Rect::new(16.0, world.y + world.h + 10.0, world.w, 38.0);
        let guide = Rect::new(16.0, hud.y + hud.h + 8.0, world.w, guide_h);
        let controls = Rect::new(palette.x, palette.y + palette.h + 12.0, pal_w, 156.0);
        let (run, step, reset, speed) = column_controls(controls);
        let hint = Rect::new(guide.x + 104.0, guide.y + guide.h - 42.0, 110.0, 32.0);
        Layout {
            w,
            h,
            shape: Shape::Wide,
            top_h: 56.0,
            levels: Rect::new(14.0, 10.0, 110.0, 36.0),
            code: Rect::new(w - 480.0, 10.0, 150.0, 36.0),
            help: Rect::new(w - 318.0, 10.0, 110.0, 36.0),
            sound: Rect::new(w - 196.0, 10.0, 180.0, 36.0),
            title: Label { x: 142.0, y: 8.0, size: 28.0 },
            subtitle: Label { x: 143.0, y: 36.0, size: 16.0 },
            stars_x: Some(w - 580.0),
            world,
            hud,
            guide,
            hint,
            goal: Rect { x: hint.x + 122.0, ..hint },
            palette: Some(palette),
            controls,
            run,
            step,
            reset,
            speed,
            editor,
        }
    }

    /// Phones held upright: everything stacked, top to bottom.
    fn tall(w: f32, h: f32, running: bool) -> Layout {
        let m = 12.0;
        let inner = w - 2.0 * m;
        // Top bar: four buttons in a row, then the level's name under them.
        let bw = (inner - 3.0 * 8.0) / 4.0;
        let button = |i: f32| Rect::new(m + i * (bw + 8.0), 8.0, bw, 40.0);
        let top_h = 96.0;
        // Shorter phones give the world and Bit a little less, so the program has room.
        let short = h < 900.0;
        let world_h = ((h - top_h) * if short { 0.22 } else { 0.26 }).clamp(160.0, 420.0);
        let world = Rect::new(m, top_h + 4.0, inner, world_h);
        let hud = Rect::new(m, world.y + world.h + 8.0, inner, 38.0);
        let guide = Rect::new(m, hud.y + hud.h + 8.0, inner, if short { 124.0 } else { 140.0 });
        // RUN / STEP / RESET / speed in one bar along the bottom.
        let controls = Rect::new(m, h - m - 64.0, inner, 64.0);
        let cw = controls.w - 20.0 - 3.0 * 8.0;
        let mut x = controls.x + 10.0;
        let mut next = |share: f32| {
            let r = Rect::new(x, controls.y + 8.0, cw * share, 48.0);
            x += r.w + 8.0;
            r
        };
        let (run, step, reset, speed) = (next(0.34), next(0.2), next(0.2), next(0.26));
        // Blocks and program side by side in what's left.
        let y0 = guide.y + guide.h + 10.0;
        let rest = controls.y - 10.0 - y0;
        let pal_w = 174.0;
        let (palette, editor) = if running {
            (None, Rect::new(m, y0, inner, rest))
        } else {
            (Some(Rect::new(m, y0, pal_w, rest)), Rect::new(m + pal_w + 10.0, y0, inner - pal_w - 10.0, rest))
        };
        // Hint and Goal stack under Bit, so the message gets the panel's full height.
        let hint = Rect::new(guide.x + 10.0, guide.y + 52.0, 84.0, 32.0);
        Layout {
            w,
            h,
            shape: Shape::Tall,
            top_h,
            levels: button(0.0),
            code: button(1.0),
            help: button(2.0),
            sound: button(3.0),
            title: Label { x: m, y: 54.0, size: 20.0 },
            subtitle: Label { x: m, y: 76.0, size: 16.0 },
            stars_x: None,
            world,
            hud,
            guide,
            hint,
            goal: Rect { y: hint.y + hint.h + 4.0, ..hint },
            palette,
            controls,
            run,
            step,
            reset,
            speed,
            editor,
        }
    }

    /// Where the palette lists its blocks, under its title.
    pub fn palette_list(&self) -> Option<Rect> {
        self.palette.map(|p| Rect::new(p.x, p.y + 38.0, p.w, p.h - 40.0))
    }

    /// A centred popup of up to `w` x `h`, at most `y` from the top (the
    /// desktop's position), shrunk to fit small canvases.
    pub fn popup(&self, w: f32, h: f32, y: f32) -> Rect {
        let (w, h) = (w.min(self.w - 24.0), h.min(self.h - 24.0));
        Rect::new((self.w - w) / 2.0, y.min((self.h - h) / 2.0), w, h)
    }
}

/// RUN on top, STEP and RESET side by side, then speed: the desktop's column.
fn column_controls(c: Rect) -> (Rect, Rect, Rect, Rect) {
    let half = (c.w - 30.0) / 2.0;
    let run = Rect::new(c.x + 10.0, c.y + 10.0, c.w - 20.0, 50.0);
    let step = Rect::new(c.x + 10.0, run.y + 60.0, half, 36.0);
    let reset = Rect::new(c.x + 20.0 + half, run.y + 60.0, half, 36.0);
    let speed = Rect::new(c.x + 10.0, run.y + 106.0, c.w - 20.0, 32.0);
    (run, step, reset, speed)
}
