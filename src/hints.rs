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
//! - A program that already wins but is long gets help finding a pattern.

use crate::levels::Level;
use crate::program::{Cond, Kind, Node, Program, Step, Vm, simulate};
use crate::world::{Dir, Failure, Outcome, Tile, World};

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
    if p.main.is_empty() || p.block_count() > MAX_SEARCH_BLOCKS {
        return with_level_hint(level, diagnose(level, p, start, seed));
    }
    // A found fix ends with the exact change, so the level's general hint
    // would only be a step backwards.
    match find_problem_fix(level, p, start, seed) {
        Some((problem, fix)) => fix_ladder(&problem, &fix),
        None => with_level_hint(level, diagnose(level, p, start, seed)),
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
    Some((problem, find_fix(level, p, start, seed)?))
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
        return already_wins(level, p);
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
            let mut out = vec![Hint::new(failure_advice(f, &w), Some(id))];
            out.extend(way_to_goal(&w).map(|t| Hint::new(t, Some(id))));
            out
        }
        (Run::Ended(w), _) => {
            let mut out = vec![Hint::new(w.unfinished_message(), None)];
            out.extend(way_to_goal(&w).map(|t| Hint::new(t, None)));
            out
        }
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
            Step::Done => return Run::Ended(w),
            Step::Error(msg, id) => return Run::Stuck(msg, id),
        }
    }
}

fn wins_everywhere(level: &Level, p: &Program, start: &World, seed: u64) -> bool {
    simulate(p, start.clone()).is_ok() && failing_test_world(level, p, seed).is_none()
}

// ---------------------------------------------------------------- the fix search

#[derive(Clone, Debug, PartialEq)]
enum Fix {
    Count { id: u32, from: u32, to: u32 },
    Swap { id: u32, from: Kind, to: Kind },
    Check { id: u32, kind: Kind, from: Cond, to: Cond },
    Remove { id: u32, kind: Kind },
    Add { kind: Kind, at: Spot },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Spot {
    After(u32),
    Before(u32),
    /// Inside a container that's empty there (`true` = its ELSE part).
    Inside(u32, Kind, bool),
    EmptyFunction,
}

impl Fix {
    /// Smaller, more "almost right" changes are suggested first.
    fn rank(&self) -> u8 {
        match self {
            Fix::Count { .. } => 0,
            Fix::Check { .. } => 1,
            Fix::Swap { .. } => 2,
            Fix::Remove { .. } => 3,
            Fix::Add { .. } => 4,
        }
    }

    fn block(&self) -> Option<u32> {
        match *self {
            Fix::Count { id, .. } | Fix::Swap { id, .. } | Fix::Check { id, .. } | Fix::Remove { id, .. } => Some(id),
            Fix::Add { at: Spot::After(id) | Spot::Before(id) | Spot::Inside(id, ..), .. } => Some(id),
            Fix::Add { at: Spot::EmptyFunction, .. } => None,
        }
    }
}

/// The best single change that makes `p` win here and in every test world.
fn find_fix(level: &Level, p: &Program, start: &World, seed: u64) -> Option<Fix> {
    let mut cands = Vec::new();
    for (main, fix) in list_edits(&p.main, level, None) {
        cands.push((Program { main, func: p.func.clone() }, fix));
    }
    if level.palette.contains(&Kind::Call) {
        for (func, fix) in list_edits(&p.func, level, None) {
            cands.push((Program { main: p.main.clone(), func }, fix));
        }
    }
    let max = level.max_blocks.unwrap_or(u32::MAX);
    cands.retain(|(c, _)| c.block_count() <= max);
    cands.sort_by_key(|(_, f)| f.rank()); // stable: earlier blocks first within a rank
    cands.into_iter().find(|(c, _)| wins_everywhere(level, c, start, seed)).map(|(_, f)| f)
}

/// Every program list one edit away from `list`, with the edit that made it.
/// `parent` is the container (and whether it's the ELSE part) holding `list`.
fn list_edits(list: &[Node], level: &Level, parent: Option<(u32, Kind, bool)>) -> Vec<(Vec<Node>, Fix)> {
    let actions: Vec<Kind> = level.palette.iter().copied().filter(|k| !k.is_container()).collect();
    let mut out = Vec::new();

    // Add one simple block at each gap.
    for i in 0..=list.len() {
        let at = match (i, list.is_empty(), parent) {
            (_, true, Some((id, kind, els))) => Spot::Inside(id, kind, els),
            (_, true, None) => Spot::EmptyFunction,
            (0, false, _) => Spot::Before(list[0].id),
            _ => Spot::After(list[i - 1].id),
        };
        for &k in &actions {
            // Adding a block just before an identical one is the same as adding
            // it just after; only offer the last spot ("one more FORWARD here").
            if list.get(i).is_some_and(|n| n.kind == k) {
                continue;
            }
            let mut l = list.to_vec();
            l.insert(i, Node::new(k, Cond::PathAhead));
            out.push((l, Fix::Add { kind: k, at }));
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
        // Change a REPEAT's number.
        if n.kind == Kind::Repeat {
            for c in (1..=n.count.max(9) + 3).filter(|&c| c != n.count) {
                with(Node { count: c, ..n.clone() }, Fix::Count { id: n.id, from: n.count, to: c });
            }
        }
        // Change what an IF / WHILE checks.
        if matches!(n.kind, Kind::If | Kind::IfElse | Kind::While) {
            for &c in level.conds.iter().filter(|&&c| c != n.cond) {
                with(Node { cond: c, ..n.clone() }, Fix::Check { id: n.id, kind: n.kind, from: n.cond, to: c });
            }
        }
        // One edit inside a container.
        if n.kind.is_container() {
            for (body, fix) in list_edits(&n.body, level, Some((n.id, n.kind, false))) {
                with(Node { body, ..n.clone() }, fix);
            }
            if n.kind == Kind::IfElse {
                for (els, fix) in list_edits(&n.els, level, Some((n.id, n.kind, true))) {
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
        Fix::Count { from, to, .. } => format!(
            "Your REPEAT block (outlined in yellow) is almost right - it needs to repeat {} times than {from}.",
            if to > from { "MORE" } else { "FEWER" }
        ),
        Fix::Swap { from: Kind::TurnLeft, to: Kind::TurnRight, .. } | Fix::Swap { from: Kind::TurnRight, to: Kind::TurnLeft, .. } => {
            "The turn outlined in yellow goes the wrong way. Pretend YOU are the miner: which hand points where you need to go?".to_string()
        }
        Fix::Check { kind, .. } => {
            format!("Look at what the outlined {} block checks. Is it checking the right thing?", name(*kind))
        }
        Fix::Add { at: Spot::EmptyFunction, .. } => "Your FUNCTION is empty. What should it do?".to_string(),
        _ => "Look closely at the block outlined in yellow. The change you need is right there.".to_string(),
    };
    vec![
        Hint::new(format!("{problem}. Good news: you're only ONE change away from solving it! Can you spot it?"), None),
        Hint::new(nudge, b),
        Hint::new(exact(fix), b),
    ]
}

fn exact(fix: &Fix) -> String {
    match *fix {
        Fix::Count { from, to, .. } => format!("Change the outlined REPEAT from {from} to {to}."),
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
        Fix::Add { kind, at } => {
            let what = format!("Add {} {} block", article(kind), name(kind));
            match at {
                Spot::After(_) => format!("{what} right AFTER the outlined block."),
                Spot::Before(_) => format!("{what} right BEFORE the outlined block."),
                Spot::Inside(_, Kind::IfElse, true) => format!("{what} in the ELSE part of the outlined IF / ELSE."),
                Spot::Inside(_, c, _) => format!("{what} inside the outlined {} block.", name(c)),
                Spot::EmptyFunction => format!("{what} to your FUNCTION."),
            }
        }
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

fn failure_advice(f: Failure, w: &World) -> String {
    match f {
        Failure::Bonk(Tile::Stone | Tile::Cobble | Tile::Ore) => {
            "The outlined block walks into a block the miner could MINE. Add a MINE right before it.".into()
        }
        Failure::Bonk(_) => "The outlined block walks into a wall. Maybe a turn is missing just before it, \
             or the miner walked too far. Count the squares again!"
            .into(),
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
        "From where the miner is, {what} is {squares} away. The way there starts {way}.",
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

fn already_wins(level: &Level, p: &Program) -> Vec<Hint> {
    let used = p.block_count();
    if used <= level.par {
        return vec![Hint::new(
            "Your program already solves this level with the fewest blocks. Press RUN and watch it go!",
            None,
        )];
    }
    let goal = format!(
        "Your program works! For 3 stars, solve it with {} blocks or fewer - you're using {used}.",
        level.par
    );
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
    if level.palette.contains(&Kind::Call) {
        out.push(Hint::new(
            "Are the same steps done in different places? Put them in your FUNCTION once, then CALL it wherever you need it.",
            None,
        ));
    }
    out
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
