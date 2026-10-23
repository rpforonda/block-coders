//! The gameplay screen: the world, the block palette, run controls and popups.

use crate::art::{draw_tex, Art};
use crate::check::CHECKS;
use crate::editor::{kind_color, Editor, Tab};
use crate::hints::{self, Hint};

/// How many changes Undo can go back.
const HISTORY: usize = 100;
use crate::levels::{Level, CHAPTERS, LEVELS};
use crate::program::{contains, Action, Program, Step, Vm};
use crate::save::{Progress, Saved};
use crate::sound::{Sfx, Sounds};
use crate::ui::*;
use crate::layout::{Layout, Shape};
use crate::tour::{self, Facts, Spot, Tour};
use crate::world::{Dir, Failure, Outcome, Tile, World};
use macroquad::prelude::*;

/// Height of one palette block, and the step from one to the next.
const BLOCK_H: f32 = 44.0;
const BLOCK_STEP: f32 = 51.0;
const SPEEDS: [(&str, f32); 4] = [("Slow", 0.9), ("Normal", 0.5), ("Fast", 0.22), ("Turbo", 0.07)];

#[derive(PartialEq, Clone, Copy)]
enum RunState {
    Editing,
    Running,
    Done,
}

#[derive(PartialEq, Clone, Copy)]
enum MsgKind {
    Info,
    Error,
    Success,
}

enum Modal {
    None,
    Intro,
    Code,
    Victory,
    OtherWorld(u64, String),
}

pub enum Nav {
    Stay,
    Back,
    Goto(usize),
    /// Take this world's World Check.
    Check(usize),
}

#[derive(PartialEq, Clone, Copy)]
enum AnimKind {
    /// A block changed `count`: nothing moves, the count box lights up.
    Var,
    Move,
    Turn,
    Mine,
    Place,
    Bump,
    Hazard,
    Shake,
}

struct Anim {
    kind: AnimKind,
    t: f32,
    dur: f32,
    from: (i32, i32),
    to: (i32, i32),
    from_dir: Dir,
    target: (i32, i32),
    old_tile: Tile,
    fired: bool,
    gem: Option<bool>, // Some(true) = diamond, Some(false) = emerald
}

impl Anim {
    fn progress(&self) -> f32 {
        (self.t / self.dur).min(1.0)
    }
    fn finished(&self) -> bool {
        self.t >= self.dur
    }
}

struct Particle {
    pos: Vec2,
    vel: Vec2,
    color: Color,
    life: f32,
    size: f32,
}

struct Floater {
    pos: Vec2,
    text: String,
    color: Color,
    life: f32,
}

pub struct Play {
    pub idx: usize,
    level: &'static Level,
    seed: u64,
    start: World,
    world: World,
    pub program: Program,
    editor: Editor,
    state: RunState,
    vm: Option<Vm>,
    stepping: bool,
    anim: Option<Anim>,
    particles: Vec<Particle>,
    floaters: Vec<Floater>,
    msg: String,
    msg_kind: MsgKind,
    running_id: Option<u32>,
    error_id: Option<u32>,
    speed: usize,
    modal: Modal,
    win_timer: Option<f32>,
    stars: u8,
    shake: f32,
    shirt: usize,
    /// Earlier and undone versions of the program, for Undo / Redo.
    undo_stack: Vec<Program>,
    redo_stack: Vec<Program>,
    /// Set by undo/redo so that restoring a version isn't itself recorded.
    restoring: bool,
    /// The highlighted block whose tab we last switched to.
    followed: Option<u32>,
    /// Smart hints for the program and world they were worked out for
    /// (serialized program, seed), and how many have been shown.
    hints: Option<(String, u64, Vec<Hint>)>,
    hints_shown: usize,
    /// Lights up the count box when a block changes it (1 = just changed).
    count_flash: f32,
    /// How far the palette's list of blocks is scrolled, and its scrollbar.
    palette_scroll: f32,
    palette_bar: Scrollbar,
    /// Where everything goes this frame, for this screen's shape.
    lay: Layout,
    /// How far the Rust code view is scrolled, and its scrollbar.
    code_scroll: f32,
    code_bar: Scrollbar,
    /// The guided tour, while it runs.
    tour: Option<Tour>,
    /// A click the tour kept from reaching the screen, for its own buttons.
    tour_click: bool,
}

/// Where palette block `i` is drawn in the palette's `list` when it isn't scrolled.
pub fn palette_button(list: Rect, i: usize) -> Rect {
    Rect::new(list.x + 10.0, list.y + 4.0 + i as f32 * BLOCK_STEP, list.w - 20.0, BLOCK_H)
}

/// How many palette blocks fit in `list` at once.
pub fn palette_rows(list: Rect) -> usize {
    ((list.h - 4.0 - BLOCK_H) / BLOCK_STEP).max(0.0) as usize + 1
}

/// Where each spot the tour points at is on screen.
pub fn spot_rect(lay: &Layout, spot: Spot) -> Option<Rect> {
    let span = |a: Rect, b: Rect| a.combine_with(b);
    Some(match spot {
        Spot::None => return None,
        Spot::World => lay.world,
        Spot::FirstBlock => palette_button(lay.palette_list()?, 0),
        Spot::Program => lay.editor,
        Spot::StepReset => span(lay.step, lay.reset),
        Spot::HintGoal => span(lay.hint, lay.goal),
        Spot::Help => lay.help,
        Spot::Run => lay.run,
    })
}

fn fresh_seed() -> u64 {
    (get_time() * 1_000_000.0) as u64 ^ (macroquad::rand::rand() as u64) << 20
}

fn ease(t: f32) -> f32 {
    t * t * (3.0 - 2.0 * t)
}

impl Play {
    pub fn new(idx: usize, saved: Option<Program>, speed: usize, shirt: usize) -> Play {
        let level = &LEVELS[idx];
        let program = saved.unwrap_or_default();
        // A first visit to level 1 gets the guided tour instead of the intro.
        let tour = (idx == 0 && program.is_empty()).then(|| Tour::new(crate::keyboard::is_touch()));
        let seed = fresh_seed();
        let start = World::from_level(level, seed);
        let mut p = Play {
            idx,
            level,
            seed,
            world: start.clone(),
            start,
            program,
            editor: Editor::new(),
            state: RunState::Editing,
            vm: None,
            stepping: false,
            anim: None,
            particles: Vec::new(),
            floaters: Vec::new(),
            msg: String::new(),
            msg_kind: MsgKind::Info,
            running_id: None,
            error_id: None,
            speed,
            modal: if level.intro.is_some() && tour.is_none() { Modal::Intro } else { Modal::None },
            win_timer: None,
            stars: 0,
            shake: 0.0,
            shirt,
            undo_stack: Vec::new(),
            redo_stack: Vec::new(),
            restoring: false,
            followed: None,
            hints: None,
            hints_shown: 0,
            count_flash: 0.0,
            palette_scroll: 0.0,
            palette_bar: Scrollbar::default(),
            lay: Layout::new(VW, VH, false),
            code_scroll: 0.0,
            code_bar: Scrollbar::default(),
            tour,
            tour_click: false,
        };
        p.reset_run();
        p
    }

    pub fn speed(&self) -> usize {
        self.speed
    }

    // Hooks for the screenshot autopilot (see BC_SHOT in main.rs).
    pub fn debug_solve(&mut self) {
        self.modal = Modal::None;
        self.tour = None;
        let src = crate::dev_env("BC_PROGRAM").unwrap_or_else(|| self.level.solution.to_string());
        self.program = Program::deserialize(&src).unwrap_or_default();
    }
    pub fn debug_code(&mut self) {
        self.code_scroll = 0.0;
        self.modal = Modal::Code;
    }
    pub fn debug_run(&mut self, sfx: &Sounds) {
        self.modal = Modal::None;
        self.tour = None;
        self.start(false);
        self.step_once(sfx);
    }
    pub fn debug_finished(&self) -> bool {
        self.state == RunState::Done && self.win_timer.is_none() && self.particles.len() < 5
    }

    fn say(&mut self, msg: impl Into<String>, kind: MsgKind) {
        self.msg = msg.into();
        self.msg_kind = kind;
    }

    /// Bit's next hint for this program in this world: each press goes a step
    /// further, from a nudge to the exact change.
    fn show_hint(&mut self) {
        let key = self.program.serialize();
        let fresh = !matches!(&self.hints, Some((k, s, _)) if *k == key && *s == self.seed);
        if fresh {
            let ladder = hints::ladder(self.level, &self.program, &self.start, self.seed);
            self.hints = Some((key, self.seed, ladder));
            self.hints_shown = 0;
        }
        let Some((_, _, ladder)) = &self.hints else { return };
        let i = self.hints_shown.min(ladder.len() - 1);
        let hint = ladder[i].clone();
        let more = i + 1 < ladder.len();
        self.hints_shown += 1;
        self.editor.hint = hint.block;
        let text = if more { format!("{}  (Still stuck? Press Hint again.)", hint.text) } else { hint.text };
        self.say(text, MsgKind::Info);
    }

    fn goal_text(&self) -> String {
        let w = &self.start;
        let mut parts = Vec::new();
        if w.diamonds_total > 0 {
            parts.push(format!("mine all {} diamond{}", w.diamonds_total, if w.diamonds_total == 1 { "" } else { "s" }));
        }
        if w.emeralds_total > 0 {
            parts.push(format!("collect all {} emeralds", w.emeralds_total));
        }
        let mut s = String::from("Goal: ");
        if !parts.is_empty() {
            s.push_str(&parts.join(" and "));
            s.push_str(", then ");
        }
        s.push_str("walk into the portal.");
        if self.level.max_blocks.is_some() {
            s.push_str(&format!(" You can use up to {} blocks.", self.level.max_blocks.unwrap()));
        }
        if self.level.is_random() {
            s.push_str(" This world CHANGES each time - your code must work in every version! Try the dice button.");
        }
        s
    }

    fn reset_run(&mut self) {
        self.editor.hint = None;
        self.world = self.start.clone();
        self.state = RunState::Editing;
        self.vm = None;
        self.anim = None;
        self.stepping = false;
        self.running_id = None;
        self.error_id = None;
        self.win_timer = None;
        self.particles.clear();
        self.floaters.clear();
        let g = self.goal_text();
        self.say(g, MsgKind::Info);
    }

    fn new_world(&mut self, seed: u64) {
        self.seed = seed;
        self.start = World::from_level(self.level, seed);
        self.reset_run();
    }

    fn start(&mut self, stepping: bool) -> bool {
        self.reset_run();
        if self.program.main.is_empty() {
            let m = if self.program.func.is_empty() {
                "Your program is empty! Click some blocks to add them first."
            } else {
                "Your MAIN program is empty! Add a CALL FUNCTION block to MAIN to use your function."
            };
            self.say(m, MsgKind::Error);
            return false;
        }
        self.vm = Some(Vm::new(&self.program));
        self.state = RunState::Running;
        self.stepping = stepping;
        if stepping {
            self.say("Step mode: press STEP to run one block at a time. Press RUN to go at full speed.", MsgKind::Info);
        } else {
            self.say("Running... watch the highlighted block in your program!", MsgKind::Info);
        }
        true
    }

    fn fail(&mut self, msg: String, id: Option<u32>, sfx: &Sounds) {
        self.state = RunState::Done;
        self.error_id = id;
        self.running_id = None;
        self.say(format!("{msg}  (The block with the red outline is where it went wrong. Press RESET and try again!)"), MsgKind::Error);
        self.shake = 0.35;
        sfx.play(Sfx::Fail);
    }

    fn step_once(&mut self, sfx: &Sounds) {
        let Some(vm) = self.vm.as_mut() else { return };
        let dur = SPEEDS[self.speed].1;
        match vm.next(&self.world) {
            Step::Act(a, id) => {
                self.running_id = Some(id);
                let before = self.world.clone();
                let out = self.world.apply(a);
                let target = before.front();
                let mut anim = Anim {
                    kind: AnimKind::Shake,
                    t: 0.0,
                    dur,
                    from: before.pos,
                    to: self.world.pos,
                    from_dir: before.dir,
                    target,
                    old_tile: before.tile(target),
                    fired: false,
                    gem: None,
                };
                let failed = matches!(out, Outcome::Fail(_));
                anim.kind = match (a, &out) {
                    (Action::Forward, Outcome::Fail(Failure::Hazard(_))) => AnimKind::Hazard,
                    (Action::Forward, Outcome::Fail(_)) => AnimKind::Bump,
                    (Action::Forward, _) => AnimKind::Move,
                    (Action::TurnLeft | Action::TurnRight, _) => AnimKind::Turn,
                    (_, Outcome::Fail(_)) => AnimKind::Shake,
                    (Action::Mine, _) => AnimKind::Mine,
                    (Action::Place, _) => AnimKind::Place,
                };
                if anim.kind == AnimKind::Turn {
                    anim.dur = dur * 0.7;
                }
                if self.world.emeralds > before.emeralds {
                    anim.gem = Some(false);
                }
                if self.world.diamonds > before.diamonds {
                    anim.gem = Some(true);
                }
                match anim.kind {
                    AnimKind::Move => sfx.play(Sfx::Step),
                    AnimKind::Turn => sfx.play(Sfx::Turn),
                    AnimKind::Place => sfx.play(Sfx::Place),
                    AnimKind::Bump | AnimKind::Shake => sfx.play(Sfx::Bonk),
                    _ => {}
                }
                self.anim = Some(anim);
                match out {
                    Outcome::Ok => {}
                    Outcome::Won => {
                        self.state = RunState::Done;
                        self.win_timer = Some(dur + 0.4);
                    }
                    Outcome::Fail(f) => self.fail(f.message(), Some(id), sfx),
                }
                let _ = failed;
            }
            Step::Var(id) => {
                self.running_id = Some(id);
                self.count_flash = 1.0;
                sfx.play(Sfx::Click);
                let w = &self.world;
                self.anim = Some(Anim {
                    kind: AnimKind::Var,
                    t: 0.0,
                    dur: dur * 0.6,
                    from: w.pos,
                    to: w.pos,
                    from_dir: w.dir,
                    target: w.front(),
                    old_tile: w.tile(w.front()),
                    fired: false,
                    gem: None,
                });
            }
            Step::Done => {
                self.state = RunState::Done;
                self.running_id = None;
                let m = self.world.unfinished_message();
                self.say(m, MsgKind::Error);
                sfx.play(Sfx::Fail);
            }
            Step::Error(m, id) => self.fail(m, id, sfx),
        }
    }

    fn on_win(&mut self, sfx: &Sounds, progress: &mut Progress) {
        if let Some((seed, e)) = hints::failing_test_world(self.level, &self.program, self.seed) {
            self.say("It worked in this world... but not in every world!", MsgKind::Error);
            self.modal = Modal::OtherWorld(seed, e);
            sfx.play(Sfx::Fail);
            return;
        }
        let used = self.program.block_count();
        let par = self.level.par;
        self.stars = if used <= par { 3 } else if used <= par + 3 { 2 } else { 1 };
        progress.stars[self.idx] = progress.stars[self.idx].max(self.stars);
        progress.programs[self.idx] = Some(self.program.clone());
        if self.saved(progress) == Saved::StartedOverElsewhere {
            return;
        }
        sfx.play(Sfx::Win);
        self.say("You did it!", MsgKind::Success);
        self.modal = Modal::Victory;
        // Confetti!
        let c = vec2(self.world.pos.0 as f32 + 0.5, self.world.pos.1 as f32 + 0.5);
        for i in 0..60 {
            let a = i as f32 * 0.7;
            let sp = 2.0 + (i % 7) as f32 * 0.6;
            let colors = [rgb8(255, 215, 40), rgb8(90, 220, 215), rgb8(110, 240, 140), rgb8(215, 130, 255)];
            self.particles.push(Particle {
                pos: c,
                vel: vec2(a.cos() * sp, a.sin() * sp - 3.0),
                color: colors[i % 4],
                life: 1.6,
                size: 0.1,
            });
        }
    }

    fn burst(&mut self, at: (i32, i32), colors: &[Color], n: usize) {
        let c = vec2(at.0 as f32 + 0.5, at.1 as f32 + 0.5);
        for i in 0..n {
            let a = i as f32 * 2.399;
            let sp = 1.0 + (i % 5) as f32 * 0.5;
            self.particles.push(Particle {
                pos: c + vec2(a.cos(), a.sin()) * 0.2,
                vel: vec2(a.cos() * sp, a.sin() * sp - 2.5),
                color: colors[i % colors.len()],
                life: 0.7,
                size: 0.09 + (i % 3) as f32 * 0.03,
            });
        }
    }

    fn update(&mut self, dt: f32, sfx: &Sounds, progress: &mut Progress) {
        self.shake = (self.shake - dt).max(0.0);
        self.count_flash = (self.count_flash - dt * 1.5).max(0.0);
        let mut fire = None;
        if let Some(a) = self.anim.as_mut() {
            a.t += dt;
            let trigger = match a.kind {
                AnimKind::Mine => 0.55,
                AnimKind::Move => 0.8,
                _ => 2.0,
            };
            if !a.fired && a.progress() >= trigger {
                a.fired = true;
                fire = Some((a.kind, a.target, a.to, a.old_tile, a.gem));
            }
        }
        if let Some((kind, target, to, old, gem)) = fire {
            if kind == AnimKind::Mine {
                sfx.play(Sfx::Mine);
                let cols: Vec<Color> = match old {
                    Tile::Ore => vec![rgb8(127, 127, 127), rgb8(90, 225, 215), rgb8(200, 255, 250)],
                    _ => vec![rgb8(127, 127, 127), rgb8(95, 95, 95), rgb8(160, 160, 160)],
                };
                self.burst(target, &cols, 16);
            }
            match gem {
                Some(true) => {
                    sfx.play(Sfx::Collect);
                    self.floaters.push(Floater {
                        pos: vec2(target.0 as f32 + 0.5, target.1 as f32 + 0.2),
                        text: "+1 diamond!".into(),
                        color: rgb8(120, 240, 235),
                        life: 1.0,
                    });
                }
                Some(false) => {
                    sfx.play(Sfx::Collect);
                    self.burst(to, &[rgb8(110, 240, 140), rgb8(40, 190, 85), WHITE], 12);
                    self.floaters.push(Floater {
                        pos: vec2(to.0 as f32 + 0.5, to.1 as f32 + 0.2),
                        text: "+1 emerald!".into(),
                        color: rgb8(130, 255, 150),
                        life: 1.0,
                    });
                }
                None => {}
            }
        }
        for p in &mut self.particles {
            p.vel.y += 7.0 * dt;
            p.pos += p.vel * dt;
            p.life -= dt;
        }
        self.particles.retain(|p| p.life > 0.0);
        for f in &mut self.floaters {
            f.pos.y -= dt * 0.8;
            f.life -= dt;
        }
        self.floaters.retain(|f| f.life > 0.0);

        if let Some(t) = self.win_timer.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.win_timer = None;
                self.on_win(sfx, progress);
            }
        }

        let anim_done = self.anim.as_ref().is_none_or(|a| a.finished());
        if self.state == RunState::Running && !self.stepping && anim_done {
            self.step_once(sfx);
        }
    }

    fn press_run(&mut self, sfx: &Sounds) {
        sfx.play(Sfx::Click);
        match self.state {
            RunState::Editing | RunState::Done => {
                self.start(false);
            }
            RunState::Running => {
                self.stepping = !self.stepping;
                if self.stepping {
                    self.say("Paused. Press STEP to go one block at a time, or RUN to keep going.", MsgKind::Info);
                } else {
                    self.say("Running... watch the highlighted block in your program!", MsgKind::Info);
                }
            }
        }
    }

    fn press_step(&mut self, sfx: &Sounds) {
        sfx.play(Sfx::Click);
        match self.state {
            RunState::Editing | RunState::Done => {
                if self.start(true) {
                    self.step_once(sfx);
                }
            }
            RunState::Running => {
                self.stepping = true;
                if self.anim.as_ref().is_none_or(|a| a.finished()) {
                    self.step_once(sfx);
                }
            }
        }
    }

    /// One frame on a `canvas` (width, height) the size of the screen.
    pub fn frame(&mut self, ui: &mut Ui, art: &Art, sfx: &mut Sounds, progress: &mut Progress, canvas: (f32, f32)) -> Nav {
        let dt = get_frame_time().min(0.1);
        self.update(dt, sfx, progress);
        self.lay = Layout::new(canvas.0, canvas.1, self.state != RunState::Editing);
        // Remember the program as it was, so any change this frame can be undone.
        let before = self.program.clone();
        let mut nav = Nav::Stay;
        let modal_open = !matches!(self.modal, Modal::None);

        // During the tour, only the spot it points at can be clicked.
        let tour_spot = self.tour.as_ref().and_then(|t| spot_rect(&self.lay, t.current().spot));
        self.tour_click = false;
        if let Some(t) = &self.tour {
            if ui.click && !tour::allows(t.current(), tour_spot, ui.mouse) {
                ui.click = false;
                self.tour_click = true;
            }
            ui.rclick = false;
        }

        // Keyboard shortcuts
        if self.tour.is_some() {
            if is_key_pressed(KeyCode::Escape) {
                self.tour = None;
            }
        } else if !modal_open {
            if is_key_pressed(KeyCode::Enter) {
                self.press_run(sfx);
            }
            if is_key_pressed(KeyCode::Backspace) || is_key_pressed(KeyCode::Delete) {
                if self.editor.delete_before_cursor(&mut self.program) {
                    self.program_changed(progress);
                }
            }
            let ctrl = is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl);
            let shift = is_key_down(KeyCode::LeftShift) || is_key_down(KeyCode::RightShift);
            if ctrl && is_key_pressed(KeyCode::Z) {
                if shift { self.redo(progress, sfx) } else { self.undo(progress, sfx) }
            }
            if ctrl && is_key_pressed(KeyCode::Y) {
                self.redo(progress, sfx);
            }
            if ctrl && shift && is_key_pressed(KeyCode::S) {
                if let Some(p) = Program::deserialize(self.level.solution) {
                    self.program = p;
                    self.program_changed(progress);
                    self.say("Parent mode: loaded a working solution. Press RUN to watch it!", MsgKind::Success);
                }
            }
        } else if is_key_pressed(KeyCode::Escape) {
            self.modal = Modal::None;
        }

        ui.enabled = !modal_open;

        // Background
        let lay = self.lay.clone();
        draw_rectangle(0.0, 0.0, lay.w, lay.h, rgb8(28, 28, 34));

        // ---- Top bar ----
        draw_rectangle(0.0, 0.0, lay.w, lay.top_h, rgb8(50, 50, 60));
        draw_rectangle(0.0, lay.top_h - 2.0, lay.w, 2.0, BLACK);
        if ui.button(lay.levels, "< Levels", rgb8(110, 110, 120), 20.0) {
            sfx.play(Sfx::Click);
            nav = Nav::Back;
        }
        let ch = &CHAPTERS[self.level.chapter];
        let title = format!("Level {}: {}", self.idx + 1, self.level.name);
        // The title shrinks (a little) to stop short of the buttons or the stars.
        let room = match lay.shape {
            Shape::Wide => lay.code.x - lay.title.x - 16.0,
            Shape::Tall => lay.w - lay.title.x - 100.0,
        };
        let title_size = fit_size(&title, room, lay.title.size, 18.0);
        shadow_text(&title, lay.title.x, lay.title.y + (lay.title.size - title_size) / 2.0, title_size, WHITE);
        let sub = lay.subtitle;
        text(&format!("{}  -  learning: {}", ch.name, ch.concept), sub.x, sub.y, sub.size, rgb8(185, 185, 200));
        let best = progress.stars[self.idx];
        // Stars beside the title when there's room, else after it on the next line.
        let stars_at = match lay.stars_x {
            Some(x) if lay.title.x + text_width(&title, title_size) + 16.0 < x => Some((x, 28.0)),
            Some(_) => None,
            None => Some((lay.w - 92.0, lay.title.y + 12.0)),
        };
        if let Some((sx, sy)) = stars_at {
            for i in 0..3 {
                star(sx + i as f32 * 26.0, sy, 11.0, i < best);
            }
        }
        let code_label = if lay.shape == Shape::Tall { "Show code" } else { "Show Rust code" };
        if ui.button(lay.code, code_label, rgb8(183, 65, 14), 18.0) {
            sfx.play(Sfx::Click);
            self.code_scroll = 0.0;
            self.modal = Modal::Code;
        }
        if ui.button(lay.help, "Help", rgb8(60, 140, 90), 18.0) {
            sfx.play(Sfx::Click);
            if self.level.intro.is_some() {
                self.modal = Modal::Intro;
            } else {
                self.show_hint();
            }
        }
        let snd = if sfx.muted { "Sound: off" } else { "Sound: on" };
        if ui.button(lay.sound, snd, rgb8(90, 90, 110), 18.0) {
            sfx.muted = !sfx.muted;
            sfx.play(Sfx::Click);
        }

        // ---- World ----
        self.draw_world(art);

        // ---- HUD ----
        self.draw_hud(ui, art, sfx);

        // ---- Guide box ----
        self.draw_guide(ui, art, sfx);

        // ---- Palette & controls ----
        self.draw_palette(ui, art, sfx);

        // ---- Editor ----
        // Show the tab holding the running (or failed) block, but only when that
        // block changes, so kids can still switch tabs themselves afterwards.
        let highlighted = self.running_id.or(self.error_id).or(self.editor.hint);
        if highlighted != self.followed {
            self.followed = highlighted;
            if let Some(id) = highlighted {
                if contains(&self.program.func, id) {
                    self.editor.tab = Tab::Func;
                } else if contains(&self.program.main, id) {
                    self.editor.tab = Tab::Main;
                }
            }
        }
        let history = (!self.undo_stack.is_empty(), !self.redo_stack.is_empty());
        let out = self.editor.draw(ui, &mut self.program, self.level, lay.editor, self.running_id, self.error_id, art, history);
        if out.changed {
            self.program_changed(progress);
            sfx.play(Sfx::Click);
        }
        if out.undo {
            self.undo(progress, sfx);
        }
        if out.redo {
            self.redo(progress, sfx);
        }

        // ---- Popups ----
        ui.enabled = true;
        if modal_open {
            draw_rectangle(0.0, 0.0, lay.w, lay.h, Color::new(0.0, 0.0, 0.0, 0.55));
        }
        if let Some(n) = self.draw_modal(ui, art, sfx, progress) {
            nav = n;
        }
        self.draw_tour(ui, art);

        // Record this frame's change (if any) for Undo.
        if !self.restoring && before.serialize() != self.program.serialize() {
            self.undo_stack.push(before);
            if self.undo_stack.len() > HISTORY {
                self.undo_stack.remove(0);
            }
            self.redo_stack.clear();
        }
        self.restoring = false;
        nav
    }

    /// Moves the tour on when the player has done what it asked, and draws it.
    fn draw_tour(&mut self, ui: &mut Ui, art: &Art) {
        let facts = Facts { main_blocks: self.program.main.len(), running: self.state != RunState::Editing };
        let Some(t) = &mut self.tour else { return };
        let mut out = t.update(&facts);
        if out == tour::Out::Stay {
            // The tour's own buttons get the click it held back.
            ui.click = self.tour_click;
            out = tour::draw(t, spot_rect(&self.lay, t.current().spot), (self.lay.w, self.lay.h), ui, art);
        }
        if out == tour::Out::Done {
            self.tour = None;
        }
    }

    fn undo(&mut self, progress: &mut Progress, sfx: &Sounds) {
        if let Some(prev) = self.undo_stack.pop() {
            self.redo_stack.push(std::mem::replace(&mut self.program, prev));
            self.restoring = true;
            self.program_changed(progress);
            sfx.play(Sfx::Click);
        }
    }

    fn redo(&mut self, progress: &mut Progress, sfx: &Sounds) {
        if let Some(next) = self.redo_stack.pop() {
            self.undo_stack.push(std::mem::replace(&mut self.program, next));
            self.restoring = true;
            self.program_changed(progress);
            sfx.play(Sfx::Click);
        }
    }

    fn program_changed(&mut self, progress: &mut Progress) {
        self.editor.hint = None;
        if self.state != RunState::Editing {
            self.reset_run();
        }
        progress.programs[self.idx] = Some(self.program.clone());
        self.saved(progress);
    }

    /// Saves, and if the player started over in another tab or window, swaps
    /// in the stored (wiped) program. Otherwise this tab's old program would be
    /// saved again on the next edit, bringing it back.
    fn saved(&mut self, progress: &mut Progress) -> Saved {
        let saved = progress.save();
        if saved == Saved::StartedOverElsewhere {
            self.program = progress.programs[self.idx].clone().unwrap_or_default();
            self.editor = Editor::new();
            self.undo_stack.clear();
            self.redo_stack.clear();
            self.reset_run();
            self.say("This player started over in another window, so this level starts fresh too.", MsgKind::Info);
        }
        saved
    }

    fn draw_world(&self, art: &Art) {
        let w = &self.world;
        let area = self.lay.world;
        let ts = (area.w / w.w as f32).min(area.h / w.h as f32).floor().min(72.0);
        let shake = if self.shake > 0.0 { (get_time() * 60.0).sin() as f32 * self.shake * 14.0 } else { 0.0 };
        let ox = area.x + (area.w - ts * w.w as f32) / 2.0 + shake;
        let oy = area.y + (area.h - ts * w.h as f32) / 2.0;
        let time = get_time();
        draw_rectangle(area.x - 3.0, area.y - 3.0, area.w + 6.0, area.h + 6.0, BLACK);
        draw_rectangle(area.x, area.y, area.w, area.h, rgb8(20, 20, 24));
        let floor = self.level.floor;
        let anim = self.anim.as_ref();
        let ap = anim.map_or(1.0, |a| a.progress());
        let portal_on = w.collected_all();

        for y in 0..w.h {
            for x in 0..w.w {
                let px = ox + x as f32 * ts;
                let py = oy + y as f32 * ts;
                draw_tex(art.floor(floor), px, py, ts, ts, WHITE);
                let mut t = w.tile((x, y));
                if let Some(a) = anim {
                    if a.kind == AnimKind::Mine && a.target == (x, y) && !a.fired {
                        t = a.old_tile;
                    }
                }
                if let Some(tex) = art.tile(t, floor, time, portal_on) {
                    let placing = anim.is_some_and(|a| a.kind == AnimKind::Place && a.target == (x, y) && ap < 1.0);
                    if placing {
                        let s = ts * (0.4 + 0.6 * ease(ap));
                        draw_tex(tex, px + (ts - s) / 2.0, py + (ts - s) / 2.0, s, s, WHITE);
                    } else {
                        draw_tex(tex, px, py, ts, ts, WHITE);
                    }
                }
                if t == Tile::Emerald {
                    let bob = ((time * 3.0 + x as f64 * 0.7).sin() * ts as f64 * 0.05) as f32;
                    let s = ts * 0.62;
                    draw_ellipse(px + ts / 2.0, py + ts * 0.8, s * 0.35, s * 0.12, 0.0, Color::new(0.0, 0.0, 0.0, 0.25));
                    draw_tex(&art.emerald, px + (ts - s) / 2.0, py + (ts - s) / 2.0 + bob - ts * 0.05, s, s, WHITE);
                }
                if t == Tile::Portal && portal_on {
                    let g = ((time * 4.0).sin() * 0.15 + 0.25) as f32;
                    draw_rectangle(px, py, ts, ts, Color::new(0.8, 0.5, 1.0, g * 0.5));
                }
                draw_rectangle_lines(px, py, ts, ts, 1.0, Color::new(0.0, 0.0, 0.0, 0.12));
            }
        }

        // Cracks on the block being mined.
        if let Some(a) = anim {
            if a.kind == AnimKind::Mine && !a.fired {
                let (tx, ty) = (ox + a.target.0 as f32 * ts, oy + a.target.1 as f32 * ts);
                let stage = (ap / 0.55 * 4.0) as i32;
                let c = Color::new(0.0, 0.0, 0.0, 0.7);
                let cracks = [
                    (0.5, 0.5, 0.2, 0.25),
                    (0.5, 0.5, 0.8, 0.35),
                    (0.5, 0.5, 0.35, 0.85),
                    (0.2, 0.25, 0.1, 0.6),
                    (0.8, 0.35, 0.9, 0.8),
                ];
                for &(x0, y0, x1, y1) in cracks.iter().take(stage as usize + 1) {
                    draw_line(tx + x0 * ts, ty + y0 * ts, tx + x1 * ts, ty + y1 * ts, (ts / 16.0).max(2.0), c);
                }
            }
        }

        // The miner.
        let (fx, fy, dir, angle, tint, scale) = self.player_pose(ap);
        let cx = ox + (fx + 0.5) * ts;
        let cy = oy + (fy + 0.5) * ts;
        let hop = if anim.is_some_and(|a| a.kind == AnimKind::Move) && ap < 1.0 {
            (ap * std::f32::consts::PI).sin() * ts * 0.08
        } else {
            0.0
        };
        draw_ellipse(cx, cy + ts * 0.36, ts * 0.28 * scale, ts * 0.09 * scale, 0.0, Color::new(0.0, 0.0, 0.0, 0.3));
        let sh = ts * 0.88 * scale;
        let sw = sh * 12.0 / 16.0;
        draw_tex(art.player(self.shirt, dir), cx - sw / 2.0, cy - sh / 2.0 - hop - ts * 0.04, sw, sh, tint);

        // Direction arrow.
        let a = angle.to_radians();
        let d = vec2(a.sin(), -a.cos());
        let perp = vec2(-d.y, d.x);
        let c = vec2(cx, cy);
        let tip = c + d * ts * 0.56;
        let b1 = c + d * ts * 0.36 + perp * ts * 0.14;
        let b2 = c + d * ts * 0.36 - perp * ts * 0.14;
        draw_triangle(tip + d * 2.0, b1 + (b1 - c).normalize() * 2.0, b2 + (b2 - c).normalize() * 2.0, BLACK);
        draw_triangle(tip, b1, b2, rgb8(255, 225, 40));

        for p in &self.particles {
            let s = p.size * ts;
            draw_rectangle(ox + p.pos.x * ts - s / 2.0, oy + p.pos.y * ts - s / 2.0, s, s, p.color.with_alpha(p.life.min(1.0)));
        }
        for f in &self.floaters {
            let wdt = text_width(&f.text, 20.0);
            shadow_text(&f.text, ox + f.pos.x * ts - wdt / 2.0, oy + f.pos.y * ts, 20.0, f.color.with_alpha(f.life.min(1.0)));
        }
    }

    /// Where the miner is drawn mid-animation: (x, y, facing, arrow angle, tint, scale).
    fn player_pose(&self, ap: f32) -> (f32, f32, Dir, f32, Color, f32) {
        let w = &self.world;
        let mut x = w.pos.0 as f32;
        let mut y = w.pos.1 as f32;
        let mut dir = w.dir;
        let mut angle = w.dir.angle();
        let mut tint = WHITE;
        let mut scale = 1.0;
        if let Some(a) = &self.anim {
            let (dx, dy) = a.from_dir.delta();
            let (fx, fy) = (a.from.0 as f32, a.from.1 as f32);
            match a.kind {
                AnimKind::Move => {
                    let e = ease(ap);
                    x = fx + (a.to.0 as f32 - fx) * e;
                    y = fy + (a.to.1 as f32 - fy) * e;
                }
                AnimKind::Turn => {
                    let from = a.from_dir.angle();
                    let mut delta = w.dir.angle() - from;
                    if delta > 180.0 {
                        delta -= 360.0;
                    }
                    if delta < -180.0 {
                        delta += 360.0;
                    }
                    angle = from + delta * ease(ap);
                    if ap < 0.5 {
                        dir = a.from_dir;
                    }
                }
                AnimKind::Bump => {
                    let k = (ap * std::f32::consts::PI).sin() * 0.25;
                    x = fx + dx as f32 * k;
                    y = fy + dy as f32 * k;
                }
                AnimKind::Hazard => {
                    let k = (ap * 2.0).min(1.0) * 0.6;
                    x = fx + dx as f32 * k;
                    y = fy + dy as f32 * k;
                    tint = Color::new(1.0, 1.0 - ap * 0.7, 1.0 - ap * 0.7, 1.0);
                    scale = 1.0 - ap * 0.35;
                }
                _ => {}
            }
        }
        (x, y, dir, angle, tint, scale)
    }

    fn draw_hud(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds) {
        let w = &self.world;
        let hud = self.lay.hud;
        let y = hud.y;
        dark_panel(hud);
        // On narrow screens the backpack is just its picture and number.
        let compact = hud.w < 640.0;
        let dice = self.level.is_random().then(|| {
            let w = if compact { 116.0 } else { 144.0 };
            Rect::new(hud.x + hud.w - w - 6.0, y + 4.0, w, 30.0)
        });
        let mut x = hud.x + 10.0;
        draw_tex(&art.cobble, x, y + 7.0, 24.0, 24.0, WHITE);
        let backpack = if compact { w.blocks.to_string() } else { format!("Backpack: {}", w.blocks) };
        shadow_text(&backpack, x + 32.0, y + 10.0, 20.0, WHITE);
        x += if compact { 76.0 } else { 190.0 };
        let gem_step = if compact { 96.0 } else { 110.0 };
        if w.diamonds_total > 0 {
            draw_tex(&art.diamond, x, y + 6.0, 26.0, 26.0, WHITE);
            shadow_text(&format!("{} / {}", w.diamonds, w.diamonds_total), x + 32.0, y + 10.0, 20.0, rgb8(150, 245, 240));
            x += gem_step;
        }
        if w.emeralds_total > 0 {
            draw_tex(&art.emerald, x, y + 6.0, 26.0, 26.0, WHITE);
            shadow_text(&format!("{} / {}", w.emeralds, w.emeralds_total), x + 32.0, y + 10.0, 20.0, rgb8(150, 255, 170));
            x += gem_step;
        }
        // The `count` variable, drawn as a labelled box holding its number.
        if self.level.palette.iter().any(|k| k.uses_count()) {
            let value = self.vm.as_ref().map_or(0, |vm| vm.count);
            let glow = self.count_flash;
            shadow_text("count", x, y + 10.0, 20.0, rgb8(140, 235, 220));
            let b = Rect::new(x + 66.0, y + 4.0, 46.0, 30.0);
            // The box keeps a readable colour; a change flashes its outline yellow.
            let bg = kind_color(crate::program::Kind::Set);
            draw_rectangle(b.x, b.y, b.w, b.h, bg);
            let edge = Color::new(1.0, 1.0 - glow * 0.15, 1.0 - glow * 0.85, 1.0);
            draw_rectangle_lines(b.x, b.y, b.w, b.h, 2.0 + glow * 2.0, edge);
            text_centered(&value.to_string(), b, 22.0, text_on(bg), true);
            x += 130.0;
        }
        if let Some(vm) = &self.vm {
            // Only when there's room before the dice button.
            let steps = format!("Steps: {}", vm.actions);
            if x + text_width(&steps, 20.0) < dice.map_or(hud.x + hud.w, |d| d.x) - 8.0 {
                shadow_text(&steps, x, y + 10.0, 20.0, rgb8(220, 220, 220));
            }
        }
        if let Some(d) = dice {
            let label = if compact { "New world" } else { "[dice] New world" };
            if ui.button(d, label, rgb8(70, 130, 180), 18.0) {
                sfx.play(Sfx::Click);
                self.new_world(fresh_seed());
            }
        }
    }

    fn draw_guide(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds) {
        let r = self.lay.guide;
        dark_panel(r);
        let bob = ((get_time() * 2.0).sin() * 3.0) as f32;
        // Hint and Goal either sit under the message, or (on narrow screens)
        // under Bit, who then gets smaller and the message gets the full height.
        let side_buttons = self.lay.hint.x < r.x + 100.0;
        let bit = if side_buttons { 40.0 } else { (r.h - 50.0).clamp(48.0, 72.0) };
        draw_tex(&art.bit, r.x + 14.0 + (72.0 - bit) / 2.0, r.y + (if side_buttons { 8.0 } else { 14.0 }) + bob, bit, bit, WHITE);
        if r.h >= 130.0 && !side_buttons {
            shadow_text("Bit", r.x + 34.0, r.y + bit + 26.0, 20.0, rgb8(120, 230, 220));
        }
        let color = match self.msg_kind {
            MsgKind::Info => rgb8(235, 235, 235),
            MsgKind::Error => rgb8(255, 150, 130),
            MsgKind::Success => rgb8(140, 255, 150),
        };
        let tx = r.x + 104.0;
        draw_triangle(vec2(tx - 12.0, r.y + 50.0), vec2(tx, r.y + 42.0), vec2(tx, r.y + 58.0), rgb8(60, 60, 72));
        let bubble = Rect::new(tx, r.y + 12.0, r.w - 116.0, if side_buttons { r.h - 24.0 } else { r.h - 62.0 });
        draw_rectangle(bubble.x, bubble.y, bubble.w, bubble.h, rgb8(60, 60, 72));
        // Long messages in a small box step down a little, but never below 16px.
        let size = [19.0, 17.5, 16.0]
            .into_iter()
            .find(|&s| wrap(&self.msg, bubble.w - 20.0, s).len() as f32 * s * 1.3 <= bubble.h - 12.0)
            .unwrap_or(16.0);
        paragraph(&self.msg, tx + 10.0, r.y + 20.0, bubble.w - 20.0, size, color);
        if ui.button(self.lay.hint, "Hint", rgb8(200, 160, 40), 18.0) {
            sfx.play(Sfx::Click);
            self.show_hint();
        }
        if ui.button(self.lay.goal, "Goal", rgb8(78, 78, 96), 18.0) {
            sfx.play(Sfx::Click);
            let g = self.goal_text();
            self.say(g, MsgKind::Info);
        }
        // Keyboard and mouse tips, where there's room and a keyboard.
        if r.w >= 600.0 && !crate::keyboard::is_touch() {
            text(
                "Drag blocks to move them    Ctrl+Z = undo",
                r.x + r.w - 350.0,
                r.y + r.h - 32.0,
                15.0,
                rgb8(165, 165, 180),
            );
        }
    }

    fn draw_palette(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds) {
        if let (Some(r), Some(list)) = (self.lay.palette, self.lay.palette_list()) {
            self.draw_blocks(ui, art, sfx, r, list);
        }
        self.draw_controls(ui, sfx);
    }

    /// The palette of blocks to add, in `r`, listed in `list`.
    fn draw_blocks(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, r: Rect, list: Rect) {
        dark_panel(r);
        shadow_text("BLOCKS", r.x + 12.0, r.y + 9.0, 22.0, WHITE);
        let full = self.level.max_blocks.is_some_and(|m| self.program.block_count() >= m);
        // Scroll the list when it's taller than the panel (wheel or scrollbar).
        // It moves a whole block at a time, so no block is ever cut in half.
        let hidden = self.level.palette.len().saturating_sub(palette_rows(list));
        let overflow = hidden > 0;
        if ui.hover(list) && ui.wheel != 0.0 {
            self.palette_scroll -= ui.wheel.signum() * BLOCK_STEP;
        }
        let track = Rect::new(list.x + list.w - SCROLLBAR_W - 4.0, list.y, SCROLLBAR_W, list.h);
        let content = list.h + hidden as f32 * BLOCK_STEP;
        self.palette_bar.update(ui, track, content, list.h, &mut self.palette_scroll);
        let first = ((self.palette_scroll / BLOCK_STEP).round() as usize).min(hidden);
        for (i, &k) in self.level.palette.iter().enumerate() {
            let mut b = palette_button(list, i);
            b.y -= first as f32 * BLOCK_STEP;
            if overflow {
                b.w -= SCROLLBAR_W + 2.0;
            }
            // Only whole blocks are shown, so a hidden one can't be clicked.
            if b.y < list.y - 1.0 || b.y + b.h > list.y + list.h + 1.0 {
                continue;
            }
            let c = if full { rgb8(90, 90, 90) } else { kind_color(k) };
            let hover = ui.hover(b) && !full;
            let tc = text_on(c);
            let dark = tc != WHITE;
            bevel(b, if hover { hover_shade(c, !dark) } else { c });
            block_icon(k, b.x + 6.0, b.y + 10.0, 24.0, tc, art);
            let label = k.palette_label();
            // Narrow palettes shrink a long label a little to keep it on its block.
            let size = fit_size(label, b.w - 44.0, 20.0, 16.0);
            let ly = b.y + 13.0 + (20.0 - size) / 2.0;
            if dark {
                text(label, b.x + 38.0, ly, size, tc);
            } else {
                shadow_text(label, b.x + 38.0, ly, size, WHITE);
            }
            if !full && ui.clicked(b) {
                // Click to add at the cursor, or drag it into place.
                self.editor.press_palette(k, ui.mouse);
            } else if full && ui.clicked(b) {
                self.say(
                    format!("You've used all {} blocks! Can you find a pattern and use a loop to make your program shorter?", self.level.max_blocks.unwrap()),
                    MsgKind::Error,
                );
                sfx.play(Sfx::Bonk);
            }
        }

    }

    /// RUN, STEP, RESET and speed, in their own panel.
    fn draw_controls(&mut self, ui: &mut Ui, sfx: &Sounds) {
        dark_panel(self.lay.controls);
        let run_label = match (self.state, self.stepping) {
            (RunState::Running, false) => "PAUSE",
            (RunState::Running, true) => "RUN >",
            (RunState::Done, _) => "RUN AGAIN",
            _ => "RUN >",
        };
        let run_color = if self.state == RunState::Running && !self.stepping { rgb8(200, 150, 30) } else { rgb8(60, 170, 60) };
        if ui.button(self.lay.run, run_label, run_color, 26.0) {
            self.press_run(sfx);
        }
        if ui.button(self.lay.step, "STEP", rgb8(70, 130, 180), 20.0) {
            self.press_step(sfx);
        }
        if ui.button(self.lay.reset, "RESET", rgb8(150, 70, 60), 20.0) {
            sfx.play(Sfx::Click);
            self.reset_run();
        }
        let speed_label = format!("Speed: {}", SPEEDS[self.speed].0);
        if ui.button(self.lay.speed, &speed_label, rgb8(90, 90, 110), 18.0) {
            sfx.play(Sfx::Click);
            self.speed = (self.speed + 1) % SPEEDS.len();
        }
    }

    fn draw_modal(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, progress: &mut Progress) -> Option<Nav> {
        match &self.modal {
            Modal::None => None,
            Modal::Intro => {
                let (title, body) = self.level.intro.unwrap_or(("", ""));
                let r = self.lay.popup(760.0, 540.0, 120.0);
                let narrow = r.w < 600.0;
                panel(r);
                draw_tex(&art.bit, r.x + 24.0, r.y + 24.0, 80.0, 80.0, WHITE);
                let size = fit_size(title, r.w - 144.0, 32.0, 22.0);
                text(title, r.x + 124.0, r.y + 44.0, size, TITLE_ON_LIGHT);
                draw_rectangle(r.x + 20.0, r.y + 118.0, r.w - 40.0, r.h - 200.0, rgb8(60, 60, 72));
                paragraph(body, r.x + 36.0, r.y + 134.0, r.w - 72.0, 21.0, WHITE);
                // Narrow popups put "Let's go!" on the right, clear of "Show me around".
                let go = if narrow {
                    Rect::new(r.x + r.w - 204.0, r.y + r.h - 66.0, 180.0, 48.0)
                } else {
                    Rect::new(r.x + r.w / 2.0 - 110.0, r.y + r.h - 66.0, 220.0, 48.0)
                };
                if ui.button(go, "Let's go!", rgb8(60, 170, 60), 26.0) {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                }
                // Level 1 can replay the guided tour.
                if self.idx == 0
                    && ui.button(Rect::new(r.x + 24.0, r.y + r.h - 60.0, 180.0, 36.0), "Show me around", rgb8(70, 130, 180), 18.0)
                {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                    self.tour = Some(Tour::new(crate::keyboard::is_touch()));
                }
                None
            }
            Modal::Code => {
                let r = self.lay.popup(840.0, 660.0, 70.0);
                panel(r);
                let heading = "Your program, written in Rust";
                text(heading, r.x + 24.0, r.y + 18.0, fit_size(heading, r.w - 48.0, 30.0, 20.0), WARN_ON_LIGHT);
                let said = paragraph(
                    "Real developers type code like this. Every block you snap together matches a line of real Rust!",
                    r.x + 24.0,
                    r.y + 56.0,
                    r.w - 48.0,
                    17.0,
                    rgb8(40, 40, 40),
                );
                let top = r.y + 62.0 + said;
                let code = Rect::new(r.x + 20.0, top, r.w - 40.0, r.y + r.h - 76.0 - top);
                draw_rectangle(code.x, code.y, code.w, code.h, rgb8(30, 30, 38));
                let lines = self.program.to_rust();
                // Smaller for long programs, and small enough that the widest line fits.
                let base = if lines.len() > 22 { 16.0 } else { 20.0 };
                let widest = lines.iter().max_by_key(|l| l.len()).map_or("", |l| l.as_str());
                // Long programs scroll, a line at a time, with the wheel or a scrollbar.
                let fit = |room: f32| {
                    let size = fit_code_size(widest, room, base, 11.0);
                    let lh = size * 1.25;
                    let shown = (((code.h - 20.0) / lh).floor() as usize).max(1);
                    (size, lh, shown, lines.len().saturating_sub(shown))
                };
                let (mut size, mut lh, mut shown, mut hidden) = fit(code.w - 32.0);
                if hidden > 0 {
                    (size, lh, shown, hidden) = fit(code.w - 32.0 - SCROLLBAR_W - 6.0);
                }
                if ui.hover(code) && ui.wheel != 0.0 {
                    self.code_scroll -= ui.wheel.signum() * lh * 3.0;
                }
                let track = Rect::new(code.x + code.w - SCROLLBAR_W - 4.0, code.y + 4.0, SCROLLBAR_W, code.h - 8.0);
                self.code_bar.update(ui, track, code.h + hidden as f32 * lh, code.h, &mut self.code_scroll);
                let first = ((self.code_scroll / lh).round() as usize).min(hidden);
                for (i, line) in lines.iter().skip(first).take(shown).enumerate() {
                    draw_code_line(line, code.x + 16.0, code.y + 12.0 + i as f32 * lh, size);
                }
                // Wraps to the left of the Close button on narrow screens.
                let fun = "Fun fact: this whole game is written in Rust too!";
                let room = r.w - 48.0 - 170.0;
                let fun_size = if text_width(fun, 18.0) <= room { 18.0 } else { 16.0 };
                paragraph(fun, r.x + 24.0, r.y + r.h - if text_width(fun, fun_size) <= room { 56.0 } else { 64.0 }, room, fun_size, rgb8(40, 40, 40));
                if ui.button(Rect::new(r.x + r.w - 160.0, r.y + r.h - 62.0, 140.0, 44.0), "Close", rgb8(90, 90, 110), 22.0) {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                }
                None
            }
            Modal::OtherWorld(seed, err) => {
                let (seed, err) = (*seed, err.clone());
                let narrow = self.lay.w < 700.0;
                let r = self.lay.popup(680.0, if narrow { 440.0 } else { 380.0 }, 200.0);
                panel(r);
                draw_tex(&art.bit, r.x + 24.0, r.y + 24.0, 72.0, 72.0, WHITE);
                text("So close!", r.x + 116.0, r.y + 40.0, 34.0, WARN_ON_LIGHT);
                let mut y = r.y + 116.0;
                y += paragraph(
                    "Your program worked in THIS world. But I tested it in a different version of the level, and this happened:",
                    r.x + 30.0,
                    y,
                    r.w - 60.0,
                    20.0,
                    rgb8(30, 30, 30),
                );
                y += 10.0;
                draw_rectangle(r.x + 24.0, y - 6.0, r.w - 48.0, 84.0, rgb8(60, 60, 72));
                paragraph(&err, r.x + 36.0, y + 2.0, r.w - 72.0, 19.0, rgb8(255, 160, 140));
                let mut out = None;
                let show_w = 300f32.min(r.w - 60.0 - 152.0);
                if ui.button(Rect::new(r.x + 30.0, r.y + r.h - 64.0, show_w, 46.0), "Show me that world", rgb8(60, 140, 200), 22.0) {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                    self.new_world(seed);
                    self.say(
                        "Here's the world where your program had trouble. Press RUN to watch what goes wrong. Hint: use blocks that CHECK the world, like IF and WHILE.",
                        MsgKind::Info,
                    );
                    out = Some(Nav::Stay);
                }
                if ui.button(Rect::new(r.x + r.w - 170.0, r.y + r.h - 64.0, 140.0, 46.0), "OK", rgb8(90, 90, 110), 22.0) {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                }
                out
            }
            Modal::Victory => {
                let last = self.idx + 1 >= LEVELS.len();
                // Narrow screens stack the buttons in two rows, so the popup is taller.
                let narrow = self.lay.w < 700.0;
                let r = self.lay.popup(680.0, if narrow { 600.0 } else { 520.0 }, 130.0);
                panel(r);
                let title = if last { "YOU BEAT THE GAME!" } else { "Level complete!" };
                let size = fit_size(title, r.w - 40.0, 40.0, 26.0);
                text_centered(title, Rect::new(r.x, r.y + 20.0, r.w, 44.0), size, TITLE_ON_LIGHT, false);
                for i in 0..3 {
                    let pop = ((get_time() * 3.0 + i as f64).sin() * 2.0) as f32;
                    star(r.x + r.w / 2.0 - 90.0 + i as f32 * 90.0, r.y + 120.0 + pop, 34.0, i < self.stars as usize);
                }
                let used = self.program.block_count();
                let mut y = r.y + 172.0;
                text_centered(&format!("You used {used} blocks."), Rect::new(r.x, y, r.w, 26.0), 22.0, rgb8(30, 30, 30), false);
                y += 30.0;
                let extra = if self.stars < 3 {
                    format!("Can you solve it with only {} blocks? Try again for 3 stars!", self.level.par)
                } else {
                    "Perfect! That's as short as it gets.".to_string()
                };
                y += centered_paragraph(&extra, r.x + 20.0, y, r.w - 40.0, 19.0, rgb8(60, 60, 60)) + 15.0;
                draw_rectangle(r.x + 30.0, y, r.w - 60.0, 184.0, rgb8(40, 80, 50));
                shadow_text("What you learned:", r.x + 46.0, y + 12.0, 20.0, rgb8(160, 255, 170));
                let ch = self.level.chapter;
                let world_end = !last && LEVELS[self.idx + 1].chapter != ch;
                let lesson = if world_end && ch < CHECKS && !progress.checks[ch] {
                    format!("{}\n\nYou finished {}! Pass the World Check to move on.", self.level.lesson, CHAPTERS[ch].name)
                } else if world_end && !progress.unlocked(self.idx + 1) {
                    format!("{}\n\nThe {} opens once every World Check is passed.", self.level.lesson, CHAPTERS[ch + 1].name)
                } else if last {
                    format!("{}\n\nYou earned {} of {} stars. Keep going - maybe one day YOU'LL make games like this!", self.level.lesson, progress.total_stars(), LEVELS.len() * 3)
                } else {
                    self.level.lesson.to_string()
                };
                paragraph(&lesson, r.x + 46.0, y + 42.0, r.w - 92.0, 19.0, WHITE);
                let by = r.y + r.h - 68.0;
                let mut out = None;
                // At the end of a world, the World Check comes before the next level.
                let ch = self.level.chapter;
                let world_end = !last && LEVELS[self.idx + 1].chapter != ch;
                let take_check = world_end && ch < CHECKS && !progress.checks[ch];
                // Wide: [Try again] [Levels] ... [Next]. Narrow: [Next] on its own row above.
                let half = (r.w - 80.0) / 2.0;
                let next_r = if narrow {
                    Rect::new(r.x + 30.0, by - 60.0, r.w - 60.0, 48.0)
                } else {
                    Rect::new(r.x + r.w - 230.0, by, 200.0, 48.0)
                };
                if take_check {
                    if ui.button(next_r, "World Check >", rgb8(225, 175, 40), 24.0) {
                        sfx.play(Sfx::Click);
                        out = Some(Nav::Check(ch));
                    }
                } else if !last && progress.unlocked(self.idx + 1) && ui.button(next_r, "Next level >", rgb8(60, 170, 60), 24.0) {
                    sfx.play(Sfx::Click);
                    out = Some(Nav::Goto(self.idx + 1));
                }
                let try_r = if narrow { Rect::new(r.x + 30.0, by, half, 48.0) } else { Rect::new(r.x + 30.0, by, 160.0, 48.0) };
                if ui.button(try_r, "Try again", rgb8(70, 130, 180), 22.0) {
                    sfx.play(Sfx::Click);
                    self.modal = Modal::None;
                    self.reset_run();
                }
                let levels_r = if narrow {
                    Rect::new(r.x + 50.0 + half, by, half, 48.0)
                } else {
                    Rect::new(if last { r.x + r.w - 190.0 } else { r.x + 210.0 }, by, 160.0, 48.0)
                };
                if ui.button(levels_r, "Levels", rgb8(110, 110, 120), 22.0) {
                    sfx.play(Sfx::Click);
                    out = Some(Nav::Back);
                }
                out
            }
        }
    }
}

/// Draws one line of Rust with simple syntax colours (the font is monospaced).
fn draw_code_line(line: &str, x: f32, y: f32, size: f32) {
    let cw = code_width("0000000000", size) / 10.0;
    if line.trim_start().starts_with("//") {
        code_text(line, x, y, size, rgb8(120, 170, 120));
        return;
    }
    let chars: Vec<char> = line.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let start = i;
        let color;
        if chars[i].is_alphanumeric() || chars[i] == '_' {
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            let word: String = chars[start..i].iter().collect();
            color = if ["fn", "for", "in", "if", "else", "while"].contains(&word.as_str()) {
                rgb8(200, 130, 255)
            } else if word.chars().all(|c| c.is_ascii_digit()) {
                rgb8(255, 180, 90)
            } else if chars.get(i) == Some(&'(') {
                rgb8(110, 200, 255)
            } else {
                rgb8(230, 230, 230)
            };
        } else {
            i += 1;
            color = if chars[start] == '!' { rgb8(255, 120, 120) } else { rgb8(180, 180, 180) };
        }
        let tok: String = chars[start..i].iter().collect();
        if !tok.trim().is_empty() {
            code_text(&tok, x + start as f32 * cw, y, size, color);
        }
    }
}
