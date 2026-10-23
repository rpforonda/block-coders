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
//!   the running block is big and right under the miner. The moment it stops,
//!   the blocks come back.
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
    /// The layout for a `w` x `h` canvas. `running`: a program is running or
    /// paused, which in the Tall layout gives the program the blocks' space.
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
        popup((self.w, self.h), w, h, y)
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

// ------------------------------------------------------------------ menus
//
// The title, players, level select and World Check screens, laid out the
// same way: at 1280x800 every rectangle is the desktop's own, and smaller
// canvases reshape. The phone designs were weighed with TypeSafe (see
// docs/DESIGN.md, "Small screens").

/// A canvas: (width, height).
pub type Canvas = (f32, f32);

/// Phones held upright get the stacked screens.
pub fn is_tall(c: Canvas) -> bool {
    c.0 < c.1
}

/// A popup of up to `w` x `h`, centred on the canvas, at most `y` from the top
/// (the desktop's position), shrunk to fit.
pub fn popup(c: Canvas, w: f32, h: f32, y: f32) -> Rect {
    let (w, h) = (w.min(c.0 - 24.0), h.min(c.1 - 24.0));
    Rect::new((c.0 - w) / 2.0, y.min((c.1 - h) / 2.0), w, h)
}

/// The title screen.
pub struct TitleLayout {
    pub title_y: f32,
    pub title_size: f32,
    pub subtitle: Rect,
    pub miner: Rect,
    pub bit: Rect,
    pub play: Rect,
    /// The one-line description, wrapped to this width.
    pub tagline: Rect,
}

pub fn title(c: Canvas) -> TitleLayout {
    let (w, h) = c;
    // Desktop offsets from the centre, squeezed on short screens.
    let k = (h / VH).clamp(0.6, 1.0);
    let (cx, cy) = (w / 2.0, h / 2.0);
    // On the narrowest screens the miner and Bit get smaller and sit at the edges.
    let (mw, bw) = if cx - 230.0 < 12.0 { (72.0, 72.0) } else { (96.0, 96.0) };
    TitleLayout {
        title_y: cy - 260.0 * k,
        title_size: 120.0 * k.max(0.7),
        subtitle: Rect::new(0.0, cy - 110.0 * k, w, 40.0),
        miner: Rect::new((cx - 230.0).max(12.0), cy - 40.0 * k, mw, mw * 4.0 / 3.0),
        bit: Rect::new((cx + 140.0).min(w - 12.0 - bw), cy - 20.0 * k, bw, bw),
        play: Rect::new(cx - 120.0, cy, 240.0, 70.0),
        tagline: Rect::new(20.0, cy + 140.0 * k, w - 40.0, 60.0),
    }
}

/// The "Who's playing?" screen.
pub struct UsersLayout {
    pub back: Rect,
    /// "Who's playing?", centred in this.
    pub title: Rect,
    pub title_size: f32,
    pub cards: Vec<Rect>,
    /// Cards too small for the desktop design get the one-row design.
    pub compact: bool,
    /// The tip at the bottom, if there's room.
    pub tip: Option<Rect>,
}

/// Cards for `n` players (plus the "New player" card, counted in `n`).
pub fn users(c: Canvas, n: usize) -> UsersLayout {
    let (w, h) = c;
    if w >= 1150.0 && h >= 760.0 {
        // The desktop: three columns of big cards.
        let left = (w - 3.0 * 360.0 - 2.0 * 30.0) / 2.0;
        let cards = (0..n).map(|i| Rect::new(left + (i % 3) as f32 * 390.0, 130.0 + (i / 3) as f32 * 280.0, 360.0, 250.0)).collect();
        return UsersLayout {
            back: Rect::new(20.0, 18.0, 110.0, 40.0),
            title: Rect::new(0.0, 40.0, w, 60.0),
            title_size: 52.0,
            cards,
            compact: false,
            tip: Some(Rect::new(0.0, h - 58.0, w, 30.0)),
        };
    }
    // Everything else: the fewest columns where every card fits without
    // scrolling. Cards shrink (never below 96px) rather than scroll.
    let (m, gap, top) = (12.0, 10.0, 110.0);
    let n = n.max(1);
    let fit = |tip: f32| {
        (1..=3).find_map(|cols| {
            let card_w = ((w - 2.0 * m - (cols - 1) as f32 * gap) / cols as f32).min(380.0);
            let rows = n.div_ceil(cols);
            let card_h = ((h - top - m - tip - (rows - 1) as f32 * gap) / rows as f32).min(250.0);
            (card_w >= 300.0 && card_h >= 96.0).then_some((cols, card_w, card_h))
        })
    };
    let (tip, (cols, card_w, card_h)) = match fit(48.0) {
        Some(f) => (true, f),
        // No room for the tip: drop it before shrinking cards further.
        None => (false, fit(0.0).unwrap_or((1, (w - 2.0 * m).min(380.0), 96.0))),
    };
    let left = (w - cols as f32 * card_w - (cols - 1) as f32 * gap) / 2.0;
    let cards = (0..n)
        .map(|i| Rect::new(left + (i % cols) as f32 * (card_w + gap), top + (i / cols) as f32 * (card_h + gap), card_w, card_h))
        .collect();
    UsersLayout {
        back: Rect::new(m, 10.0, 110.0, 40.0),
        title: Rect::new(0.0, 56.0, w, 44.0),
        title_size: 40.0,
        cards,
        compact: card_h < 200.0 || card_w < 340.0,
        tip: tip.then(|| Rect::new(m, h - m - 44.0, w - 2.0 * m, 44.0)),
    }
}

/// The level select screen, for wide canvases: worlds down the left, the
/// chosen world's levels in a grid on the right.
pub struct SelectWide {
    pub back: Rect,
    pub player_x: f32,
    pub worlds: Vec<Rect>,
    pub panel: Rect,
    /// Space for the world's name above the grid.
    pub header_h: f32,
    pub tile: f32,
    pub cols: usize,
    pub gap_x: f32,
    pub gap_y: f32,
    pub info: Rect,
}

impl SelectWide {
    pub fn tile_rect(&self, n: usize) -> Rect {
        Rect::new(
            self.panel.x + 30.0 + (n % self.cols) as f32 * (self.tile + self.gap_x),
            self.panel.y + self.header_h + (n / self.cols) as f32 * (self.tile + self.gap_y),
            self.tile,
            self.tile,
        )
    }
}

/// `tiles`: how many tiles the chosen world has (levels plus its World Check).
pub fn select_wide(c: Canvas, worlds: usize, tiles: usize) -> SelectWide {
    let (w, h) = c;
    let step = (h - 93.0) / worlds as f32;
    // Narrower screens get a slimmer column of worlds, so the levels have room.
    let col = if w >= SLIM_W { 330.0 } else { 240.0 };
    let world_rects = (0..worlds).map(|i| Rect::new(20.0, 80.0 + i as f32 * step, col, step - 9.0)).collect();
    let panel = Rect::new(col + 50.0, 84.0, w - col - 70.0, h - 140.0);
    let short = panel.h < 500.0;
    let (header_h, gap_x, gap_y) = if short { (56.0, 20.0, 16.0) } else { (80.0, 20.0, 32.0) };
    // The columns that give the biggest tiles (148 at most); on a tie, more
    // columns (so the desktop keeps its 5).
    let size = |cols: usize| {
        let rows = tiles.max(1).div_ceil(cols);
        let by_w = (panel.w - 60.0 - (cols - 1) as f32 * gap_x) / cols as f32;
        let by_h = (panel.h - header_h - 10.0 - (rows - 1) as f32 * gap_y) / rows as f32;
        by_w.min(by_h).min(148.0)
    };
    let cols = (3..=8).max_by(|&a, &b| size(a).total_cmp(&size(b)).then(a.cmp(&b))).unwrap_or(5);
    SelectWide {
        back: Rect::new(20.0, 18.0, 170.0, 40.0),
        player_x: (w - 640.0).max(500.0),
        worlds: world_rects,
        panel,
        header_h,
        tile: size(cols),
        cols,
        gap_x,
        gap_y,
        info: Rect::new(0.0, h - 40.0, w, 28.0),
    }
}

/// The level select screen on a phone held upright: two pages, a list of
/// worlds and then one world's levels.
pub struct SelectTall {
    pub back: Rect,
    pub worlds: Vec<Rect>,
    pub tile: f32,
    pub grid_x: f32,
    pub grid_y: f32,
    pub gap: f32,
    pub info: Rect,
}

impl SelectTall {
    pub fn tile_rect(&self, n: usize) -> Rect {
        Rect::new(self.grid_x + (n % 3) as f32 * (self.tile + self.gap), self.grid_y + (n / 3) as f32 * (self.tile + self.gap), self.tile, self.tile)
    }
}

pub fn select_tall(c: Canvas, worlds: usize, tiles: usize) -> SelectTall {
    let (w, h) = c;
    let (m, gap) = (12.0, 12.0);
    let top = 120.0;
    let step = (h - top - m) / worlds as f32;
    let world_rects = (0..worlds).map(|i| Rect::new(m, top + i as f32 * step, w - 2.0 * m, step - 8.0)).collect();
    let rows = tiles.max(1).div_ceil(3);
    let info_h = 44.0;
    let tile = ((w - 2.0 * m - 2.0 * gap) / 3.0).min((h - top - info_h - m - 8.0 - (rows - 1) as f32 * gap) / rows as f32).min(148.0);
    SelectTall {
        back: Rect::new(m, 10.0, 140.0, 44.0),
        worlds: world_rects,
        tile,
        grid_x: (w - 3.0 * tile - 2.0 * gap) / 2.0,
        grid_y: top,
        gap,
        info: Rect::new(m, h - m - info_h, w - 2.0 * m, info_h),
    }
}

/// What kind of answer a World Check question takes.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Answer {
    Text,
    Spots,
    Programs,
}

/// Picking one program at a time (phones): the shown program, arrows to the
/// others, and a button to pick it.
#[derive(Clone, Copy, Debug)]
pub struct Carousel {
    pub prev: Rect,
    pub label: Rect,
    pub next: Rect,
    pub pick: Rect,
}

/// A World Check question screen.
#[derive(Clone, Debug)]
pub struct CheckLayout {
    pub header_h: f32,
    pub back: Rect,
    pub title: Label,
    /// The question pips: left edge, top, size; they step by size + 6.
    pub pips: (f32, f32, f32),
    pub bit: Rect,
    pub counter: Label,
    /// The question, wrapped to this width.
    pub prompt: Rect,
    pub prompt_size: f32,
    pub program: Option<Rect>,
    pub map: Option<Rect>,
    /// One box per answer, or one box for the program being shown (`carousel`).
    pub choices: Vec<Rect>,
    pub carousel: Option<Carousel>,
    /// Bit's explanation after answering, and the Next button inside it.
    pub explain: Rect,
    pub next: Rect,
}

pub fn check(c: Canvas, answer: Answer, n: usize, has_program: bool, has_map: bool) -> CheckLayout {
    let (w, h) = c;
    if is_tall(c) {
        return check_tall(c, answer, n, has_program, has_map);
    }
    let short = h < 640.0;
    let header_h = if short { 56.0 } else { 64.0 };
    let pip = 22.0;
    let head = |y: f32| (Rect::new(16.0, (header_h - 40.0) / 2.0, 130.0, 40.0), Label { x: 168.0, y, size: 32.0 });
    let (back, title) = head((header_h - 36.0) / 2.0);
    let pips = (w - 20.0 - 10.0 * (pip + 6.0), (header_h - pip) / 2.0, pip);
    let explain = Rect::new(24.0, h - 88.0, w - 48.0, 80.0);
    let next = Rect::new(explain.x + explain.w - 196.0, explain.y + 16.0, 180.0, 48.0);
    if short {
        // Phones held sideways: the program and map on the left, the answers on the right.
        let top = header_h + 44.0;
        let col_w = (w - 3.0 * 16.0) / 2.0;
        // Bit's explanation goes under the program and map, so the answers
        // (often a whole program to read) get all of the right-hand column.
        let explain = Rect::new(16.0, h - 12.0 - 110.0, col_w, 110.0);
        let next = Rect::new(explain.x + explain.w - 150.0, explain.y + explain.h - 56.0, 140.0, 48.0);
        let left = Rect::new(16.0, top, col_w, explain.y - 10.0 - top);
        let right = Rect::new(32.0 + col_w, top, col_w, h - top - 12.0);
        let (program, map) = match (has_program, has_map) {
            (true, true) => {
                let half = (left.h - 10.0) / 2.0;
                (Some(Rect::new(left.x, left.y, left.w, half)), Some(Rect::new(left.x, left.y + half + 10.0, left.w, half)))
            }
            (true, false) => (Some(left), None),
            (false, true) => (None, Some(left)),
            (false, false) => (None, None),
        };
        let (choices, carousel) = answers_in(right, answer, n);
        return CheckLayout {
            header_h,
            back,
            title,
            pips,
            bit: Rect::new(16.0, header_h + 4.0, 36.0, 36.0),
            counter: Label { x: 60.0, y: header_h + 6.0, size: 16.0 },
            prompt: Rect::new(60.0, header_h + 22.0, w - 80.0, 20.0),
            prompt_size: 20.0,
            program,
            map,
            choices,
            carousel,
            explain,
            next,
        };
    }
    // The desktop arrangement (any wide screen tall enough for it).
    let top = 148.0;
    let middle = explain.y - 12.0 - top;
    let stretch = middle / 552.0;
    let ph = if answer == Answer::Programs { 260.0 * stretch } else { middle - 16.0 - 170.0 - 6.0 };
    let both = has_program && has_map;
    let wide = |x: f32, pw: f32| Rect::new(x, top, pw, ph);
    let centred = Rect::new(w / 2.0 - 400.0, top, 800.0, ph);
    let program = has_program.then(|| if both { wide(24.0, 600.0 * w / VW) } else { centred });
    let map = has_map.then(|| if both { Rect::new(48.0 + 600.0 * w / VW, top, w - 72.0 - 600.0 * w / VW, ph) } else { centred });
    let below = top + ph + 16.0;
    let choices = match answer {
        Answer::Text => (0..n).map(|i| Rect::new(w / 2.0 - 450.0, below + i as f32 * 60.0, 900.0, 50.0)).collect(),
        Answer::Spots => (0..n).map(|i| Rect::new(w / 2.0 - 330.0 + i as f32 * 230.0, below + 16.0, 200.0, 60.0)).collect(),
        Answer::Programs => {
            let bw = (w - 48.0 - (n - 1) as f32 * 16.0) / n as f32;
            (0..n).map(|i| Rect::new(24.0 + i as f32 * (bw + 16.0), below, bw.min(400.0), explain.y - 12.0 - below)).collect()
        }
    };
    CheckLayout {
        header_h,
        back,
        title,
        pips,
        bit: Rect::new(20.0, 76.0, 56.0, 56.0),
        counter: Label { x: 92.0, y: 76.0, size: 18.0 },
        prompt: Rect::new(92.0, 98.0, w - 120.0, 48.0),
        prompt_size: 24.0,
        program,
        map,
        choices,
        carousel: None,
        explain,
        next,
    }
}

/// Phones held upright: everything stacked, and programs to pick from shown
/// one at a time (TypeSafe's pick: readable 0.85 against 0.38 for stacking all three).
fn check_tall(c: Canvas, answer: Answer, n: usize, has_program: bool, has_map: bool) -> CheckLayout {
    let (w, h) = c;
    let m = 12.0;
    let inner = w - 2.0 * m;
    let short = h < 900.0;
    let header_h = if short { 92.0 } else { 100.0 };
    let pip = 22.0;
    let explain_h = if short { 104.0 } else { 140.0 };
    let explain = Rect::new(m, h - m - explain_h, inner, explain_h);
    let next = Rect::new(explain.x + explain.w - 150.0, explain.y + explain.h - 56.0, 140.0, 48.0);
    let prompt_h = if short { 60.0 } else { 90.0 };
    let top = header_h + prompt_h + 10.0;
    let bottom = explain.y - 10.0;
    // The answers first (they need a fixed size), then the program and map share what's left.
    let answers_h = match answer {
        Answer::Text => n as f32 * 58.0 - 8.0,
        Answer::Spots => 56.0,
        // With a program and a map to show too, the answers get half.
        Answer::Programs => ((bottom - top) * if has_program && has_map { 0.5 } else { 0.6 }).max(240.0),
    };
    let room = Rect::new(m, bottom - answers_h, inner, answers_h);
    let (choices, carousel) = answers_in(room, answer, n);
    let panels = room.y - 10.0 - top;
    let (program, map) = match (has_program, has_map) {
        (true, true) => {
            let half = (panels - 10.0) / 2.0;
            (Some(Rect::new(m, top, inner, half)), Some(Rect::new(m, top + half + 10.0, inner, half)))
        }
        (true, false) => (Some(Rect::new(m, top, inner, panels)), None),
        (false, true) => (None, Some(Rect::new(m, top, inner, panels))),
        (false, false) => (None, None),
    };
    CheckLayout {
        header_h,
        back: Rect::new(m, 8.0, 110.0, 40.0),
        title: Label { x: m + 122.0, y: 14.0, size: 26.0 },
        pips: ((w - 10.0 * (pip + 6.0) + 6.0) / 2.0, header_h - 34.0, pip),
        bit: Rect::new(m, header_h + 4.0, 44.0, 44.0),
        counter: Label { x: m + 56.0, y: header_h + 4.0, size: 16.0 },
        prompt: Rect::new(m + 56.0, header_h + 24.0, inner - 56.0, prompt_h - 24.0),
        prompt_size: 21.0,
        program,
        map,
        choices,
        carousel,
        explain,
        next,
    }
}

/// The answer boxes inside `room` for the compact layouts: text answers
/// stacked, square buttons in a row, and programs one at a time.
fn answers_in(room: Rect, answer: Answer, n: usize) -> (Vec<Rect>, Option<Carousel>) {
    match answer {
        Answer::Text => ((0..n).map(|i| Rect::new(room.x, room.y + i as f32 * 58.0, room.w, 50.0)).collect(), None),
        Answer::Spots => {
            let bw = (room.w - (n - 1) as f32 * 10.0) / n as f32;
            ((0..n).map(|i| Rect::new(room.x + i as f32 * (bw + 10.0), room.y, bw, 56.0)).collect(), None)
        }
        Answer::Programs => {
            let controls = 48.0;
            let pick_h = 52.0;
            let box_r = Rect::new(room.x, room.y, room.w, room.h - controls - pick_h - 16.0);
            let y = box_r.y + box_r.h + 8.0;
            let carousel = Carousel {
                prev: Rect::new(room.x, y, 64.0, controls),
                label: Rect::new(room.x + 72.0, y, room.w - 144.0, controls),
                next: Rect::new(room.x + room.w - 64.0, y, 64.0, controls),
                pick: Rect::new(room.x, y + controls + 8.0, room.w, pick_h),
            };
            (vec![box_r], Some(carousel))
        }
    }
}

/// One key of the on-screen keyboard.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum KeyCap {
    Char(char),
    Erase,
    Shift,
    Space,
    Hide,
}

/// The on-screen keyboard: its area and every key. Phones held upright get
/// phone-style rows ten keys wide; wider screens get the desktop rows,
/// narrower or shorter if needed.
pub fn keyboard(c: Canvas) -> (Rect, Vec<(KeyCap, Rect)>) {
    let (w, h) = c;
    let mut keys = Vec::new();
    if w < 900.0 {
        let (m, gap, key_h) = (6.0, 5.0, 52.0);
        let unit = ((w - 2.0 * m - 9.0 * gap) / 10.0).min(78.0);
        let area_h = 5.0 * (key_h + gap) + 12.0;
        let area = Rect::new(0.0, h - area_h, w, area_h);
        let row_y = |r: usize| area.y + 10.0 + r as f32 * (key_h + gap);
        let mut row = |r: usize, caps: &[(KeyCap, f32)]| {
            let total: f32 = caps.iter().map(|c| c.1).sum::<f32>() * unit + (caps.len() - 1) as f32 * gap;
            let mut x = (w - total) / 2.0;
            for &(cap, units) in caps {
                keys.push((cap, Rect::new(x, row_y(r), units * unit + (units - 1.0).max(0.0) * gap, key_h)));
                x += units * unit + (units - 1.0).max(0.0) * gap + gap;
            }
        };
        let chars = |s: &str| s.chars().map(|c| (KeyCap::Char(c), 1.0)).collect::<Vec<_>>();
        row(0, &chars("1234567890"));
        row(1, &chars("qwertyuiop"));
        row(2, &chars("asdfghjkl-"));
        let mut r3 = vec![(KeyCap::Shift, 1.4)];
        r3.extend(chars("zxcvbnm"));
        r3.push((KeyCap::Erase, 1.4));
        row(3, &r3);
        row(4, &[(KeyCap::Char('_'), 1.0), (KeyCap::Space, 5.4), (KeyCap::Hide, 2.0)]);
        return (area, keys);
    }
    // The desktop rows: at 1280x800 exactly the original keyboard.
    let gap = 8.0;
    let key = 78f32.min((w - 24.0 - 12.0 * gap) / 12.6);
    let key_h = if h < 640.0 { 44.0 } else { 52.0 };
    let area_h = 5.0 * (key_h + gap) + 14.0;
    let area = Rect::new(0.0, h - area_h, w, area_h);
    let row_y = |r: usize| area.y + 14.0 + r as f32 * (key_h + gap);
    let row_x = |n: f32| (w - n * key - (n - 1.0) * gap) / 2.0;
    let rows = ["1234567890-", "qwertyuiop_", "asdfghjkl", "zxcvbnm"];
    for (r, letters) in rows.iter().enumerate() {
        let extra = match r {
            0 => 1.6,
            3 => 3.0,
            _ => 0.0,
        };
        let n = letters.len() as f32;
        let mut x = row_x(n + extra) + if r == 3 { 1.5 * key + gap } else { 0.0 };
        if r == 2 {
            x += key * 0.3;
        }
        for ch in letters.chars() {
            keys.push((KeyCap::Char(ch), Rect::new(x, row_y(r), key, key_h)));
            x += key + gap;
        }
        if r == 0 {
            keys.push((KeyCap::Erase, Rect::new(x, row_y(0), key * 1.6, key_h)));
        }
        if r == 3 {
            keys.push((KeyCap::Shift, Rect::new(row_x(n + extra), row_y(3), key * 1.5, key_h)));
            keys.push((KeyCap::Shift, Rect::new(x, row_y(3), key * 1.5, key_h)));
        }
    }
    let space = Rect::new(row_x(8.0), row_y(4), 6.0 * key + 5.0 * gap, key_h);
    keys.push((KeyCap::Space, space));
    keys.push((KeyCap::Hide, Rect::new(space.x + space.w + gap, row_y(4), 2.0 * key + gap, key_h)));
    (area, keys)
}
