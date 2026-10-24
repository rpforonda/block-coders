//! World Checks: a 10-question read-the-code quiz at the end of each world.
//! Passing (8 or more right) opens the next world. The questions live in
//! `questions.rs`.

use crate::art::{draw_tex, shade, Art};
use crate::editor::kind_color;
use crate::items::Look;
use crate::layout::{self, Answer, Canvas, CheckLayout};
use crate::levels::{Level, BASE, CHAPTERS, LEVELS};
use crate::program::{Kind, Node, Program};
#[cfg(test)]
use crate::program::{Action, Step, Vm};
use crate::questions::{Choices, Question, BANKS};
#[cfg(test)]
use crate::questions::End;
use crate::save::Progress;
use crate::sound::{Sfx, Sounds};
use crate::ui::*;
use crate::world::{Dir, Floor, Rng, Tile, World};
#[cfg(test)]
use crate::world::{Failure, Outcome};
use macroquad::prelude::*;

/// Questions per attempt, and how many must be right to pass (75%, rounded up).
pub const ASKED: usize = 10;
pub const PASS: u32 = 8;
/// How many World Checks there are (one after every world but the last).
pub const CHECKS: usize = BANKS.len();

/// "Which program works every time?" questions make kids picture every
/// version of a changing map at once. They're the hardest kind, so an
/// attempt asks at most this many.
pub const MAX_CHANGING: usize = 2;

/// Whether the question's map changes between versions.
pub fn changes(q: &Question) -> bool {
    q.map.iter().any(|row| row.chars().any(|c| random_meaning(c).is_some()))
}

/// Picks the questions for an attempt. Every question that wasn't in the
/// previous attempt comes first, so each retry is mostly new questions.
pub fn pick(bank: &'static [Question], last: &[String], rng: &mut Rng) -> Vec<&'static Question> {
    let shuffle = |v: &mut Vec<&'static Question>, rng: &mut Rng| {
        for i in (1..v.len()).rev() {
            v.swap(i, rng.below(i + 1));
        }
    };
    let (mut fresh, mut seen): (Vec<_>, Vec<_>) = bank.iter().partition(|q| !last.iter().any(|id| id == q.id));
    shuffle(&mut fresh, rng);
    shuffle(&mut seen, rng);
    let mut out = Vec::new();
    let mut changing = 0;
    for q in fresh.into_iter().chain(seen) {
        if out.len() == ASKED {
            break;
        }
        if changes(q) {
            if changing == MAX_CHANGING {
                continue;
            }
            changing += 1;
        }
        out.push(q);
    }
    shuffle(&mut out, rng);
    out
}

// ---------------------------------------------------------------------------
// Running a question's programs (used by the screen and by the tests).

/// A level holding just this question's map, so `World::from_level` can build it.
pub fn level_for(q: &'static Question) -> Level {
    Level { name: q.id, maps: std::slice::from_ref(&q.map), start_blocks: q.blocks, ..BASE }
}

/// The question's world: its map (one version of it, picked by `seed`), or a
/// big empty field with the miner in the middle facing up.
pub fn world_for(q: &'static Question, seed: u64) -> World {
    if !q.map.is_empty() {
        return World::from_level(&level_for(q), seed);
    }
    let n = 61;
    World {
        w: n,
        h: n,
        tiles: vec![Tile::Floor; (n * n) as usize],
        pos: (n / 2, n / 2),
        dir: Dir::Up,
        blocks: q.blocks,
        diamonds: 0,
        diamonds_total: 0,
        emeralds: 0,
        emeralds_total: 0,
        crystals: 0,
        crystals_total: 0,
        has_portal: false,
    }
}

#[cfg(test)]
pub struct Run {
    pub world: World,
    pub end: End,
    pub actions: Vec<Action>,
    /// What the `count` variable held at the end.
    pub count: u32,
}

/// Runs a program to the end, like pressing RUN without the animation.
#[cfg(test)]
pub fn run(p: &Program, mut world: World) -> Run {
    let mut vm = Vm::new(p);
    let mut actions = Vec::new();
    let end = loop {
        match vm.next(&world) {
            Step::Act(a, _) => {
                actions.push(a);
                // A map with no portal and nothing to collect has no goal, so
                // the game would call it won straight away; just keep going.
                let goal = world.has_portal || world.diamonds_total + world.emeralds_total > 0;
                match world.apply(a) {
                    Outcome::Ok => {}
                    Outcome::Won if !goal => {}
                    Outcome::Won => break End::Wins,
                    Outcome::Fail(f) => {
                        break match f {
                            Failure::Bonk(_) => End::Bonk,
                            Failure::Hazard(_) => End::Hazard,
                            Failure::NothingToMine => End::NothingToMine,
                            Failure::CantMine(_) => End::CantMine,
                            Failure::NoBlocks => End::NoBlocks,
                            Failure::CantPlace(_) => End::CantPlace,
                        };
                    }
                }
            }
            Step::Var(_) => {}
            Step::Done => break End::Unfinished,
            Step::Error(..) => break End::Stuck,
        }
    };
    Run { world, end, actions, count: vm.count }
}

// ---------------------------------------------------------------------------
// The screen.

pub enum CheckOut {
    Stay,
    /// Back to the level select screen.
    Back,
    /// Passed: go straight to this world.
    World(usize),
}

#[derive(PartialEq)]
enum Phase {
    Intro,
    Ask,
    Done,
}

pub struct CheckScreen {
    pub chapter: usize,
    questions: Vec<&'static Question>,
    /// Per question, the order its choices are shown in (indexes into the
    /// question's choices; 0 is the right one).
    orders: Vec<Vec<usize>>,
    /// Per question answered so far: was it right?
    results: Vec<bool>,
    picked: Option<usize>,
    phase: Phase,
    look: Look,
    passed_before: bool,
    rng: Rng,
    /// On phones, which program (by its place in the shown order) is on screen.
    shown: usize,
}

fn fresh_seed() -> u64 {
    (get_time() * 1_000_000.0) as u64 ^ ((macroquad::rand::rand() as u64) << 20) ^ macroquad::miniquad::date::now().to_bits()
}

fn choice_count(q: &Question) -> usize {
    match q.choices {
        Choices::Text(c) => c.len(),
        Choices::Programs(c) => c.len(),
        Choices::Spots(c) => c.len(),
    }
}

/// The ground in a world's levels, so the check looks like that world.
fn floor_of(chapter: usize) -> Floor {
    LEVELS.iter().find(|l| l.chapter == chapter).map_or(Floor::Grass, |l| l.floor)
}

const LETTERS: [&str; 4] = ["A", "B", "C", "D"];

impl CheckScreen {
    pub fn new(chapter: usize, progress: &mut Progress, look: Look) -> CheckScreen {
        let mut s = CheckScreen {
            chapter,
            questions: Vec::new(),
            orders: Vec::new(),
            results: Vec::new(),
            picked: None,
            phase: Phase::Intro,
            look,
            passed_before: progress.checks[chapter],
            rng: Rng::new(fresh_seed()),
            shown: 0,
        };
        s.start_attempt(progress);
        s.phase = Phase::Intro;
        s
    }

    /// Screenshot autopilot (debug builds): puts question `id` first, its
    /// right answer shown second so the picker has to move to it.
    pub fn debug_question(&mut self, id: &str) {
        if let Some(q) = BANKS[self.chapter].iter().find(|q| q.id == id) {
            self.questions[0] = q;
            let n = choice_count(q);
            self.orders[0] = (0..n).map(|i| (i + n - 1) % n).collect();
        }
    }

    /// Draws a new set of questions. They count as seen straight away, so
    /// quitting halfway and coming back still gives a fresh mix.
    fn start_attempt(&mut self, progress: &mut Progress) {
        self.questions = pick(BANKS[self.chapter], &progress.check_last[self.chapter], &mut self.rng);
        self.orders = self
            .questions
            .iter()
            .map(|q| {
                let mut o: Vec<usize> = (0..choice_count(q)).collect();
                for i in (1..o.len()).rev() {
                    o.swap(i, self.rng.below(i + 1));
                }
                o
            })
            .collect();
        self.results.clear();
        self.picked = None;
        self.shown = 0;
        self.phase = Phase::Ask;
        progress.check_last[self.chapter] = self.questions.iter().map(|q| q.id.to_string()).collect();
        progress.save();
    }

    fn score(&self) -> u32 {
        self.results.iter().filter(|&&r| r).count() as u32
    }

    pub fn frame(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, progress: &mut Progress, c: Canvas) -> CheckOut {
        let ch = &CHAPTERS[self.chapter];
        let color = rgb8(ch.color.0, ch.color.1, ch.color.2);
        // The header is the same for every kind of question.
        let head = layout::check(c, Answer::Text, 3, false, false);
        draw_rectangle(0.0, 0.0, c.0, c.1, rgb8(28, 28, 34));
        draw_rectangle(0.0, 0.0, c.0, head.header_h, shade(color, 0.55));
        let mut out = CheckOut::Stay;
        if ui.button(head.back, "< Levels", rgb8(110, 110, 120), 20.0) {
            sfx.play(Sfx::Click);
            out = CheckOut::Back;
        }
        let title = format!("World {} Check: {}", self.chapter + 1, ch.concept);
        let room = if layout::is_tall(c) { c.0 - head.title.x - 12.0 } else { head.pips.0 - head.title.x - 16.0 };
        shadow_text(&title, head.title.x, head.title.y, fit_size(&title, room, head.title.size, 18.0), WHITE);
        self.draw_pips(&head);

        match self.phase {
            Phase::Intro => self.intro(ui, art, sfx, c),
            Phase::Ask => self.ask(ui, art, sfx, progress, c),
            Phase::Done => {
                if let Some(o) = self.done(ui, art, sfx, progress, c) {
                    out = o;
                }
            }
        }
        out
    }

    /// One square per question: green right, red wrong, outlined = current.
    fn draw_pips(&self, l: &CheckLayout) {
        let (x0, y, s) = l.pips;
        for i in 0..ASKED {
            let r = Rect::new(x0 + i as f32 * (s + 6.0), y, s, s);
            let fill = match self.results.get(i) {
                Some(true) => rgb8(70, 190, 80),
                Some(false) => rgb8(200, 60, 50),
                None => rgb8(60, 60, 70),
            };
            draw_rectangle(r.x, r.y, r.w, r.h, fill);
            let current = self.phase == Phase::Ask && i == self.results.len();
            draw_rectangle_lines(r.x, r.y, r.w, r.h, if current { 3.0 } else { 1.5 }, if current { WHITE } else { BLACK });
        }
    }

    fn intro(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, c: Canvas) {
        let narrow = c.0 - 24.0 < 700.0;
        let r = layout::popup(c, 760.0, if narrow { 560.0 } else { 470.0 }, 150.0);
        panel(r);
        draw_tex(&art.bit, r.x + 24.0, r.y + 24.0, 80.0, 80.0, WHITE);
        let heading = format!("World {} Check", self.chapter + 1);
        text(&heading, r.x + 124.0, r.y + 44.0, fit_size(&heading, r.w - 144.0, 34.0, 22.0), TITLE_ON_LIGHT);
        let concept = CHAPTERS[self.chapter].concept.to_lowercase();
        let unlock = if self.chapter + 1 == CHAPTERS.len() - 1 {
            format!(
                "Pass it to finish {}. The {} opens when ALL {CHECKS} World Checks are passed.",
                CHAPTERS[self.chapter].name,
                CHAPTERS[CHAPTERS.len() - 1].name
            )
        } else {
            format!("Pass it to unlock World {}: {}!", self.chapter + 2, CHAPTERS[self.chapter + 1].name)
        };
        let mut body = format!(
            "Time to show what you know about {concept}! I'll show you {ASKED} programs. Read each one carefully and work out what it does.\n\nGet {PASS} or more right to pass. {unlock}\n\nDidn't pass? No problem: try again as many times as you like, with new questions each time."
        );
        if self.passed_before {
            body = format!("You've already passed this check, so this is just for practice.\n\n{body}");
        }
        let box_r = Rect::new(r.x + 20.0, r.y + 118.0, r.w - 40.0, r.h - 200.0);
        draw_rectangle(box_r.x, box_r.y, box_r.w, box_r.h, rgb8(60, 60, 72));
        // Long text in a small box steps down a little, never below 16px.
        let size = [21.0, 19.0, 17.5, 16.0]
            .into_iter()
            .find(|&s| wrap(&body, box_r.w - 32.0, s).len() as f32 * s * 1.3 <= box_r.h - 20.0)
            .unwrap_or(16.0);
        paragraph(&body, box_r.x + 16.0, box_r.y + 16.0, box_r.w - 32.0, size, WHITE);
        if ui.button(Rect::new(r.x + r.w / 2.0 - 110.0, r.y + r.h - 66.0, 220.0, 48.0), "Start!", rgb8(60, 170, 60), 26.0)
            || is_key_pressed(KeyCode::Enter)
        {
            sfx.play(Sfx::Click);
            self.phase = Phase::Ask;
        }
    }

    fn ask(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, progress: &mut Progress, c: Canvas) {
        let n = self.results.len();
        let q = self.questions[n];
        let order = self.orders[n].clone();
        let answered = self.picked.is_some();
        let floor = floor_of(self.chapter);
        let program = (!q.program.is_empty()).then(|| Program::deserialize(q.program)).flatten();
        let has_map = !q.map.is_empty();
        let answer = match q.choices {
            Choices::Text(_) => Answer::Text,
            Choices::Spots(_) => Answer::Spots,
            Choices::Programs(_) => Answer::Programs,
        };
        let l = layout::check(c, answer, order.len(), program.is_some(), has_map);

        // The question.
        draw_tex(&art.bit, l.bit.x, l.bit.y, l.bit.w, l.bit.h, WHITE);
        text(&format!("Question {} of {ASKED}", n + 1), l.counter.x, l.counter.y, l.counter.size, rgb8(170, 170, 185));
        let size = [l.prompt_size, l.prompt_size - 3.0]
            .into_iter()
            .find(|&s| wrap(q.prompt, l.prompt.w, s).len() as f32 * s * 1.3 <= l.prompt.h + 4.0)
            .unwrap_or(l.prompt_size - 3.0);
        paragraph(q.prompt, l.prompt.x, l.prompt.y, l.prompt.w, size, WHITE);

        let spots = match q.choices {
            Choices::Spots(s) => Some(s),
            _ => None,
        };
        let spot_marks: Vec<(i32, i32, &str, Option<bool>)> = spots
            .map(|s| {
                order
                    .iter()
                    .enumerate()
                    .map(|(shown, &c)| {
                        let state = answered.then_some(c == 0).filter(|&right| right || self.picked == Some(c));
                        (s[c].0, s[c].1, LETTERS[shown], state)
                    })
                    .collect()
            })
            .unwrap_or_default();

        // The program and map to read.
        let program_choices = answer == Answer::Programs;
        if let (Some(p), Some(r)) = (&program, l.program) {
            draw_program_panel(p, r, art, if program_choices { "This program:" } else { "" });
        }
        if let Some(r) = l.map {
            draw_map(q, r, art, floor, &self.look, &spot_marks);
        }

        // The choices.
        let mut chosen = None;
        let ok = rgb8(70, 190, 80);
        let bad = rgb8(210, 60, 50);
        let mark = |r: Rect, c: usize, picked: Option<usize>| {
            if answered && (c == 0 || picked == Some(c)) {
                let col = if c == 0 { ok } else { bad };
                draw_rectangle_lines(r.x - 5.0, r.y - 5.0, r.w + 10.0, r.h + 10.0, 5.0, col);
            }
        };
        match q.choices {
            Choices::Text(texts) => {
                for (shown, &c) in order.iter().enumerate() {
                    let r = l.choices[shown];
                    let label = format!("{}   {}", LETTERS[shown], texts[c]);
                    let size = fit_size(&label, r.w - 20.0, 22.0, 16.0);
                    if answered {
                        bevel(r, rgb8(80, 80, 95));
                        text_centered(&label, r, size, WHITE, true);
                    } else if ui.button(r, &label, rgb8(70, 110, 170), size) {
                        chosen = Some(c);
                    }
                    mark(r, c, self.picked);
                }
            }
            Choices::Spots(_) => {
                for (shown, &c) in order.iter().enumerate() {
                    let r = l.choices[shown];
                    let label = format!("Square {}", LETTERS[shown]);
                    let size = fit_size(&label, r.w - 16.0, 26.0, 18.0);
                    if answered {
                        bevel(r, rgb8(80, 80, 95));
                        text_centered(&label, r, size, WHITE, true);
                    } else if ui.button(r, &label, rgb8(70, 110, 170), size) {
                        chosen = Some(c);
                    }
                    mark(r, c, self.picked);
                }
            }
            Choices::Programs(progs) => match l.carousel {
                // Phones: one program at a time, with arrows and a button to pick it.
                Some(car) => {
                    let shown = self.shown.min(order.len() - 1);
                    let c = order[shown];
                    let r = l.choices[0];
                    bevel(r, rgb8(60, 70, 95));
                    shadow_text(&format!("Program {}", LETTERS[shown]), r.x + 12.0, r.y + 8.0, 22.0, WHITE);
                    if let Some(p) = Program::deserialize(progs[c]) {
                        draw_program(&p, Rect::new(r.x + 8.0, r.y + 38.0, r.w - 16.0, r.h - 46.0), art);
                    }
                    mark(r, c, self.picked);
                    let count = order.len();
                    if ui.button(car.prev, "<", rgb8(70, 110, 170), 28.0) {
                        sfx.play(Sfx::Click);
                        self.shown = (shown + count - 1) % count;
                    }
                    if ui.button(car.next, ">", rgb8(70, 110, 170), 28.0) {
                        sfx.play(Sfx::Click);
                        self.shown = (shown + 1) % count;
                    }
                    // Wording picked with TypeSafe for clarity and kindness (see docs/DESIGN.md).
                    let counter = format!("{}  -  tap the arrows to see all {count}", LETTERS[shown]);
                    text_centered(&counter, car.label, fit_size(&counter, car.label.w - 8.0, 20.0, 15.0), WHITE, true);
                    if !answered {
                        if ui.button(car.pick, &format!("This one! (Program {})", LETTERS[shown]), rgb8(60, 170, 60), 24.0) {
                            chosen = Some(c);
                        }
                    } else {
                        let (words, col) = if c == 0 {
                            ("This one works!", ok)
                        } else if self.picked == Some(c) {
                            ("Your pick - nice try!", rgb8(255, 170, 150))
                        } else {
                            ("Not this one. Use the arrows to compare.", rgb8(170, 170, 185))
                        };
                        text_centered(words, car.pick, fit_size(words, car.pick.w - 16.0, 22.0, 16.0), col, true);
                    }
                }
                None => {
                    for (shown, &c) in order.iter().enumerate() {
                        let r = l.choices[shown];
                        let hover = !answered && ui.hover(r);
                        bevel(r, if hover { rgb8(85, 105, 150) } else { rgb8(60, 70, 95) });
                        shadow_text(&format!("Program {}", LETTERS[shown]), r.x + 12.0, r.y + 8.0, 22.0, WHITE);
                        if let Some(p) = Program::deserialize(progs[c]) {
                            draw_program(&p, Rect::new(r.x + 8.0, r.y + 38.0, r.w - 16.0, r.h - 46.0), art);
                        }
                        if !answered && ui.clicked(r) {
                            chosen = Some(c);
                        }
                        mark(r, c, self.picked);
                    }
                }
            },
        }
        // Keys 1-4 (or A-D) pick an answer too.
        if !answered {
            for (shown, keys) in [[KeyCode::Key1, KeyCode::A], [KeyCode::Key2, KeyCode::B], [KeyCode::Key3, KeyCode::C], [KeyCode::Key4, KeyCode::D]]
                .iter()
                .enumerate()
            {
                if shown < order.len() && keys.iter().any(|&k| is_key_pressed(k)) {
                    chosen = Some(order[shown]);
                }
            }
        }
        if let Some(c) = chosen {
            self.picked = Some(c);
            sfx.play(if c == 0 { Sfx::Collect } else { Sfx::Bonk });
            // On phones, show the right program once answered.
            self.shown = order.iter().position(|&o| o == 0).unwrap_or(0);
        }

        // Bit explains, then on to the next question.
        if let Some(p) = self.picked {
            let right = p == 0;
            let r = l.explain;
            draw_rectangle(r.x, r.y, r.w, r.h, if right { rgb8(35, 80, 45) } else { rgb8(95, 40, 40) });
            draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, if right { ok } else { bad });
            let head = if right { "Right! " } else { "Not quite. " };
            let words = format!("{head}{}", q.explain);
            // The words go beside the Next button, or above it when it sits along the bottom.
            let beside = l.next.y < r.y + 30.0;
            let area = if beside { Rect::new(r.x + 14.0, r.y + 8.0, r.w - 230.0, r.h - 12.0) } else { Rect::new(r.x + 14.0, r.y + 8.0, r.w - 28.0, l.next.y - r.y - 12.0) };
            let size = [18.0, 16.5, 15.0]
                .into_iter()
                .find(|&s| wrap(&words, area.w, s).len() as f32 * s * 1.3 <= area.h + 2.0)
                .unwrap_or(15.0);
            paragraph(&words, area.x, area.y, area.w, size, WHITE);
            let last = n + 1 == ASKED;
            if ui.button(l.next, if last { "Finish" } else { "Next >" }, rgb8(60, 170, 60), 24.0) || is_key_pressed(KeyCode::Enter) {
                sfx.play(Sfx::Click);
                self.results.push(right);
                self.picked = None;
                self.shown = 0;
                if last {
                    self.phase = Phase::Done;
                    if self.score() >= PASS {
                        progress.checks[self.chapter] = true;
                        progress.save();
                        sfx.play(Sfx::Win);
                    } else {
                        sfx.play(Sfx::Fail);
                    }
                }
            }
        } else {
            let tip = if crate::keyboard::is_touch() { "Tap an answer!" } else { "Pick an answer!" };
            text_centered(tip, Rect::new(l.explain.x, l.explain.y + 12.0, l.explain.w, 30.0), 20.0, rgb8(170, 170, 185), false);
        }
    }

    fn done(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, progress: &mut Progress, c: Canvas) -> Option<CheckOut> {
        let score = self.score();
        let passed = score >= PASS;
        // Narrow popups stack their buttons, so they're taller.
        let narrow = c.0 - 24.0 < 620.0;
        let r = layout::popup(c, 720.0, if narrow { 600.0 } else { 520.0 }, 130.0);
        panel(r);
        let title = if passed { "You passed!" } else { "Almost there!" };
        text_centered(title, Rect::new(r.x, r.y + 20.0, r.w, 48.0), fit_size(title, r.w - 40.0, 44.0, 28.0), if passed { TITLE_ON_LIGHT } else { WARN_ON_LIGHT }, false);
        text_centered(&format!("{score} / {ASKED}"), Rect::new(r.x, r.y + 80.0, r.w, 70.0), 64.0, DARK_TEXT, false);
        let next = self.chapter + 1;
        let next_open = progress.world_open(next);
        let body = if passed {
            if next_open {
                format!(
                    "You really understand {}! World {}: {} is open.",
                    CHAPTERS[self.chapter].concept.to_lowercase(),
                    next + 1,
                    CHAPTERS[next].name
                )
            } else {
                let missing: Vec<String> = (0..CHECKS).filter(|&c| !progress.checks[c]).map(|c| format!("{}", c + 1)).collect();
                format!(
                    "Great work! The {} opens once every World Check is passed. Still to go: World Check {}.",
                    CHAPTERS[next].name,
                    missing.join(", ")
                )
            }
        } else {
            format!(
                "You need {PASS} right to pass. Read Bit's explanations, maybe replay a few levels, then try again. You'll get some new questions."
            )
        };
        draw_tex(&art.bit, r.x + 30.0, r.y + 180.0, 72.0, 72.0, WHITE);
        paragraph(&body, r.x + 120.0, r.y + 186.0, r.w - 150.0, 22.0, rgb8(30, 30, 30));
        let by = r.y + r.h - 70.0;
        let full = |y: f32| Rect::new(r.x + 30.0, y, r.w - 60.0, 50.0);
        let mut out = None;
        if passed && next_open {
            let b = if narrow { full(by - 62.0) } else { Rect::new(r.x + r.w - 300.0, by, 270.0, 50.0) };
            if ui.button(b, &format!("Go to World {} >", next + 1), rgb8(60, 170, 60), 24.0) {
                sfx.play(Sfx::Click);
                out = Some(CheckOut::World(next));
            }
        } else if !passed {
            let b = if narrow { full(by - 62.0) } else { Rect::new(r.x + r.w - 250.0, by, 220.0, 50.0) };
            if ui.button(b, "Try again", rgb8(70, 130, 180), 24.0) {
                sfx.play(Sfx::Click);
                self.start_attempt(progress);
            }
        }
        let levels = if narrow { full(by) } else { Rect::new(r.x + 30.0, by, 180.0, 50.0) };
        if ui.button(levels, "Levels", rgb8(110, 110, 120), 22.0) {
            sfx.play(Sfx::Click);
            out = Some(CheckOut::Back);
        }
        out
    }
}

// ---------------------------------------------------------------------------
// Drawing programs and maps (read-only versions of the editor and world).

enum Line<'a> {
    Header(&'static str),
    Block(&'a Node, usize),
    Else(Kind, usize),
    /// The bottom of a container; holds the y of its first row.
    End(Kind, usize, usize),
}

fn lines<'a>(list: &'a [Node], depth: usize, out: &mut Vec<Line<'a>>) {
    for n in list {
        let start = out.len();
        out.push(Line::Block(n, depth));
        if n.kind.is_container() {
            lines(&n.body, depth + 1, out);
            if n.kind == Kind::IfElse {
                out.push(Line::Else(n.kind, depth));
                lines(&n.els, depth + 1, out);
            }
            out.push(Line::End(n.kind, depth, start));
        }
    }
}

fn line_weight(l: &Line) -> f32 {
    match l {
        Line::Header(_) | Line::Block(..) => 1.0,
        Line::Else(..) => 0.7,
        Line::End(..) => 0.35,
    }
}

/// A program as blocks, scaled to fit `area`. Shows MAIN and FUNCTION
/// separately when there is a function.
pub fn draw_program(p: &Program, area: Rect, art: &Art) {
    let mut ls = Vec::new();
    let mut used = Vec::new();
    crate::program::kinds_used(&p.main, &mut used);
    let has_func = !p.func.is_empty() || used.contains(&Kind::Call);
    if has_func {
        ls.push(Line::Header("MAIN"));
    }
    lines(&p.main, 0, &mut ls);
    if has_func {
        ls.push(Line::Header("FUNCTION"));
        lines(&p.func, 0, &mut ls);
    }
    let total: f32 = ls.iter().map(line_weight).sum();
    let row = (area.h / total.max(1.0)).min(40.0);
    let indent = (row * 0.55).max(10.0);
    let size = (row * 0.5).clamp(11.0, 20.0);
    let mut ys = Vec::with_capacity(ls.len());
    let mut y = area.y;
    for l in &ls {
        ys.push(y);
        y += row * line_weight(l);
    }
    let bx = |d: usize| area.x + d as f32 * indent;
    let bw = |d: usize| (area.w - d as f32 * indent).min(420.0 - d as f32 * indent);
    // Container spines first, so blocks draw on top.
    for (i, l) in ls.iter().enumerate() {
        if let Line::End(kind, d, start) = *l {
            let c = shade(kind_color(kind), 0.85);
            let y0 = ys[start] + row - 4.0;
            let y1 = ys[i] + row * 0.35;
            draw_rectangle(bx(d), y0, indent - 3.0, y1 - y0, c);
            draw_rectangle(bx(d), ys[i], bw(d) * 0.5, row * 0.35 - 2.0, c);
        }
    }
    for (i, l) in ls.iter().enumerate() {
        let y = ys[i];
        match *l {
            Line::Header(h) => {
                text(h, area.x, y + row * 0.25, size * 0.9, rgb8(190, 190, 205));
            }
            Line::Else(kind, d) => {
                draw_rectangle(bx(d), y, bw(d) * 0.5, row * 0.7 - 2.0, shade(kind_color(kind), 0.85));
                shadow_text("else", bx(d) + indent + 4.0, y + (row * 0.7 - size) / 2.0, size, WHITE);
            }
            Line::End(..) => {}
            Line::Block(n, d) => {
                let r = Rect::new(bx(d), y + 2.0, bw(d), row - 5.0);
                let color = kind_color(n.kind);
                bevel(r, color);
                let icon = (row - 12.0).max(10.0);
                let tc = text_on(color);
                let dark = tc != WHITE;
                crate::ui::block_icon(n.kind, r.x + 4.0, r.y + (r.h - icon) / 2.0, icon, tc, art);
                let label = n.label();
                let ty = r.y + (r.h - size) / 2.0;
                if dark {
                    text(&label, r.x + icon + 10.0, ty, size, tc);
                } else {
                    shadow_text(&label, r.x + icon + 10.0, ty, size, WHITE);
                }
            }
        }
    }
}

/// A program on a dark card, with an optional caption.
fn draw_program_panel(p: &Program, r: Rect, art: &Art, caption: &str) {
    dark_panel(r);
    let mut inner = Rect::new(r.x + 14.0, r.y + 12.0, r.w - 28.0, r.h - 24.0);
    if !caption.is_empty() {
        text(caption, inner.x, inner.y, 18.0, rgb8(190, 190, 205));
        inner.y += 28.0;
        inner.h -= 28.0;
    }
    draw_program(p, inner, art);
}

/// What a random map square might turn into, for the legend.
fn random_meaning(c: char) -> Option<&'static str> {
    Some(match c {
        '?' => "? = stone or empty",
        '!' => "! = lava or empty",
        '*' => "* = stone or lava",
        '$' => "$ = diamond or empty",
        '~' => "~ = water or empty",
        '&' => "& = stone, lava or empty",
        'p' => "p = maybe the portal",
        _ => return None,
    })
}

/// The question's map. Squares that change between versions show their
/// symbol instead of a tile. `marks` are lettered answer squares; once
/// answered they turn green (right) or red (picked, wrong).
fn draw_map(q: &'static Question, area: Rect, art: &Art, floor: Floor, look: &Look, marks: &[(i32, i32, &str, Option<bool>)]) {
    dark_panel(area);
    let world = world_for(q, 0);
    let mut legend: Vec<&str> = Vec::new();
    for row in q.map {
        for c in row.chars() {
            if let Some(m) = random_meaning(c).filter(|m| !legend.contains(m)) {
                legend.push(m);
            }
        }
    }
    let footer = if legend.is_empty() && q.blocks == 0 { 0.0 } else { 34.0 };
    let inner = Rect::new(area.x + 10.0, area.y + 10.0, area.w - 20.0, area.h - 20.0 - footer);
    let ts = (inner.w / world.w as f32).min(inner.h / world.h as f32).floor().min(64.0);
    let ox = inner.x + (inner.w - ts * world.w as f32) / 2.0;
    let oy = inner.y + (inner.h - ts * world.h as f32) / 2.0;
    let time = get_time();
    for (y, row) in q.map.iter().enumerate() {
        for (x, c) in row.chars().enumerate() {
            let (px, py) = (ox + x as f32 * ts, oy + y as f32 * ts);
            draw_tex(art.floor(floor), px, py, ts, ts, WHITE);
            if random_meaning(c).is_some() {
                draw_rectangle(px + 3.0, py + 3.0, ts - 6.0, ts - 6.0, Color::new(0.1, 0.08, 0.0, 0.55));
                draw_rectangle_lines(px + 3.0, py + 3.0, ts - 6.0, ts - 6.0, 3.0, rgb8(255, 215, 40));
                let s = c.to_string();
                let size = ts * 0.75;
                shadow_text(&s, px + (ts - text_width(&s, size)) / 2.0, py + (ts - size) / 2.0, size, rgb8(255, 220, 60));
            } else {
                let t = world.tile((x as i32, y as i32));
                if let Some(tex) = art.tile(t, floor, time, true) {
                    draw_tex(tex, px, py, ts, ts, WHITE);
                }
                if t == Tile::Emerald {
                    let s = ts * 0.62;
                    draw_tex(&art.emerald, px + (ts - s) / 2.0, py + (ts - s) / 2.0, s, s, WHITE);
                }
            }
            draw_rectangle_lines(px, py, ts, ts, 1.0, Color::new(0.0, 0.0, 0.0, 0.12));
        }
    }
    // The miner and the way it faces.
    let (cx, cy) = (ox + (world.pos.0 as f32 + 0.5) * ts, oy + (world.pos.1 as f32 + 0.5) * ts);
    let sh = ts * 0.88;
    art.miner(look, world.dir, Rect::new(cx - sh * 0.375, cy - sh / 2.0, sh * 0.75, sh), WHITE);
    let a = world.dir.angle().to_radians();
    let d = vec2(a.sin(), -a.cos());
    let perp = vec2(-d.y, d.x);
    let c = vec2(cx, cy);
    draw_triangle(c + d * ts * 0.56, c + d * ts * 0.36 + perp * ts * 0.14, c + d * ts * 0.36 - perp * ts * 0.14, rgb8(255, 225, 40));
    // Answer squares.
    for &(x, y, letter, state) in marks {
        // On the miner's own square, sit in the corner so the miner stays visible.
        let corner = if (x, y) == world.pos { 0.3 } else { 0.0 };
        let (mx, my) = (ox + (x as f32 + 0.5 + corner) * ts, oy + (y as f32 + 0.5 - corner) * ts);
        let fill = match state {
            Some(true) => rgb8(70, 190, 80),
            Some(false) => rgb8(210, 60, 50),
            None => rgb8(255, 215, 40),
        };
        let rad = ts * 0.3;
        draw_circle(mx, my, rad + 2.0, BLACK);
        draw_circle(mx, my, rad, fill);
        let s = rad * 1.3;
        text(letter, mx - text_width(letter, s) / 2.0, my - s / 2.0, s, if state.is_none() { BLACK } else { WHITE });
    }
    // Legend and backpack.
    let mut fx = area.x + 14.0;
    let fy = area.y + area.h - footer;
    if q.blocks > 0 {
        draw_tex(&art.cobble, fx, fy + 4.0, 22.0, 22.0, WHITE);
        let s = format!("Backpack: {} block{}", q.blocks, if q.blocks == 1 { "" } else { "s" });
        text(&s, fx + 30.0, fy + 6.0, 18.0, WHITE);
        fx += 40.0 + text_width(&s, 18.0);
    }
    if !legend.is_empty() {
        text(&legend.join("   "), fx, fy + 6.0, 18.0, rgb8(255, 230, 120));
    }
}
