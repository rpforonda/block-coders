//! The guided tour: the first time a player opens level 1, Bit walks them
//! round the screen one thing at a time. Each step dims everything except one
//! spot, and only that spot (or the tour's own buttons) reacts to a click, so
//! a child who has never used a computer can't get lost.
//!
//! Nothing about the tour is saved. It starts whenever level 1 opens with an
//! empty program, and level 1's Help popup can start it again.

use crate::art::{draw_tex, Art};
use crate::ui::*;
use macroquad::prelude::*;

/// A part of the play screen a step points at. `play.rs` knows where each one is.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Spot {
    /// Nothing: the whole screen is dimmed.
    None,
    World,
    /// The first block in the palette (FORWARD on level 1).
    FirstBlock,
    Program,
    StepReset,
    HintGoal,
    Help,
    Run,
}

/// What finishes a step.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Wait {
    /// The player presses the tour's NEXT button.
    Next,
    /// MAIN has at least this many blocks.
    Blocks(usize),
    /// The program starts running.
    Run,
}

pub struct Step {
    pub spot: Spot,
    pub text: &'static str,
    /// Used instead of `text` on touch screens, where needed. Other steps just
    /// say "tap" for "click".
    pub touch_text: Option<&'static str>,
    pub wait: Wait,
}

pub const STEPS: &[Step] = &[
    Step {
        spot: Spot::None,
        text: "Hi! I'm Bit, your robot helper. I'll show you how to play.\n\n\
               The MOUSE moves a little pointer on the screen. To CLICK something, \
               put the pointer on it and press the LEFT mouse button once.\n\n\
               Try it now: click the green NEXT button.",
        touch_text: Some(
            "Hi! I'm Bit, your robot helper. I'll show you how to play.\n\n\
             To TAP something, touch it once with one finger.\n\n\
             Try it now: tap the green NEXT button.",
        ),
        wait: Wait::Next,
    },
    Step {
        spot: Spot::World,
        text: "This is the WORLD. That's your miner! The yellow arrow shows which way the miner is looking.\n\n\
               The purple square is the PORTAL. Your job is to get the miner into the portal. Click NEXT.",
        touch_text: None,
        wait: Wait::Next,
    },
    Step {
        spot: Spot::FirstBlock,
        text: "These are BLOCKS. Each block tells the miner to do one thing.\n\nClick the blue FORWARD block.",
        touch_text: None,
        wait: Wait::Blocks(1),
    },
    Step {
        spot: Spot::Program,
        text: "Your block is now in your PROGRAM. A program is a list of steps. \
               The miner does them in order, from top to bottom.\n\n\
               Oops, a wrong block? Click the x on it to take it away. Click NEXT.",
        touch_text: None,
        wait: Wait::Next,
    },
    Step {
        spot: Spot::FirstBlock,
        text: "FORWARD moves the miner one square. The portal is 4 squares away.\n\n\
               Click FORWARD again until your program has 4 blocks.",
        touch_text: None,
        wait: Wait::Blocks(4),
    },
    Step {
        spot: Spot::StepReset,
        text: "STEP runs your program one block at a time, so you can watch closely.\n\n\
               RESET puts the miner back at the start. Click NEXT.",
        touch_text: None,
        wait: Wait::Next,
    },
    Step {
        spot: Spot::HintGoal,
        text: "Stuck? Click HINT and I'll help you.\n\nGOAL tells you what to do in each level. Click NEXT.",
        touch_text: None,
        wait: Wait::Next,
    },
    Step {
        spot: Spot::Help,
        text: "HELP shows the instructions for a level again. On this level, it can show you this tour again too.\n\nClick NEXT.",
        touch_text: None,
        wait: Wait::Next,
    },
    Step {
        spot: Spot::Run,
        text: "Ready? Click RUN and watch the miner follow your program!",
        touch_text: None,
        wait: Wait::Run,
    },
];

/// What the tour needs to know about the play screen each frame.
pub struct Facts {
    pub main_blocks: usize,
    pub running: bool,
}

pub struct Tour {
    pub step: usize,
    pub touch: bool,
}

/// What happened to the tour this frame.
#[derive(PartialEq, Eq, Debug)]
pub enum Out {
    Stay,
    Done,
}

impl Tour {
    pub fn new(touch: bool) -> Tour {
        Tour { step: 0, touch }
    }

    pub fn current(&self) -> &'static Step {
        &STEPS[self.step.min(STEPS.len() - 1)]
    }

    /// The step's words, with "tap" for "click" on touch screens.
    pub fn text(&self) -> String {
        let s = self.current();
        match (self.touch, s.touch_text) {
            (true, Some(t)) => t.to_string(),
            (true, None) => s.text.replace("click", "tap").replace("Click", "Tap").replace("CLICK", "TAP"),
            (false, _) => s.text.to_string(),
        }
    }

    /// Moves on when the player has done what the step asks.
    pub fn update(&mut self, f: &Facts) -> Out {
        let done = match self.current().wait {
            Wait::Next => false,
            Wait::Blocks(n) => f.main_blocks >= n,
            Wait::Run => f.running,
        };
        if done { self.next() } else { Out::Stay }
    }

    pub fn next(&mut self) -> Out {
        self.step += 1;
        if self.step >= STEPS.len() { Out::Done } else { Out::Stay }
    }
}

const BUBBLE_W: f32 = 510.0;
/// The tallest a bubble gets; its text must fit in this.
pub const BUBBLE_MAX_H: f32 = 330.0;
const TEXT_SIZE: f32 = 19.0;
/// How wide the text is: the bubble minus Bit on the left.
const TEXT_W: f32 = BUBBLE_W - 112.0;

/// Where Bit's speech bubble of height `h` goes: beside the spot, never on top of it.
pub fn bubble(spot: Option<Rect>, h: f32) -> Rect {
    let Some(s) = spot else {
        return Rect::new((VW - BUBBLE_W) / 2.0, (VH - h) / 2.0, BUBBLE_W, h);
    };
    let x = if s.x + s.w / 2.0 > VW / 2.0 { s.x - 20.0 - BUBBLE_W } else { s.x + s.w + 20.0 };
    let x = x.clamp(10.0, VW - BUBBLE_W - 10.0);
    let y = (s.y + s.h / 2.0 - h / 2.0).clamp(66.0, VH - h - 10.0);
    Rect::new(x, y, BUBBLE_W, h)
}

/// The height of a bubble holding `lines` lines of text.
fn bubble_h(lines: usize) -> f32 {
    (lines as f32 * TEXT_SIZE * 1.3 + 90.0).clamp(180.0, BUBBLE_MAX_H)
}

/// Whether a click at `mouse` may reach the screen under the tour.
pub fn allows(step: &Step, spot: Option<Rect>, mouse: Vec2) -> bool {
    step.wait != Wait::Next && spot.is_some_and(|r| r.contains(mouse))
}

/// Draws the dimming, the highlight, a pointing hand and Bit's bubble.
/// Returns Done if the player finished or skipped the tour.
pub fn draw(tour: &mut Tour, spot: Option<Rect>, ui: &mut Ui, art: &Art) -> Out {
    let dim = Color::new(0.0, 0.0, 0.0, 0.62);
    match spot {
        None => draw_rectangle(0.0, 0.0, VW, VH, dim),
        Some(r) => {
            let pad = 6.0;
            let (x0, y0, x1, y1) = (r.x - pad, r.y - pad, r.x + r.w + pad, r.y + r.h + pad);
            draw_rectangle(0.0, 0.0, VW, y0, dim);
            draw_rectangle(0.0, y1, VW, VH - y1, dim);
            draw_rectangle(0.0, y0, x0, y1 - y0, dim);
            draw_rectangle(x1, y0, VW - x1, y1 - y0, dim);
            let pulse = ((get_time() * 4.0).sin() * 0.5 + 0.5) as f32;
            draw_rectangle_lines(x0, y0, x1 - x0, y1 - y0, 4.0 + pulse * 2.0, rgb8(255, 215, 40));
            if tour.current().wait != Wait::Next {
                pointer(vec2(r.x + r.w * 0.5, r.y + r.h * 0.6), pulse);
            }
        }
    }

    let words = tour.text();
    let b = bubble(spot, bubble_h(wrap(&words, TEXT_W, TEXT_SIZE).len()));
    panel(b);
    draw_tex(&art.bit, b.x + 16.0, b.y + 16.0, 64.0, 64.0, WHITE);
    text(&format!("{} of {}", tour.step + 1, STEPS.len()), b.x + 22.0, b.y + 88.0, 15.0, rgb8(90, 90, 100));
    paragraph(&words, b.x + 96.0, b.y + 18.0, TEXT_W, TEXT_SIZE, rgb8(30, 30, 30));
    let mut out = Out::Stay;
    if tour.current().wait == Wait::Next
        && ui.button(Rect::new(b.x + b.w - 146.0, b.y + b.h - 54.0, 130.0, 40.0), "NEXT >", rgb8(60, 170, 60), 22.0)
    {
        out = tour.next();
    }
    if ui.button(Rect::new(b.x + 16.0, b.y + b.h - 48.0, 120.0, 32.0), "Skip tour", rgb8(120, 120, 130), 16.0) {
        out = Out::Done;
    }
    out
}

/// A pointing-hand cursor that taps at `at`, so the child sees what to do.
fn pointer(at: Vec2, pulse: f32) {
    let tip = at + vec2(0.0, -6.0 * pulse);
    // An arrow head and its tail, as triangles.
    let tris = [
        [vec2(0.0, 0.0), vec2(0.0, 34.0), vec2(25.0, 25.0)],
        [vec2(8.0, 26.0), vec2(15.0, 23.0), vec2(23.0, 39.0)],
        [vec2(8.0, 26.0), vec2(23.0, 39.0), vec2(16.0, 42.0)],
    ];
    for (c, grow) in [(BLACK, 1.3), (WHITE, 1.0)] {
        let center = vec2(10.0, 22.0);
        for t in &tris {
            let q = |v: Vec2| tip + center + (v - center) * grow;
            draw_triangle(q(t[0]), q(t[1]), q(t[2]), c);
        }
    }
}
