//! Code blocks, and the little virtual machine that runs them one step at a time.

use crate::world::{Outcome, World};
use std::sync::atomic::{AtomicU32, Ordering};

static NEXT_ID: AtomicU32 = AtomicU32::new(1);

fn next_id() -> u32 {
    NEXT_ID.fetch_add(1, Ordering::Relaxed)
}

/// Programs that take more actions than this are assumed to be stuck.
pub const MAX_ACTIONS: u32 = 600;
const MAX_CALL_DEPTH: usize = 100;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Kind {
    Forward,
    TurnLeft,
    TurnRight,
    Mine,
    Place,
    Repeat,
    If,
    IfElse,
    While,
    Call,
}

impl Kind {
    pub fn is_container(self) -> bool {
        matches!(self, Kind::Repeat | Kind::If | Kind::IfElse | Kind::While)
    }

    pub fn palette_label(self) -> &'static str {
        match self {
            Kind::Forward => "forward",
            Kind::TurnLeft => "turn left",
            Kind::TurnRight => "turn right",
            Kind::Mine => "mine",
            Kind::Place => "place block",
            Kind::Repeat => "repeat",
            Kind::If => "if",
            Kind::IfElse => "if / else",
            Kind::While => "while",
            Kind::Call => "call function",
        }
    }

    fn action(self) -> Option<Action> {
        Some(match self {
            Kind::Forward => Action::Forward,
            Kind::TurnLeft => Action::TurnLeft,
            Kind::TurnRight => Action::TurnRight,
            Kind::Mine => Action::Mine,
            Kind::Place => Action::Place,
            _ => return None,
        })
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Cond {
    PathAhead,
    PathLeft,
    PathRight,
    StoneAhead,
    DiamondAhead,
    LavaAhead,
    WaterAhead,
    NotAtPortal,
}

pub const ALL_CONDS: [Cond; 8] = [
    Cond::PathAhead,
    Cond::PathLeft,
    Cond::PathRight,
    Cond::StoneAhead,
    Cond::DiamondAhead,
    Cond::LavaAhead,
    Cond::WaterAhead,
    Cond::NotAtPortal,
];

impl Cond {
    pub fn label(self) -> &'static str {
        match self {
            Cond::PathAhead => "path ahead",
            Cond::PathLeft => "path to left",
            Cond::PathRight => "path to right",
            Cond::StoneAhead => "stone ahead",
            Cond::DiamondAhead => "diamond ahead",
            Cond::LavaAhead => "lava ahead",
            Cond::WaterAhead => "water ahead",
            Cond::NotAtPortal => "not at portal",
        }
    }

    pub fn code(self) -> &'static str {
        match self {
            Cond::PathAhead => "path_ahead()",
            Cond::PathLeft => "path_left()",
            Cond::PathRight => "path_right()",
            Cond::StoneAhead => "stone_ahead()",
            Cond::DiamondAhead => "diamond_ahead()",
            Cond::LavaAhead => "lava_ahead()",
            Cond::WaterAhead => "water_ahead()",
            Cond::NotAtPortal => "!at_portal()",
        }
    }

    fn index(self) -> usize {
        ALL_CONDS.iter().position(|&c| c == self).unwrap()
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Action {
    Forward,
    TurnLeft,
    TurnRight,
    Mine,
    Place,
}

/// One code block. Containers (repeat/if/while) hold more blocks in `body`;
/// `els` is only used by if/else.
#[derive(Clone, Debug)]
pub struct Node {
    pub id: u32,
    pub kind: Kind,
    pub count: u32,
    pub cond: Cond,
    pub body: Vec<Node>,
    pub els: Vec<Node>,
}

impl Node {
    pub fn new(kind: Kind, cond: Cond) -> Node {
        Node { id: next_id(), kind, count: 3, cond, body: Vec::new(), els: Vec::new() }
    }
}

#[derive(Clone, Debug, Default)]
pub struct Program {
    pub main: Vec<Node>,
    pub func: Vec<Node>,
}

fn count(list: &[Node]) -> u32 {
    list.iter().map(|n| 1 + count(&n.body) + count(&n.els)).sum()
}

pub fn contains(list: &[Node], id: u32) -> bool {
    list.iter().any(|n| n.id == id || contains(&n.body, id) || contains(&n.els, id))
}

pub fn kinds_used(list: &[Node], out: &mut Vec<Kind>) {
    for n in list {
        out.push(n.kind);
        kinds_used(&n.body, out);
        kinds_used(&n.els, out);
    }
}

impl Program {
    pub fn block_count(&self) -> u32 {
        count(&self.main) + count(&self.func)
    }

    pub fn is_empty(&self) -> bool {
        self.main.is_empty() && self.func.is_empty()
    }

    /// What this program would look like written in real Rust.
    pub fn to_rust(&self) -> Vec<String> {
        let mut out = vec!["fn main() {".to_string()];
        if self.main.is_empty() {
            out.push("    // your code goes here!".to_string());
        }
        gen_rust(&self.main, 1, &mut out);
        out.push("}".to_string());
        let mut used = Vec::new();
        kinds_used(&self.main, &mut used);
        if !self.func.is_empty() || used.contains(&Kind::Call) {
            out.push(String::new());
            out.push("fn my_function() {".to_string());
            gen_rust(&self.func, 1, &mut out);
            out.push("}".to_string());
        }
        out
    }

    /// Save format, e.g. `F [R3 M F ] [E3 M | P ] / L R`
    pub fn serialize(&self) -> String {
        let mut s = String::new();
        ser(&self.main, &mut s);
        s.push('/');
        ser(&self.func, &mut s);
        s
    }

    pub fn deserialize(s: &str) -> Option<Program> {
        let (main, func) = s.split_once('/').unwrap_or((s, ""));
        Some(Program { main: parse_list(main)?, func: parse_list(func)? })
    }
}

fn gen_rust(list: &[Node], depth: usize, out: &mut Vec<String>) {
    let pad = "    ".repeat(depth);
    for n in list {
        let line = match n.kind {
            Kind::Forward => "forward();".to_string(),
            Kind::TurnLeft => "turn_left();".to_string(),
            Kind::TurnRight => "turn_right();".to_string(),
            Kind::Mine => "mine();".to_string(),
            Kind::Place => "place_block();".to_string(),
            Kind::Call => "my_function();".to_string(),
            Kind::Repeat => format!("for _ in 0..{} {{", n.count),
            Kind::If | Kind::IfElse => format!("if {} {{", n.cond.code()),
            Kind::While => format!("while {} {{", n.cond.code()),
        };
        out.push(format!("{pad}{line}"));
        if n.kind.is_container() {
            gen_rust(&n.body, depth + 1, out);
            if n.kind == Kind::IfElse {
                out.push(format!("{pad}}} else {{"));
                gen_rust(&n.els, depth + 1, out);
            }
            out.push(format!("{pad}}}"));
        }
    }
}

fn ser(list: &[Node], s: &mut String) {
    for n in list {
        let tok = match n.kind {
            Kind::Forward => "F".to_string(),
            Kind::TurnLeft => "L".to_string(),
            Kind::TurnRight => "R".to_string(),
            Kind::Mine => "M".to_string(),
            Kind::Place => "P".to_string(),
            Kind::Call => "C".to_string(),
            Kind::Repeat => format!("[R{}", n.count),
            Kind::If => format!("[I{}", n.cond.index()),
            Kind::IfElse => format!("[E{}", n.cond.index()),
            Kind::While => format!("[W{}", n.cond.index()),
        };
        s.push_str(&tok);
        s.push(' ');
        if n.kind.is_container() {
            ser(&n.body, s);
            if n.kind == Kind::IfElse {
                s.push_str("| ");
                ser(&n.els, s);
            }
            s.push_str("] ");
        }
    }
}

fn parse_list(s: &str) -> Option<Vec<Node>> {
    let toks: Vec<&str> = s.split_whitespace().collect();
    let mut i = 0;
    let list = parse_seq(&toks, &mut i)?;
    if i == toks.len() { Some(list) } else { None }
}

fn parse_seq(toks: &[&str], i: &mut usize) -> Option<Vec<Node>> {
    let mut out = Vec::new();
    while *i < toks.len() {
        let t = toks[*i];
        if t == "]" || t == "|" {
            break;
        }
        *i += 1;
        let simple = |k| Some(Node::new(k, Cond::PathAhead));
        let node = match t {
            "F" => simple(Kind::Forward),
            "L" => simple(Kind::TurnLeft),
            "R" => simple(Kind::TurnRight),
            "M" => simple(Kind::Mine),
            "P" => simple(Kind::Place),
            "C" => simple(Kind::Call),
            _ if t.starts_with('[') && t.len() >= 3 => {
                let arg: usize = t[2..].parse().ok()?;
                let kind = match &t[1..2] {
                    "R" => Kind::Repeat,
                    "I" => Kind::If,
                    "E" => Kind::IfElse,
                    "W" => Kind::While,
                    _ => return None,
                };
                let mut n = Node::new(kind, Cond::PathAhead);
                if kind == Kind::Repeat {
                    n.count = arg.clamp(1, 99) as u32;
                } else {
                    n.cond = *ALL_CONDS.get(arg)?;
                }
                n.body = parse_seq(toks, i)?;
                if kind == Kind::IfElse {
                    if toks.get(*i) != Some(&"|") {
                        return None;
                    }
                    *i += 1;
                    n.els = parse_seq(toks, i)?;
                }
                if toks.get(*i) != Some(&"]") {
                    return None;
                }
                *i += 1;
                Some(n)
            }
            _ => None,
        }?;
        out.push(node);
    }
    Some(out)
}

#[derive(Clone, Copy, Debug)]
enum Op {
    Act(Action, u32),
    Push(u32),
    Loop(usize),
    JumpIfNot(Cond, usize, u32),
    Jump(usize),
    Call(u32),
    Ret,
    Halt,
}

fn compile(list: &[Node], ops: &mut Vec<Op>) {
    for n in list {
        if let Some(a) = n.kind.action() {
            ops.push(Op::Act(a, n.id));
            continue;
        }
        match n.kind {
            Kind::Call => ops.push(Op::Call(n.id)),
            Kind::Repeat => {
                ops.push(Op::Push(n.count.max(1)));
                let top = ops.len();
                compile(&n.body, ops);
                ops.push(Op::Loop(top));
            }
            Kind::While => {
                let top = ops.len();
                ops.push(Op::JumpIfNot(n.cond, 0, n.id));
                compile(&n.body, ops);
                ops.push(Op::Jump(top));
                let end = ops.len();
                ops[top] = Op::JumpIfNot(n.cond, end, n.id);
            }
            Kind::If => {
                let j = ops.len();
                ops.push(Op::JumpIfNot(n.cond, 0, n.id));
                compile(&n.body, ops);
                let end = ops.len();
                ops[j] = Op::JumpIfNot(n.cond, end, n.id);
            }
            Kind::IfElse => {
                let j = ops.len();
                ops.push(Op::JumpIfNot(n.cond, 0, n.id));
                compile(&n.body, ops);
                let k = ops.len();
                ops.push(Op::Jump(0));
                let else_start = ops.len();
                compile(&n.els, ops);
                let end = ops.len();
                ops[j] = Op::JumpIfNot(n.cond, else_start, n.id);
                ops[k] = Op::Jump(end);
            }
            _ => unreachable!(),
        }
    }
}

pub enum Step {
    /// Do this action; the id is the block that asked for it.
    Act(Action, u32),
    /// The program reached its end.
    Done,
    /// The program got stuck; the id (if any) is the block to blame.
    Error(String, Option<u32>),
}

pub struct Vm {
    ops: Vec<Op>,
    func_start: usize,
    pc: usize,
    counters: Vec<u32>,
    calls: Vec<usize>,
    pub actions: u32,
    last_block: Option<u32>,
}

impl Vm {
    pub fn new(p: &Program) -> Vm {
        let mut ops = Vec::new();
        compile(&p.main, &mut ops);
        ops.push(Op::Halt);
        let func_start = ops.len();
        compile(&p.func, &mut ops);
        ops.push(Op::Ret);
        Vm { ops, func_start, pc: 0, counters: Vec::new(), calls: Vec::new(), actions: 0, last_block: None }
    }

    /// Runs the program until it wants to do the next action.
    pub fn next(&mut self, w: &World) -> Step {
        let mut spins = 0;
        loop {
            spins += 1;
            if spins > 5000 {
                return Step::Error(
                    "Your program is stuck in a loop that never does anything! \
                     Check that your WHILE loop has blocks inside that move the miner."
                        .into(),
                    self.last_block,
                );
            }
            match self.ops[self.pc] {
                Op::Act(a, id) => {
                    self.pc += 1;
                    self.actions += 1;
                    self.last_block = Some(id);
                    if self.actions > MAX_ACTIONS {
                        return Step::Error(
                            format!(
                                "Your program did more than {MAX_ACTIONS} steps! The miner might be \
                                 walking in circles forever. Watch closely and look for the bug."
                            ),
                            Some(id),
                        );
                    }
                    return Step::Act(a, id);
                }
                Op::Push(n) => {
                    self.counters.push(n);
                    self.pc += 1;
                }
                Op::Loop(top) => {
                    let c = self.counters.last_mut().expect("loop counter");
                    *c -= 1;
                    if *c > 0 {
                        self.pc = top;
                    } else {
                        self.counters.pop();
                        self.pc += 1;
                    }
                }
                Op::JumpIfNot(cond, target, id) => {
                    self.last_block = Some(id);
                    self.pc = if w.check(cond) { self.pc + 1 } else { target };
                }
                Op::Jump(t) => self.pc = t,
                Op::Call(id) => {
                    self.last_block = Some(id);
                    if self.calls.len() >= MAX_CALL_DEPTH {
                        return Step::Error(
                            "Your function keeps calling itself over and over and never stops!".into(),
                            Some(id),
                        );
                    }
                    self.calls.push(self.pc + 1);
                    self.pc = self.func_start;
                }
                Op::Ret => match self.calls.pop() {
                    Some(r) => self.pc = r,
                    None => return Step::Done,
                },
                Op::Halt => return Step::Done,
            }
        }
    }
}

/// Runs a whole program instantly (no animation). Returns Ok if it wins.
pub fn simulate(p: &Program, mut w: World) -> Result<(), String> {
    let mut vm = Vm::new(p);
    loop {
        match vm.next(&w) {
            Step::Act(a, _) => match w.apply(a) {
                Outcome::Ok => {}
                Outcome::Won => return Ok(()),
                Outcome::Fail(f) => return Err(f.message()),
            },
            Step::Done => return Err(w.unfinished_message()),
            Step::Error(msg, _) => return Err(msg),
        }
    }
}
