//! Smart hints: Bit looks at the kid's actual program and explains what to try
//! next, a little more each time Hint is pressed.
//!
//! Everything runs on the player's device with the same simulator the game
//! uses, so a hint never claims something the game wouldn't agree with:
//! - If ONE change (add, remove or swap a block, change a REPEAT number or an
//!   IF check) makes the program win in this world and every test world, Bit
//!   leads the kid to it: first a nudge, then the block, then the exact change.
//! - Otherwise Bit explains what went wrong, points at that block, and says
//!   which way the goal is from where the miner stopped.
//! - A program that already wins but is long gets help finding a pattern, or
//!   a block that ends both parts of an IF / ELSE (it can go once, after it).

use crate::levels::Level;
use crate::program::{Cond, Kind, Node, Program, Step, Vm, contains, kinds_used, simulate};
use crate::world::{Dir, Failure, Outcome, Tile, World};
use std::collections::HashSet;

/// How many other versions of a changing level a winning program must also beat.
pub const TEST_WORLDS: u64 = 80;

/// The worlds a program is tested in after winning a changing level (and
/// that a suggested fix must also beat).
pub fn test_seeds(seed: u64) -> impl Iterator<Item = u64> {
    (0..TEST_WORLDS).map(move |i| seed.wrapping_add(i.wrapping_mul(0x9E37_79B9)).wrapping_add(1))
}

/// The first test world this program fails in, with what went wrong.
pub fn failing_test_world(level: &Level, p: &Program, seed: u64) -> Option<(u64, String)> {
    if !level.is_random() {
        return None;
    }
    test_seeds(seed).find_map(|s| simulate(p, World::from_level(level, s)).err().map(|e| (s, e)))
}

/// One rung of the hint ladder: what Bit says, and the block to outline.
#[derive(Clone, Debug, PartialEq)]
pub struct Hint {
    pub text: String,
    pub block: Option<u32>,
}

impl Hint {
    fn new(text: impl Into<String>, block: Option<u32>) -> Hint {
        Hint { text: text.into(), block }
    }
}

/// Programs bigger than this skip the fix search (it tries every single edit).
const MAX_SEARCH_BLOCKS: u32 = 60;

/// The hints for this program in this world, from gentlest to most direct.
/// Never empty. Show them one per press of the Hint button.
pub fn ladder(level: &Level, p: &Program, start: &World, seed: u64) -> Vec<Hint> {
    // The level's general hint explains how to solve it, which doesn't help
    // a program that already works.
    let general = |out| {
        if !p.main.is_empty() && wins_everywhere(level, p, start, seed) { out } else { with_level_hint(level, out) }
    };
    if p.main.is_empty() || p.block_count() > MAX_SEARCH_BLOCKS {
        return general(diagnose(level, p, start, seed));
    }
    // A found fix ends with the exact change, so the level's general hint
    // would only be a step backwards.
    match find_problem_fix(level, p, start, seed) {
        Some((problem, fix)) => fix_ladder(&problem, &fix),
        None => general(diagnose(level, p, start, seed)),
    }
}

fn with_level_hint(level: &Level, mut out: Vec<Hint>) -> Vec<Hint> {
    let authored = format!("Hint: {}", level.hint);
    if !out.iter().any(|h| h.text == authored) {
        out.push(Hint::new(authored, None));
    }
    out
}

/// If the program doesn't win everywhere yet but one change would fix it:
/// what goes wrong now, and that change.
fn find_problem_fix(level: &Level, p: &Program, start: &World, seed: u64) -> Option<(String, Fix)> {
    let problem = match run(p, start.clone()) {
        Run::Won if failing_test_world(level, p, seed).is_some() => {
            "Your program works in THIS world, but not in every version of it".to_string()
        }
        Run::Won => return None,
        Run::Failed(f, ..) => problem(f),
        Run::Ended(_) => "Your program stops before the job is done".to_string(),
        Run::Stuck(..) => "Your program gets stuck".to_string(),
    };
    Some((problem, find_fix(level, p, start, seed)?.1))
}

/// Hints for a program with no single-change fix (or none needed).
fn diagnose(level: &Level, p: &Program, start: &World, seed: u64) -> Vec<Hint> {
    if p.main.is_empty() {
        if !p.func.is_empty() {
            return vec![Hint::new(
                "Your FUNCTION has blocks, but MAIN is empty. The miner only follows MAIN - \
                 add a CALL FUNCTION block to MAIN to use your function.",
                None,
            )];
        }
        let mut out = vec![Hint::new(
            "Let's start! Pretend YOU are the miner. What's the very first thing you'd do? \
             Click that block in the BLOCKS list to add it.",
            None,
        )];
        out.push(Hint::new(format!("Hint: {}", level.hint), None));
        return out;
    }

    let run = run(p, start.clone());
    let here_won = matches!(run, Run::Won);
    let elsewhere = if here_won { failing_test_world(level, p, seed) } else { None };

    if here_won && elsewhere.is_none() {
        return already_wins(level, p, start, seed);
    }

    match (run, elsewhere) {
        (_, Some((_, err))) => vec![
            Hint::new(
                format!(
                    "Your program works in THIS world, but not in every version of it. The world \
                     CHANGES each time you press RUN or the dice. In another version: \"{err}\""
                ),
                None,
            ),
            Hint::new(changing_world_advice(level), None),
        ],
        (Run::Failed(f, id, w), _) => {
            let mut out = vec![Hint::new(failure_advice(level, p, id, f, &w), Some(id))];
            out.extend(way_to_goal(&w).map(|t| Hint::new(t, Some(id))));
            out
        }
        // The game already says what's missing when a run ends, so don't
        // repeat it: start with where to go.
        (Run::Ended(w), _) => vec![Hint::new(way_to_goal(&w).unwrap_or_else(|| w.unfinished_message()), None)],
        (Run::Stuck(msg, id), _) => vec![Hint::new(
            format!("{msg} Press STEP to run one block at a time and watch where it goes wrong."),
            id,
        )],
        (Run::Won, None) => unreachable!(),
    }
}

// ---------------------------------------------------------------- running

enum Run {
    Won,
    /// What went wrong, the block that did it, and the world just before.
    Failed(Failure, u32, World),
    /// The program ended without winning; the world as it was left.
    Ended(World),
    Stuck(String, Option<u32>),
}

fn run(p: &Program, mut w: World) -> Run {
    let mut vm = Vm::new(p);
    loop {
        match vm.next(&w) {
            Step::Act(a, id) => match w.apply(a) {
                Outcome::Ok => {}
                Outcome::Won => return Run::Won,
                // `apply` checks before changing anything, so `w` is the world
                // the failing action saw.
                Outcome::Fail(f) => return Run::Failed(f, id, w),
            },
            Step::Var(_) => {}
            Step::Done => return Run::Ended(w),
            Step::Error(msg, id) => return Run::Stuck(msg, id),
        }
    }
}

fn wins_everywhere(level: &Level, p: &Program, start: &World, seed: u64) -> bool {
    simulate(p, start.clone()).is_ok() && failing_test_world(level, p, seed).is_none()
}

/// This world, plus every test world of a changing level.
fn all_worlds(level: &Level, start: &World, seed: u64) -> Vec<World> {
    let mut out = vec![start.clone()];
    if level.is_random() {
        out.extend(test_seeds(seed).map(|s| World::from_level(level, s)));
    }
    out
}

/// How a program wins across some worlds.
struct WinShape {
    /// Per world: the steps it would still take after winning if the game
    /// didn't stop it (None: it never wins, or would fail or get stuck after).
    after: Vec<Option<u32>>,
    /// Simple blocks that never do anything before a win, in any world.
    unused: Vec<u32>,
}

fn win_shape(p: &Program, worlds: &[World]) -> WinShape {
    let mut ran = HashSet::new();
    let after = worlds.iter().map(|w| steps_after_win(p, w.clone(), &mut ran)).collect();
    let mut blocks = Vec::new();
    simple_blocks(&p.main, &mut blocks);
    let mut used = Vec::new();
    kinds_used(&p.main, &mut used);
    if used.contains(&Kind::Call) {
        simple_blocks(&p.func, &mut blocks);
    }
    WinShape { after, unused: blocks.into_iter().filter(|id| !ran.contains(id)).collect() }
}

/// The level's own solution's shape, to compare fixes with. (Some solutions
/// win partway through their last loop, and that's fine.)
fn solution_shape(level: &Level, worlds: &[World]) -> WinShape {
    match Program::deserialize(level.solution) {
        Some(s) => win_shape(&s, worlds),
        None => WinShape { after: vec![None; worlds.len()], unused: Vec::new() },
    }
}

/// Runs past the win, recording in `ran` the blocks that acted before it.
fn steps_after_win(p: &Program, mut w: World, ran: &mut HashSet<u32>) -> Option<u32> {
    let mut vm = Vm::new(p);
    let mut after: Option<u32> = None;
    loop {
        match vm.next(&w) {
            Step::Act(a, id) => {
                match after {
                    Some(ref mut n) => *n += 1,
                    None => {
                        ran.insert(id);
                    }
                }
                match w.apply(a) {
                    Outcome::Won if after.is_none() => after = Some(0),
                    Outcome::Ok | Outcome::Won => {}
                    Outcome::Fail(_) => return None,
                }
            }
            Step::Var(id) => {
                if after.is_none() {
                    ran.insert(id);
                }
            }
            Step::Done => return after,
            Step::Error(..) => return None,
        }
    }
}

/// Blocks that move or act (not containers or CALL FUNCTION), in program order.
fn simple_blocks(list: &[Node], out: &mut Vec<u32>) {
    for n in list {
        if !n.kind.is_container() && n.kind != Kind::Call {
            out.push(n.id);
        }
        simple_blocks(&n.body, out);
        simple_blocks(&n.els, out);
    }
}

// ---------------------------------------------------------------- the fix search

#[derive(Clone, Debug, PartialEq)]
enum Fix {
    /// Change the number on a REPEAT, SET COUNT or ADD TO COUNT block.
    Count { id: u32, kind: Kind, from: u32, to: u32 },
    Swap { id: u32, from: Kind, to: Kind },
    Check { id: u32, kind: Kind, from: Cond, to: Cond },
    Remove { id: u32, kind: Kind },
    /// Add a block (`number` is what SET COUNT / ADD TO COUNT should hold).
    Add { kind: Kind, number: u32, at: Spot },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Spot {
    /// After this block (whose kind is given, so a REPEAT can say "not inside it").
    After(u32, Kind),
    Before(u32),
    /// Inside a container that's empty there (`true` = its ELSE part).
    Inside(u32, Kind, bool),
    EmptyFunction,
}

impl Fix {
    /// Smaller, more "almost right" changes are suggested first.
    /// Filling an empty part comes first: a kid who left it empty clearly
    /// meant to put something there.
    fn rank(&self) -> u8 {
        match self {
            Fix::Add { at: Spot::Inside(..) | Spot::EmptyFunction, .. } => 0,
            Fix::Count { .. } => 1,
            Fix::Check { .. } => 2,
            Fix::Swap { .. } => 3,
            Fix::Remove { .. } => 4,
            Fix::Add { .. } => 5,
        }
    }

    fn block(&self) -> Option<u32> {
        match *self {
            Fix::Count { id, .. } | Fix::Swap { id, .. } | Fix::Check { id, .. } | Fix::Remove { id, .. } => Some(id),
            Fix::Add { at: Spot::After(id, _) | Spot::Before(id) | Spot::Inside(id, ..), .. } => Some(id),
            Fix::Add { at: Spot::EmptyFunction, .. } => None,
        }
    }
}

/// The best single change that makes `p` win here and in every test world,
/// and the program it makes.
fn find_fix(level: &Level, p: &Program, start: &World, seed: u64) -> Option<(Program, Fix)> {
    let mut cands = Vec::new();
    // Calling an empty function does nothing, so it's never the fix. And a
    // function that calls itself is never what a kid meant either.
    for (main, fix) in list_edits(&p.main, level, None, !p.func.is_empty(), &[]) {
        cands.push((Program { main, func: p.func.clone() }, fix));
    }
    if level.palette.contains(&Kind::Call) {
        for (func, fix) in list_edits(&p.func, level, None, false, &[]) {
            cands.push((Program { main: p.main.clone(), func }, fix));
        }
    }
    let max = level.max_blocks.unwrap_or(u32::MAX);
    cands.retain(|(c, _)| c.block_count() <= max);
    cands.sort_by_key(|(_, f)| f.rank()); // stable: earlier blocks first within a rank
    let worlds = all_worlds(level, start, seed);
    let solution = solution_shape(level, &worlds);
    cands
        .into_iter()
        .find(|(c, _)| wins_everywhere(level, c, start, seed) && wins_cleanly(&win_shape(c, &worlds), &solution))
}

/// A fix must win the way the level means to be won, not by luck: every
/// block does something, and it doesn't keep going after the win any longer
/// than the level's own solution does. (The game stops the moment the miner
/// wins, so "REPEAT 3" can "fix" a path that needed one more step, and swapping
/// an extra block for FORWARD can "fix" a hallway by walking twice as far.)
fn wins_cleanly(shape: &WinShape, solution: &WinShape) -> bool {
    shape.unused.is_empty() && shape.after.iter().zip(&solution.after).all(|(&a, &s)| no_longer(a, s))
}

/// Whether `a` steps after the win is no more than `b` (None: it fails or
/// never ends after the win, which is worst).
fn no_longer(a: Option<u32>, b: Option<u32>) -> bool {
    match (a, b) {
        (_, None) => true,
        (None, Some(_)) => false,
        (Some(a), Some(b)) => a <= b,
    }
}

/// Every program list one edit away from `list`, with the edit that made it.
/// `parent` is the container (and whether it's the ELSE part) holding `list`.
/// `call_ok`: whether adding a CALL FUNCTION block could make sense here.
/// `outer`: what the IF / WHILE blocks around `list` check.
fn list_edits(
    list: &[Node],
    level: &Level,
    parent: Option<(u32, Kind, bool)>,
    call_ok: bool,
    outer: &[Cond],
) -> Vec<(Vec<Node>, Fix)> {
    let actions: Vec<Kind> =
        level.palette.iter().copied().filter(|&k| !k.is_container() && (call_ok || k != Kind::Call)).collect();
    let mut out = Vec::new();

    // Add one simple block at each gap.
    for i in 0..=list.len() {
        let at = match (i, list.is_empty(), parent) {
            (_, true, Some((id, kind, els))) => Spot::Inside(id, kind, els),
            (_, true, None) => Spot::EmptyFunction,
            (0, false, _) => Spot::Before(list[0].id),
            _ => Spot::After(list[i - 1].id, list[i - 1].kind),
        };
        for &k in &actions {
            // Adding a block just before an identical one is the same as adding
            // it just after; only offer the last spot ("one more FORWARD here").
            if list.get(i).is_some_and(|n| n.kind == k) {
                continue;
            }
            for number in new_numbers(k) {
                let mut node = Node::new(k, Cond::PathAhead);
                node.count = number;
                let mut l = list.to_vec();
                l.insert(i, node);
                out.push((l, Fix::Add { kind: k, number, at }));
            }
        }
    }

    for (i, n) in list.iter().enumerate() {
        let mut with = |node: Node, fix: Fix| {
            let mut l = list.to_vec();
            l[i] = node;
            out.push((l, fix));
        };
        // Swap a simple block for another.
        if !n.kind.is_container() {
            for &k in actions.iter().filter(|&&k| k != n.kind) {
                with(Node { kind: k, ..n.clone() }, Fix::Swap { id: n.id, from: n.kind, to: k });
            }
        }
        // Change a REPEAT's (or a SET / ADD's) number.
        if let Some((lo, hi)) = n.kind.number_range() {
            for c in (lo..=hi.min(n.count.max(9) + 3)).filter(|&c| c != n.count) {
                with(Node { count: c, ..n.clone() }, Fix::Count { id: n.id, kind: n.kind, from: n.count, to: c });
            }
        }
        // Change what an IF / WHILE checks. Not to something a block around it
        // already checked: inside "IF path to right", checking it again is
        // always true, and in its ELSE always false, so it only hides code.
        // And not when part of it is empty: that part is missing blocks, and a
        // different check would only steer around the gap.
        let checks = matches!(n.kind, Kind::If | Kind::IfElse | Kind::While);
        let empty_part = n.body.is_empty() || (n.kind == Kind::IfElse && n.els.is_empty());
        if checks && !empty_part {
            for &c in level.conds.iter().filter(|&&c| c != n.cond && !outer.contains(&c)) {
                with(Node { cond: c, ..n.clone() }, Fix::Check { id: n.id, kind: n.kind, from: n.cond, to: c });
            }
        }
        // One edit inside a container.
        if n.kind.is_container() {
            let mut inner = outer.to_vec();
            if checks {
                inner.push(n.cond);
            }
            for (body, fix) in list_edits(&n.body, level, Some((n.id, n.kind, false)), call_ok, &inner) {
                with(Node { body, ..n.clone() }, fix);
            }
            if n.kind == Kind::IfElse {
                for (els, fix) in list_edits(&n.els, level, Some((n.id, n.kind, true)), call_ok, &inner) {
                    with(Node { els, ..n.clone() }, fix);
                }
            }
        }
        // Remove it.
        let mut l = list.to_vec();
        l.remove(i);
        out.push((l, Fix::Remove { id: n.id, kind: n.kind }));
    }
    out
}

fn fix_ladder(problem: &str, fix: &Fix) -> Vec<Hint> {
    let b = fix.block();
    let nudge = match fix {
        Fix::Count { kind: Kind::Repeat, from, to, .. } => format!(
            "Your REPEAT block (outlined in yellow) is almost right - it needs to repeat {} times than {from}.",
            if to > from { "MORE" } else { "FEWER" }
        ),
        Fix::Count { kind, from, to, .. } => format!(
            "Your {} block (outlined in yellow) is almost right - its number needs to be {} than {from}.",
            name(*kind),
            if to > from { "BIGGER" } else { "SMALLER" }
        ),
        Fix::Swap { from: Kind::TurnLeft, to: Kind::TurnRight, .. } | Fix::Swap { from: Kind::TurnRight, to: Kind::TurnLeft, .. } => {
            "The turn outlined in yellow goes the wrong way. Pretend YOU are the miner: which hand points where you need to go?".to_string()
        }
        Fix::Check { kind, .. } => {
            format!("Look at what the outlined {} block checks. Is it checking the right thing?", name(*kind))
        }
        Fix::Swap { from, .. } => {
            format!("The outlined {} is the wrong block for that spot. Which block belongs there instead?", name(*from))
        }
        Fix::Remove { .. } => "One block is extra, and it's outlined in yellow. What happens if the miner skips it?".to_string(),
        // The outline is on a neighbour: say the change goes next to it, not on it.
        Fix::Add { at: Spot::After(..), .. } => {
            "Something is missing right AFTER the outlined block. What should the miner do next?".to_string()
        }
        Fix::Add { at: Spot::Before(_), .. } => {
            "Something is missing right BEFORE the outlined block. What should the miner do first?".to_string()
        }
        Fix::Add { at: Spot::Inside(_, Kind::IfElse, true), .. } => {
            "The ELSE part of the outlined IF / ELSE is empty. What should happen there?".to_string()
        }
        Fix::Add { at: Spot::Inside(_, k, _), .. } => format!("The outlined {} block is empty. What should go inside it?", name(*k)),
        Fix::Add { at: Spot::EmptyFunction, .. } => "Your FUNCTION is empty. What should it do?".to_string(),
    };
    vec![
        Hint::new(format!("{problem}. Good news: you're only ONE change away from solving it! Can you spot it?"), None),
        Hint::new(nudge, b),
        Hint::new(exact(fix), b),
    ]
}

fn exact(fix: &Fix) -> String {
    match *fix {
        Fix::Count { kind: Kind::Repeat, from, to, .. } => format!("Change the outlined REPEAT from {from} to {to}."),
        Fix::Count { kind, from, to, .. } => format!("Change the number in the outlined {} block from {from} to {to}.", name(kind)),
        Fix::Swap { from, to, .. } => format!("Change the outlined {} into {}.", name(from), name(to)),
        Fix::Check { kind, from, to, .. } => format!(
            "Change the outlined {} to check \"{}\" instead of \"{}\".",
            name(kind),
            to.label(),
            from.label()
        ),
        Fix::Remove { kind, .. } if kind.is_container() => {
            format!("Remove the outlined {} block (and the blocks inside it).", name(kind))
        }
        Fix::Remove { .. } => "Remove the outlined block. It doesn't belong there.".to_string(),
        Fix::Add { kind, number, at } => {
            let what = if kind.number_range().is_some() {
                let mut n = Node::new(kind, Cond::PathAhead);
                n.count = number;
                format!("Add a {} block", n.label().to_uppercase())
            } else {
                format!("Add {} {} block", article(kind), name(kind))
            };
            match at {
                Spot::After(_, k) if k.is_container() => {
                    format!("{what} right AFTER the outlined {} block - below it, not inside it.", name(k))
                }
                Spot::After(..) => format!("{what} right AFTER the outlined block."),
                Spot::Before(_) => format!("{what} right BEFORE the outlined block."),
                Spot::Inside(_, Kind::IfElse, true) => format!("{what} in the ELSE part of the outlined IF / ELSE."),
                Spot::Inside(_, c, _) => format!("{what} inside the outlined {} block.", name(c)),
                Spot::EmptyFunction => format!("{what} to your FUNCTION."),
            }
        }
    }
}

/// The numbers to try on a new block: every useful SET value, the small ADDs,
/// or just the block's usual number.
fn new_numbers(k: Kind) -> Vec<u32> {
    match k {
        Kind::Set => (0..=12).collect(),
        Kind::Add => (1..=3).collect(),
        _ => vec![Node::new(k, Cond::PathAhead).count],
    }
}

// ---------------------------------------------------------------- explanations

fn name(k: Kind) -> String {
    k.palette_label().to_uppercase()
}

fn article(k: Kind) -> &'static str {
    if matches!(k, Kind::If | Kind::IfElse) { "an" } else { "a" }
}

fn problem(f: Failure) -> String {
    match f {
        Failure::Bonk(Tile::Stone | Tile::Cobble) => "The miner bumps into stone",
        Failure::Bonk(Tile::Ore) => "The miner bumps into diamond ore",
        Failure::Bonk(_) => "The miner walks into a wall",
        Failure::Hazard(Tile::Lava) => "The miner steps into lava",
        Failure::Hazard(_) => "The miner splashes into water",
        Failure::NothingToMine => "A MINE block swings at thin air",
        Failure::CantMine(_) => "A MINE block hits something it can't break",
        Failure::NoBlocks => "The miner tries to place a block with an empty backpack",
        Failure::CantPlace(_) => "The miner tries to place a block where it can't go",
    }
    .to_string()
}

fn failure_advice(level: &Level, p: &Program, id: u32, f: Failure, w: &World) -> String {
    // In a changing level the stone or lava might not be there next time, so
    // a plain MINE or PLACE would fail in other versions: suggest checking
    // first, unless the failing block is already inside that check.
    let check = |c: Cond| level.is_random() && level.conds.contains(&c) && !checked_before(&p.main, id, c) && !checked_before(&p.func, id, c);
    match f {
        Failure::Bonk(Tile::Stone | Tile::Cobble) if check(Cond::StoneAhead) => {
            "The outlined block walks into stone. In this level the stone isn't always there, so put a MINE \
             inside an IF \"stone ahead\" block, right before this step."
                .into()
        }
        Failure::Bonk(Tile::Ore) if check(Cond::DiamondAhead) => {
            "The outlined block walks into diamond ore. In this level it isn't always there, so put a MINE \
             inside an IF \"diamond ahead\" block, right before this step."
                .into()
        }
        Failure::Bonk(Tile::Stone | Tile::Cobble | Tile::Ore) => {
            "The outlined block walks into a block the miner could MINE. Add a MINE right before it.".into()
        }
        Failure::Bonk(_) => "The outlined block walks into a wall. Check the turns before it: one might be \
             missing, extra, or going the wrong way. Or the miner walked too far - count the squares again!"
            .into(),
        Failure::Hazard(t) if check(if t == Tile::Lava { Cond::LavaAhead } else { Cond::WaterAhead }) => {
            let what = if t == Tile::Lava { "lava" } else { "water" };
            let fill = if w.blocks > 0 { "" } else { " Your backpack is empty, so MINE some stone earlier, too." };
            format!(
                "The outlined block steps into {what}! In this level it isn't always there, so put a PLACE BLOCK \
                 inside an IF \"{what} ahead\" block, right before this step.{fill}"
            )
        }
        Failure::Hazard(t) => {
            let what = if t == Tile::Lava { "lava" } else { "water" };
            if w.blocks > 0 {
                format!("The outlined block steps into {what}! Your backpack has blocks - PLACE one on the {what} first to make a bridge.")
            } else {
                format!("The outlined block steps into {what}! You need a bridge, but your backpack is empty. MINE some stone earlier to collect blocks.")
            }
        }
        Failure::NothingToMine => {
            "The outlined MINE has nothing in front of it. Maybe the miner isn't facing the stone yet, or this MINE isn't needed.".into()
        }
        Failure::CantMine(_) => {
            "The outlined MINE is pointing at something unbreakable. Is the miner facing the right way?".into()
        }
        Failure::NoBlocks => {
            "The outlined PLACE BLOCK has nothing to place. MINE some stone first to fill your backpack.".into()
        }
        Failure::CantPlace(_) => "The outlined PLACE BLOCK can't go there. Blocks go on empty ground, water or lava.".into(),
    }
}

/// Whether block `id` only runs when `c` is true: it's inside an IF, IF / ELSE
/// (not its ELSE part) or WHILE that checks `c`.
fn checked_before(list: &[Node], id: u32, c: Cond) -> bool {
    list.iter().any(|n| {
        let checks = matches!(n.kind, Kind::If | Kind::IfElse | Kind::While) && n.cond == c;
        (checks && contains(&n.body, id)) || checked_before(&n.body, id, c) || checked_before(&n.els, id, c)
    })
}

fn changing_world_advice(level: &Level) -> String {
    let checks: Vec<String> = level.conds.iter().map(|c| format!("\"{}\"", c.label())).collect();
    if checks.is_empty() {
        "Use blocks that CHECK the world, like IF and WHILE, so your program can decide what to do.".into()
    } else {
        format!(
            "Instead of planning every step, let your program LOOK first. Use IF or WHILE with a check like {}.",
            checks.join(" or ")
        )
    }
}

/// Which way the next goal is from where the miner is, e.g. "The portal is 3
/// squares away. The way there starts to the miner's LEFT."
fn way_to_goal(w: &World) -> Option<String> {
    let targets: Vec<(i32, i32)> = if !w.collected_all() {
        cells(w).filter(|&c| matches!(w.tile(c), Tile::Ore | Tile::Emerald)).collect()
    } else if w.has_portal {
        cells(w).filter(|&c| w.tile(c) == Tile::Portal).collect()
    } else {
        return None;
    };
    let (dist, first, goal) = shortest_path(w, &targets)?;
    let what = match w.tile(goal) {
        Tile::Ore => "the next diamond",
        Tile::Emerald => "the next emerald",
        _ => "the portal",
    };
    let way = if first == w.dir {
        "straight AHEAD"
    } else if first == w.dir.left() {
        "to the miner's LEFT"
    } else if first == w.dir.right() {
        "to the miner's RIGHT"
    } else {
        "BEHIND the miner"
    };
    let squares = if dist == 1 { "1 square".to_string() } else { format!("{dist} squares") };
    Some(format!(
        "From where the miner stops, {what} is {squares} away. The way there starts {way}.",
    ))
}

fn cells(w: &World) -> impl Iterator<Item = (i32, i32)> + '_ {
    (0..w.h).flat_map(move |y| (0..w.w).map(move |x| (x, y)))
}

/// Breadth-first search from the miner to the nearest target, through squares
/// the miner can walk on, mine or bridge. Returns (distance, first direction, target).
fn shortest_path(w: &World, targets: &[(i32, i32)]) -> Option<(usize, Dir, (i32, i32))> {
    let passable = |t: Tile| t.walkable() || matches!(t, Tile::Stone | Tile::Cobble | Tile::Ore | Tile::Water | Tile::Lava);
    let idx = |(x, y): (i32, i32)| (y * w.w + x) as usize;
    let mut first: Vec<Option<Dir>> = vec![None; (w.w * w.h) as usize];
    let mut dist = vec![usize::MAX; (w.w * w.h) as usize];
    let mut queue = std::collections::VecDeque::from([w.pos]);
    dist[idx(w.pos)] = 0;
    while let Some(c) = queue.pop_front() {
        if targets.contains(&c) && c != w.pos {
            return Some((dist[idx(c)], first[idx(c)]?, c));
        }
        for d in [w.dir, w.dir.left(), w.dir.right(), w.dir.left().left()] {
            let (dx, dy) = d.delta();
            let n = (c.0 + dx, c.1 + dy);
            if n.0 < 0 || n.1 < 0 || n.0 >= w.w || n.1 >= w.h || !passable(w.tile(n)) || dist[idx(n)] != usize::MAX {
                continue;
            }
            dist[idx(n)] = dist[idx(c)] + 1;
            first[idx(n)] = if c == w.pos { Some(d) } else { first[idx(c)] };
            queue.push_back(n);
        }
    }
    None
}

// ---------------------------------------------------------------- already winning

fn already_wins(level: &Level, p: &Program, start: &World, seed: u64) -> Vec<Hint> {
    // The game stops the moment the miner wins, so blocks after that never
    // run, and a REPEAT can go round more times than it needs to.
    let worlds = all_worlds(level, start, seed);
    let shape = win_shape(p, &worlds);
    if let Some(&first) = shape.unused.first() {
        let some = if shape.unused.len() == 1 { "one of your blocks never runs" } else { "some of your blocks never run" };
        return vec![
            Hint::new(format!("Your program works! But {some}, so it's using more blocks than it needs. Can you find it?"), None),
            Hint::new(
                "The outlined block never runs: the miner has already won (or always skips it) by then. Remove it!",
                Some(first),
            ),
        ];
    }
    let used = p.block_count();
    let goal = format!(
        "Your program works! For 3 stars, solve it with {} blocks or fewer - you're using {used}.",
        level.par
    );
    // Before the REPEAT numbers: taking the doubled block out is what saves
    // blocks, while ending right at the goal can cost one (a FORWARD after the
    // REPEAT).
    if used > level.par {
        if let Some(n) = [&p.main, &p.func].into_iter().find_map(|l| doubled_end(l)) {
            return doubled_end_hints(&goal, n);
        }
    }
    let solution = solution_shape(level, &worlds);
    if !wins_cleanly(&shape, &solution) {
        let mut used = Vec::new();
        kinds_used(&p.main, &mut used);
        kinds_used(&p.func, &mut used);
        let repeat = if used.contains(&Kind::Repeat) { " Check your REPEAT numbers." } else { "" };
        return vec![Hint::new(
            format!(
                "Your program works, but only because the game stops it when the miner wins - it would keep going \
                 after that. Can you make it end right at the goal?{repeat}"
            ),
            None,
        )];
    }
    if used <= level.par {
        return vec![Hint::new(
            "Your program already solves this level with the fewest blocks. Press RUN and watch it go!",
            None,
        )];
    }
    if level.palette.contains(&Kind::Repeat) {
        if let Some(r) = [&p.main, &p.func].into_iter().filter_map(|l| repeated_run(l)).max_by_key(|r| r.saves) {
            let (same, those) = if r.len == 1 {
                ("block happens".to_string(), "that block".to_string())
            } else {
                (format!("{} blocks happen", r.len), format!("those {} blocks", r.len))
            };
            return vec![
                Hint::new(format!("{goal} Look for a pattern that happens again and again."), None),
                Hint::new(
                    format!("Starting at the outlined block, the same {same} {} times in a row.", r.times),
                    Some(r.first),
                ),
                Hint::new(
                    format!("Put {those} inside a REPEAT {} block. That saves {} blocks!", r.times, r.saves),
                    Some(r.first),
                ),
            ];
        }
    }
    let mut out = vec![Hint::new(goal, None)];
    if level.palette.contains(&Kind::Call) && repeats_steps(p) {
        out.push(Hint::new(
            "Some of the same steps are done in different places. Put them in your FUNCTION once, then CALL it wherever you need it.",
            None,
        ));
    }
    out
}

/// Whether the same 2 or more blocks in a row appear in two places (that
/// don't overlap), anywhere in the program: a job for a FUNCTION.
fn repeats_steps(p: &Program) -> bool {
    fn lists<'a>(list: &'a [Node], out: &mut Vec<&'a [Node]>) {
        out.push(list);
        for n in list {
            lists(&n.body, out);
            lists(&n.els, out);
        }
    }
    let mut all = Vec::new();
    lists(&p.main, &mut all);
    lists(&p.func, &mut all);
    let mut seen: std::collections::HashMap<String, Vec<(usize, usize)>> = std::collections::HashMap::new();
    for (li, list) in all.iter().enumerate() {
        let keys: Vec<String> = list.iter().map(key).collect();
        for len in 2..=6 {
            for start in 0..list.len().saturating_sub(len - 1) {
                let at = seen.entry(keys[start..start + len].concat()).or_default();
                if at.iter().any(|&(l, s)| l != li || s + len <= start) {
                    return true;
                }
                at.push((li, start));
            }
        }
    }
    false
}

/// A block (with everything inside it) as text, for comparing blocks.
fn key(n: &Node) -> String {
    Program { main: vec![n.clone()], func: Vec::new() }.serialize()
}

/// The first IF / ELSE whose two parts end with the same block. That block
/// runs either way, so it can go once, right after the IF / ELSE, and the
/// program does exactly the same.
fn doubled_end(list: &[Node]) -> Option<&Node> {
    list.iter().find_map(|n| {
        let doubled = n.kind == Kind::IfElse
            && matches!((n.body.last(), n.els.last()), (Some(a), Some(b)) if key(a) == key(b));
        if doubled { Some(n) } else { doubled_end(&n.body).or_else(|| doubled_end(&n.els)) }
    })
}

/// Uses the TOP and BOTTOM parts the IF / ELSE intro teaches. Wording and
/// placement picked with TypeSafe (see docs/DESIGN.md).
fn doubled_end_hints(goal: &str, if_else: &Node) -> Vec<Hint> {
    let last = if_else.body.last().expect("doubled_end found a block in both parts");
    let what = last.label().to_uppercase();
    let saves = Program { main: vec![last.clone()], func: Vec::new() }.block_count();
    let blocks = if saves == 1 { "1 block".to_string() } else { format!("{saves} blocks") };
    vec![
        Hint::new(format!("{goal} Look inside your IF / ELSE blocks. Is the same block at the end of BOTH parts?"), None),
        Hint::new(
            format!("The TOP part and the BOTTOM part of the outlined IF / ELSE both end with {what}. So {what} happens either way!"),
            Some(if_else.id),
        ),
        Hint::new(
            format!(
                "Take {what} out of both parts, and put it once right AFTER the outlined IF / ELSE - below it, not \
                 inside it. That saves {blocks}!"
            ),
            Some(if_else.id),
        ),
    ]
}

struct RepeatedRun {
    first: u32,
    len: usize,
    times: usize,
    saves: usize,
}

/// The run of identical blocks a REPEAT would shorten the most, if any.
fn repeated_run(list: &[Node]) -> Option<RepeatedRun> {
    let keys: Vec<String> = list
        .iter()
        .map(|n| Program { main: vec![n.clone()], func: Vec::new() }.serialize())
        .collect();
    let mut best: Option<RepeatedRun> = None;
    for start in 0..list.len() {
        for len in 1..=6.min(list.len() - start) {
            let mut times = 1;
            while start + (times + 1) * len <= list.len()
                && keys[start + times * len..start + (times + 1) * len] == keys[start..start + len]
            {
                times += 1;
            }
            let size: usize = list[start..start + len].iter().map(|n| Program { main: vec![n.clone()], func: Vec::new() }.block_count() as usize).sum();
            // A REPEAT holding the blocks once replaces all the copies.
            let saves = (size * times).saturating_sub(size + 1);
            if times >= 2 && saves > 0 && best.as_ref().is_none_or(|b| saves > b.saves) {
                best = Some(RepeatedRun { first: list[start].id, len, times, saves });
            }
        }
    }
    // Look inside containers too.
    for n in list {
        for r in [repeated_run(&n.body), repeated_run(&n.els)].into_iter().flatten() {
            if best.as_ref().is_none_or(|b| r.saves > b.saves) {
                best = Some(r);
            }
        }
    }
    best
}
