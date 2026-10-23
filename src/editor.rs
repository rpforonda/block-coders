//! The program editor: shows the code blocks, a yellow insertion cursor, and
//! lets kids delete blocks, change repeat counts and change conditions.

use crate::art::{shade, Art};
use crate::levels::Level;
use crate::program::{Cond, Kind, Node, Program};
use crate::ui::*;
use macroquad::prelude::*;

const ROW_H: f32 = 36.0;
const ELSE_H: f32 = 26.0;
const END_H: f32 = 14.0;
const EMPTY_H: f32 = 24.0;
const INDENT: f32 = 18.0;
const TAB_H: f32 = 36.0;
const FOOTER_H: f32 = 80.0;
/// How far (in pixels) the mouse must move before a press becomes a drag.
const DRAG_START: f32 = 6.0;
/// Temporary id used while moving a block (real ids count up from 1).
const PLACEHOLDER: u32 = u32::MAX;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Tab {
    Main,
    Func,
}

/// Where a list of blocks lives: the tab, then which child list of which
/// block at each level of nesting (`true` = the else part).
#[derive(Clone, PartialEq, Debug)]
pub struct LPath {
    pub tab: Tab,
    pub steps: Vec<(usize, bool)>,
}

impl LPath {
    fn root(tab: Tab) -> LPath {
        LPath { tab, steps: Vec::new() }
    }
    fn child(&self, i: usize, els: bool) -> LPath {
        let mut p = self.clone();
        p.steps.push((i, els));
        p
    }
}

#[derive(Clone, PartialEq, Debug)]
pub struct Cursor {
    pub list: LPath,
    pub index: usize,
}

/// A block's colour, made readable (see `ui::readable_bg`): its label goes
/// on it in `text_on(kind_color(k))`.
pub fn kind_color(k: Kind) -> Color {
    readable_bg(match k {
        Kind::Forward | Kind::TurnLeft | Kind::TurnRight => rgb8(60, 120, 230),
        Kind::Mine | Kind::Place => rgb8(220, 120, 30),
        Kind::Repeat | Kind::While => rgb8(225, 170, 20),
        Kind::If | Kind::IfElse => rgb8(140, 80, 210),
        Kind::Call => rgb8(215, 65, 140),
        Kind::Set | Kind::Add | Kind::RepeatCount => rgb8(30, 160, 150),
    })
}

fn list_ref<'a>(p: &'a Program, path: &LPath) -> Option<&'a Vec<Node>> {
    let mut l = match path.tab {
        Tab::Main => &p.main,
        Tab::Func => &p.func,
    };
    for &(i, els) in &path.steps {
        let n = l.get(i)?;
        if !n.kind.is_container() || (els && n.kind != Kind::IfElse) {
            return None;
        }
        l = if els { &n.els } else { &n.body };
    }
    Some(l)
}

fn list_mut<'a>(p: &'a mut Program, path: &LPath) -> Option<&'a mut Vec<Node>> {
    let mut l = match path.tab {
        Tab::Main => &mut p.main,
        Tab::Func => &mut p.func,
    };
    for &(i, els) in &path.steps {
        let n = l.get_mut(i)?;
        if !n.kind.is_container() || (els && n.kind != Kind::IfElse) {
            return None;
        }
        l = if els { &mut n.els } else { &mut n.body };
    }
    Some(l)
}

fn default_cond(kind: Kind, level: &Level) -> Cond {
    let c = level.conds;
    if c.is_empty() {
        Cond::PathAhead
    } else if kind == Kind::While && c.contains(&Cond::NotAtPortal) {
        Cond::NotAtPortal
    } else {
        *c.iter().find(|&&x| x != Cond::NotAtPortal).unwrap_or(&c[0])
    }
}

#[derive(PartialEq)]
enum RowKind {
    Block,
    Else,
    End,
}

struct Row {
    y: f32,
    h: f32,
    depth: usize,
    kind: RowKind,
    list: LPath,
    index: usize,
    start_y: f32,
}

struct Slot {
    y: f32,
    depth: usize,
    cur: Cursor,
}

fn layout(list: &[Node], path: LPath, depth: usize, y: &mut f32, rows: &mut Vec<Row>, slots: &mut Vec<Slot>) {
    if list.is_empty() && depth > 0 {
        slots.push(Slot { y: *y + EMPTY_H / 2.0, depth, cur: Cursor { list: path, index: 0 } });
        *y += EMPTY_H;
        return;
    }
    for (i, n) in list.iter().enumerate() {
        slots.push(Slot { y: *y, depth, cur: Cursor { list: path.clone(), index: i } });
        let start = *y;
        let row = |y: f32, h: f32, kind: RowKind| Row {
            y,
            h,
            depth,
            kind,
            list: path.clone(),
            index: i,
            start_y: start,
        };
        rows.push(row(*y, ROW_H, RowKind::Block));
        *y += ROW_H;
        if n.kind.is_container() {
            layout(&n.body, path.child(i, false), depth + 1, y, rows, slots);
            if n.kind == Kind::IfElse {
                rows.push(row(*y, ELSE_H, RowKind::Else));
                *y += ELSE_H;
                layout(&n.els, path.child(i, true), depth + 1, y, rows, slots);
            }
            rows.push(row(*y, END_H, RowKind::End));
            *y += END_H;
        }
    }
    slots.push(Slot { y: *y, depth, cur: Cursor { list: path, index: list.len() } });
}

enum Edit {
    Delete(LPath, usize),
    Count(LPath, usize, i32),
    CycleCond(LPath, usize),
    SetCursor(Cursor),
    Tab(Tab),
}

/// What is being dragged: a new block from the palette, or a block (with
/// everything inside it) that's already in the program.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum DragSource {
    Palette(Kind),
    Block(u32),
}

#[derive(Clone, Copy, Debug)]
struct Drag {
    source: DragSource,
    start: Vec2,
    /// False until the mouse moves far enough; a press-and-release without
    /// moving is treated as a click.
    active: bool,
}

/// What happened in the editor this frame.
#[derive(Default)]
pub struct EditorOut {
    pub changed: bool,
    pub undo: bool,
    pub redo: bool,
}

/// Removes the block with this id from wherever it is in the tree.
fn take_node(list: &mut Vec<Node>, id: u32) -> Option<Node> {
    if let Some(i) = list.iter().position(|n| n.id == id) {
        return Some(list.remove(i));
    }
    for n in list.iter_mut() {
        if let Some(found) = take_node(&mut n.body, id) {
            return Some(found);
        }
        if let Some(found) = take_node(&mut n.els, id) {
            return Some(found);
        }
    }
    None
}

fn take(p: &mut Program, id: u32) -> Option<Node> {
    take_node(&mut p.main, id).or_else(|| take_node(&mut p.func, id))
}

/// Finds which list (and position in it) holds the block with this id.
fn locate(list: &[Node], path: LPath, id: u32) -> Option<(LPath, usize)> {
    for (i, n) in list.iter().enumerate() {
        if n.id == id {
            return Some((path, i));
        }
        if let Some(found) = locate(&n.body, path.child(i, false), id) {
            return Some(found);
        }
        if let Some(found) = locate(&n.els, path.child(i, true), id) {
            return Some(found);
        }
    }
    None
}

fn locate_in(p: &Program, id: u32) -> Option<(LPath, usize)> {
    locate(&p.main, LPath::root(Tab::Main), id).or_else(|| locate(&p.func, LPath::root(Tab::Func), id))
}

pub struct Editor {
    pub tab: Tab,
    pub cursor: Cursor,
    scroll: f32,
    bar: Scrollbar,
    follow_cursor: bool,
    last_highlight: Option<u32>,
    drag: Option<Drag>,
    /// The block a hint is pointing at (outlined in yellow).
    pub hint: Option<u32>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            tab: Tab::Main,
            cursor: Cursor { list: LPath::root(Tab::Main), index: usize::MAX },
            scroll: 0.0,
            bar: Scrollbar::default(),
            follow_cursor: true,
            last_highlight: None,
            drag: None,
            hint: None,
        }
    }

    /// Called when the mouse goes down on a palette block. It becomes a drag
    /// if the mouse moves, or a click (insert at the cursor) if it doesn't.
    pub fn press_palette(&mut self, kind: Kind, mouse: Vec2) {
        self.drag = Some(Drag { source: DragSource::Palette(kind), start: mouse, active: false });
    }

    /// Moves a block (and everything inside it) to `target`. A placeholder
    /// marks the target first, so removing the block can't shift the spot.
    fn move_node(&mut self, p: &mut Program, id: u32, target: &Cursor) -> bool {
        // A block can't go inside itself: taking it out would take the target too.
        let Some((from, i)) = locate_in(p, id) else { return false };
        let t = &target.list;
        if t.tab == from.tab && t.steps.len() > from.steps.len() && t.steps[..from.steps.len()] == from.steps[..] && t.steps[from.steps.len()].0 == i {
            return false;
        }
        let Some(list) = list_mut(p, &target.list) else { return false };
        let index = target.index.min(list.len());
        list.insert(index, Node { id: PLACEHOLDER, ..Node::new(Kind::Forward, Cond::PathAhead) });
        let Some(node) = take(p, id) else {
            take(p, PLACEHOLDER);
            return false;
        };
        let Some((path, i)) = locate_in(p, PLACEHOLDER) else { return false };
        list_mut(p, &path).unwrap()[i] = node;
        self.cursor = Cursor { list: path, index: i + 1 };
        self.follow_cursor = true;
        true
    }

    /// Makes sure the cursor points somewhere real in the visible tab.
    fn validate(&mut self, p: &Program) {
        if self.cursor.list.tab != self.tab {
            self.cursor = Cursor { list: LPath::root(self.tab), index: usize::MAX };
        }
        match list_ref(p, &self.cursor.list) {
            Some(l) => self.cursor.index = self.cursor.index.min(l.len()),
            None => {
                let len = list_ref(p, &LPath::root(self.tab)).map_or(0, |l| l.len());
                self.cursor = Cursor { list: LPath::root(self.tab), index: len };
            }
        }
    }

    pub fn insert(&mut self, p: &mut Program, kind: Kind, level: &Level) {
        self.validate(p);
        let node = Node::new(kind, default_cond(kind, level));
        let Some(list) = list_mut(p, &self.cursor.list) else { return };
        list.insert(self.cursor.index, node);
        if kind.is_container() {
            self.cursor = Cursor { list: self.cursor.list.child(self.cursor.index, false), index: 0 };
        } else {
            self.cursor.index += 1;
        }
        self.follow_cursor = true;
    }

    /// Deletes the block just above the cursor (Backspace).
    pub fn delete_before_cursor(&mut self, p: &mut Program) -> bool {
        self.validate(p);
        if self.cursor.index == 0 {
            return false;
        }
        let Some(list) = list_mut(p, &self.cursor.list) else { return false };
        list.remove(self.cursor.index - 1);
        self.cursor.index -= 1;
        self.follow_cursor = true;
        true
    }

    pub fn clear_tab(&mut self, p: &mut Program) {
        match self.tab {
            Tab::Main => p.main.clear(),
            Tab::Func => p.func.clear(),
        }
        self.cursor = Cursor { list: LPath::root(self.tab), index: 0 };
        self.scroll = 0.0;
    }

    /// Draws the editor in `area` and handles clicks. Returns true if the
    /// program changed. `running` is the block currently executing.
    #[allow(clippy::too_many_arguments)]
    pub fn draw(
        &mut self,
        ui: &mut Ui,
        p: &mut Program,
        level: &Level,
        area: Rect,
        running: Option<u32>,
        error: Option<u32>,
        art: &Art,
        can_undo_redo: (bool, bool),
    ) -> EditorOut {
        let has_func = level.palette.contains(&Kind::Call);
        if !has_func {
            self.tab = Tab::Main;
        }
        self.validate(p);
        let mut edit = None;
        if !ui.enabled {
            self.drag = None;
        }
        if let Some(d) = self.drag.as_mut() {
            if !d.active && ui.mouse.distance(d.start) > DRAG_START {
                d.active = true;
            }
        }
        let dragging = self.drag.filter(|d| d.active).map(|d| d.source);

        // Tabs
        dark_panel(area);
        let tab_r = |i: f32| Rect::new(area.x + 6.0 + i * (area.w - 12.0) / 2.0, area.y + 6.0, (area.w - 18.0) / 2.0, TAB_H - 8.0);
        if has_func {
            for (i, (t, name)) in [(Tab::Main, "MAIN"), (Tab::Func, "FUNCTION")].iter().enumerate() {
                let r = tab_r(i as f32);
                let color = if self.tab == *t { kind_color(if *t == Tab::Main { Kind::Forward } else { Kind::Call }) } else { rgb8(80, 80, 90) };
                if ui.button(r, name, color, 18.0) && self.tab != *t {
                    edit = Some(Edit::Tab(*t));
                }
                // Hovering a tab while dragging switches to it, so blocks can
                // move between MAIN and FUNCTION.
                if dragging.is_some() && ui.hover(r) && self.tab != *t {
                    edit = Some(Edit::Tab(*t));
                }
            }
        } else {
            shadow_text("MY PROGRAM", area.x + 12.0, area.y + 9.0, 22.0, WHITE);
        }

        let rows_area = Rect::new(area.x, area.y + TAB_H + 4.0, area.w, area.h - TAB_H - FOOTER_H - 8.0);
        let mut rows = Vec::new();
        let mut slots = Vec::new();
        let root = list_ref(p, &LPath::root(self.tab)).cloned().unwrap_or_default();
        let mut content_h = 6.0;
        layout(&root, LPath::root(self.tab), 0, &mut content_h, &mut rows, &mut slots);
        content_h += 40.0;

        // Scrolling (also when dragging near the top or bottom edge)
        if ui.hover(rows_area) && ui.wheel != 0.0 {
            self.scroll -= ui.wheel.signum() * 40.0;
        }
        if dragging.is_some() && ui.mouse.x >= area.x && ui.mouse.x <= area.x + area.w {
            if ui.mouse.y < rows_area.y + 30.0 && ui.mouse.y > rows_area.y - 40.0 {
                self.scroll -= 8.0;
            } else if ui.mouse.y > rows_area.y + rows_area.h - 30.0 && ui.mouse.y < rows_area.y + rows_area.h + 40.0 {
                self.scroll += 8.0;
            }
        }
        if self.follow_cursor {
            if let Some(s) = slots.iter().find(|s| s.cur == self.cursor) {
                if s.y - self.scroll > rows_area.h - 50.0 {
                    self.scroll = s.y - rows_area.h + 50.0;
                } else if s.y - self.scroll < 10.0 {
                    self.scroll = s.y - 10.0;
                }
            }
            self.follow_cursor = false;
        }
        // Scroll the running block (or the one a hint points at) into view.
        let shown = running.or(self.hint);
        if shown.is_some() && shown != self.last_highlight {
            if let Some(r) = rows.iter().find(|r| {
                r.kind == RowKind::Block && list_ref(p, &r.list).and_then(|l| l.get(r.index)).map(|n| n.id) == shown
            }) {
                if r.y - self.scroll > rows_area.h - ROW_H - 10.0 || r.y - self.scroll < 0.0 {
                    self.scroll = r.y - rows_area.h / 2.0;
                }
            }
        }
        self.last_highlight = shown;
        self.scroll = self.scroll.clamp(0.0, (content_h - rows_area.h).max(0.0));
        // A long program gets a scrollbar on the right, and its blocks make room for it.
        let track = Rect::new(area.x + area.w - SCROLLBAR_W - 3.0, rows_area.y, SCROLLBAR_W, rows_area.h);
        self.bar.update(ui, track, content_h, rows_area.h, &mut self.scroll);
        let bar_w = if content_h > rows_area.h { SCROLLBAR_W + 2.0 } else { 0.0 };

        let top = rows_area.y - self.scroll;
        let visible = |y: f32, h: f32| y >= rows_area.y - 1.0 && y + h <= rows_area.y + rows_area.h + 1.0;
        let bx = |depth: usize| area.x + 8.0 + depth as f32 * INDENT;
        let bw = |depth: usize| area.w - 16.0 - bar_w - depth as f32 * INDENT;

        if root.is_empty() {
            let msg = if self.tab == Tab::Func {
                "Build your function here. Then use CALL FUNCTION in MAIN to run it!"
            } else {
                "Click or drag a block from the left to add it here!"
            };
            paragraph(msg, area.x + 16.0, rows_area.y + 50.0, area.w - 32.0, 18.0, rgb8(170, 170, 185));
        }

        // Pass 1: the "C" shaped spines of containers.
        for r in rows.iter().filter(|r| r.kind == RowKind::End) {
            let node = &list_ref(p, &r.list).unwrap()[r.index];
            let c = shade(kind_color(node.kind), 0.85);
            let y0 = (top + r.start_y + ROW_H - 4.0).max(rows_area.y);
            let y1 = (top + r.y + r.h - 2.0).min(rows_area.y + rows_area.h);
            if y1 > y0 {
                draw_rectangle(bx(r.depth), y0, INDENT - 4.0, y1 - y0, c);
            }
            if visible(top + r.y, r.h) {
                draw_rectangle(bx(r.depth), top + r.y, bw(r.depth) * 0.6, r.h - 3.0, c);
            }
        }

        // Pass 2: blocks and else rows.
        let hl_pulse = (get_time() * 6.0).sin() as f32 * 0.5 + 0.5;
        for r in &rows {
            let sy = top + r.y;
            if !visible(sy, r.h) || r.kind == RowKind::End {
                continue;
            }
            let node = &list_ref(p, &r.list).unwrap()[r.index];
            let x = bx(r.depth);
            let w = bw(r.depth);
            let color = kind_color(node.kind);
            if r.kind == RowKind::Else {
                let rr = Rect::new(x, sy + 1.0, w * 0.6, r.h - 3.0);
                draw_rectangle(rr.x, rr.y, rr.w, rr.h, shade(color, 0.85));
                shadow_text("else", x + 22.0, sy + 4.0, 18.0, WHITE);
                continue;
            }
            let rect = Rect::new(x, sy + 2.0, w, ROW_H - 6.0);
            bevel(rect, color);
            let tc = text_on(color);
            let dark_text = tc != WHITE;
            block_icon(node.kind, x + 5.0, sy + 6.0, 22.0, tc, art);
            let label_x = x + 34.0;
            let ty = sy + 9.0;
            let say = |s: &str, x: f32| if dark_text { text(s, x, ty, 18.0, tc) } else { shadow_text(s, x, ty, 18.0, tc) };
            // Words, a number with - and + buttons, then more words.
            let mut number = |before: &str, after: &str| {
                say(before, label_x);
                let mx = label_x + text_width(before, 18.0) + 8.0;
                let minus = Rect::new(mx, sy + 7.0, 20.0, 20.0);
                let num = format!("{}", node.count);
                let nx = mx + 26.0;
                let plus = Rect::new(nx + text_width("00", 20.0) + 6.0, sy + 7.0, 20.0, 20.0);
                if ui.button(minus, "-", rgb8(250, 240, 200), 20.0) {
                    edit = Some(Edit::Count(r.list.clone(), r.index, -1));
                }
                text(&num, nx + (text_width("00", 20.0) - text_width(&num, 20.0)) / 2.0, sy + 8.0, 20.0, tc);
                if ui.button(plus, "+", rgb8(250, 240, 200), 20.0) {
                    edit = Some(Edit::Count(r.list.clone(), r.index, 1));
                }
                say(after, plus.x + 26.0);
            };
            match node.kind {
                Kind::Repeat => number("repeat", "times"),
                Kind::Set => number("set count to", ""),
                Kind::Add => number("add", "to count"),
                Kind::RepeatCount => say("repeat count times", label_x),
                Kind::If | Kind::IfElse | Kind::While => {
                    let word = if node.kind == Kind::While { "while" } else { "if" };
                    say(word, label_x);
                    let cx = label_x + text_width(word, 18.0) + 8.0;
                    let label = node.cond.label();
                    let chip = Rect::new(cx, sy + 6.0, text_width(label, 18.0) + 14.0, 22.0);
                    if ui.button(chip, label, rgb8(245, 245, 245), 18.0) {
                        edit = Some(Edit::CycleCond(r.list.clone(), r.index));
                    }
                }
                k => say(k.palette_label(), label_x),
            }
            let del = Rect::new(x + w - 25.0, sy + 8.0, 18.0, 18.0);
            if ui.hover(rect) || ui.hover(del) {
                if ui.button(del, "x", rgb8(200, 50, 50), 18.0) {
                    edit = Some(Edit::Delete(r.list.clone(), r.index));
                }
                if ui.rclick {
                    ui.rclick = false;
                    edit = Some(Edit::Delete(r.list.clone(), r.index));
                }
            }
            if ui.click && ui.hover(rect) {
                // Press on a block: maybe the start of a drag (to move it).
                ui.click = false;
                self.drag = Some(Drag { source: DragSource::Block(node.id), start: ui.mouse, active: false });
            }
            if dragging == Some(DragSource::Block(node.id)) {
                draw_rectangle(rect.x - 2.0, rect.y - 2.0, rect.w + 4.0, rect.h + 4.0, Color::new(0.1, 0.1, 0.12, 0.7));
            }
            if Some(node.id) == running {
                let c = Color::new(1.0, 1.0, 1.0, 0.6 + 0.4 * hl_pulse);
                draw_rectangle_lines(rect.x - 4.0, rect.y - 4.0, rect.w + 8.0, rect.h + 8.0, 4.0, c);
                draw_triangle(vec2(area.x + 1.0, sy + 8.0), vec2(area.x + 1.0, sy + 28.0), vec2(area.x + 8.0, sy + 18.0), WHITE);
            }
            if Some(node.id) == error {
                draw_rectangle_lines(rect.x - 4.0, rect.y - 4.0, rect.w + 8.0, rect.h + 8.0, 4.0, rgb8(255, 60, 60));
            } else if Some(node.id) == self.hint {
                let c = Color::new(1.0, 0.85, 0.15, 0.7 + 0.3 * hl_pulse);
                draw_rectangle_lines(rect.x - 4.0, rect.y - 4.0, rect.w + 8.0, rect.h + 8.0, 4.0, c);
            }
        }

        // While moving a block, it can't be dropped inside itself.
        let inside_dragged = |s: &Slot| -> bool {
            let Some(DragSource::Block(id)) = dragging else { return false };
            let Some((list, i)) = rows.iter().find_map(|r| {
                let n = list_ref(p, &r.list)?.get(r.index)?;
                (r.kind == RowKind::Block && n.id == id).then(|| (r.list.clone(), r.index))
            }) else {
                return false;
            };
            let n = list.steps.len();
            s.cur.list.tab == list.tab
                && s.cur.list.steps.len() > n
                && s.cur.list.steps[..n] == list.steps[..]
                && s.cur.list.steps[n].0 == i
        };

        // Cursor and hover preview.
        let nearest = if ui.hover(rows_area) {
            slots.iter().filter(|s| !inside_dragged(s)).min_by(|a, b| {
                let da = (top + a.y - ui.mouse.y).abs();
                let db = (top + b.y - ui.mouse.y).abs();
                da.partial_cmp(&db).unwrap()
            })
        } else {
            None
        };
        let drop_target = if dragging.is_some() { nearest.map(|s| s.cur.clone()) } else { None };
        if let Some(s) = nearest {
            if dragging.is_some() {
                // Where the dragged block will land.
                let y = top + s.y;
                draw_rectangle(bx(s.depth), y - 4.0, bw(s.depth), 8.0, rgb8(80, 230, 110));
                draw_triangle(vec2(bx(s.depth) - 10.0, y - 9.0), vec2(bx(s.depth) - 10.0, y + 9.0), vec2(bx(s.depth), y), rgb8(80, 230, 110));
            } else if s.cur != self.cursor && visible(top + s.y - 2.0, 4.0) {
                draw_rectangle(bx(s.depth), top + s.y - 2.0, bw(s.depth), 4.0, Color::new(1.0, 1.0, 0.3, 0.3));
            }
            if ui.clicked(rows_area) {
                edit = Some(Edit::SetCursor(s.cur.clone()));
            }
        }
        if let Some(s) = slots.iter().find(|s| s.cur == self.cursor).filter(|_| dragging.is_none()) {
            let y = top + s.y;
            if visible(y - 3.0, 6.0) {
                let blink = ((get_time() * 3.0).sin() * 0.3 + 0.7) as f32;
                let c = Color::new(1.0, 0.92, 0.2, blink);
                draw_rectangle(bx(s.depth), y - 3.0, bw(s.depth), 6.0, c);
                draw_triangle(vec2(bx(s.depth) - 8.0, y - 7.0), vec2(bx(s.depth) - 8.0, y + 7.0), vec2(bx(s.depth), y), c);
            }
        }

        // Footer: block counter, undo/redo and clear.
        let fy = area.y + area.h - FOOTER_H;
        let used = p.block_count();
        let counter = match level.max_blocks {
            Some(m) => format!("Blocks: {used} / {m}"),
            None => format!("Blocks: {used}"),
        };
        let full = level.max_blocks.is_some_and(|m| used >= m);
        shadow_text(&counter, area.x + 12.0, fy + 6.0, 20.0, if full { rgb8(255, 120, 100) } else { WHITE });
        text(&format!("3 stars: {} or fewer", level.par), area.x + 12.0, fy + 26.0, 15.0, rgb8(255, 215, 80));
        let mut out = EditorOut::default();
        let bw3 = (area.w - 32.0) / 3.0;
        let brow = |i: f32| Rect::new(area.x + 8.0 + i * (bw3 + 8.0), fy + 44.0, bw3, 30.0);
        if ui.button_if(can_undo_redo.0, brow(0.0), "Undo", rgb8(70, 110, 170), 18.0) {
            out.undo = true;
        }
        if ui.button_if(can_undo_redo.1, brow(1.0), "Redo", rgb8(70, 110, 170), 18.0) {
            out.redo = true;
        }
        if ui.button_if(!root.is_empty(), brow(2.0), "Clear", rgb8(170, 60, 60), 18.0) {
            self.clear_tab(p);
            out.changed = true;
        }
        let mut changed = false;

        match edit {
            Some(Edit::Delete(list, i)) => {
                if let Some(l) = list_mut(p, &list) {
                    l.remove(i);
                }
                self.cursor = Cursor { list, index: i };
                changed = true;
            }
            Some(Edit::Count(list, i, d)) => {
                let node = list_mut(p, &list).and_then(|l| l.get_mut(i));
                if let Some((n, (lo, hi))) = node.and_then(|n| n.kind.number_range().map(|r| (n, r))) {
                    n.count = (n.count as i32 + d).clamp(lo as i32, hi as i32) as u32;
                }
                changed = true;
            }
            Some(Edit::CycleCond(list, i)) => {
                if let Some(n) = list_mut(p, &list).and_then(|l| l.get_mut(i)) {
                    let c = level.conds;
                    if !c.is_empty() {
                        let next = c.iter().position(|&x| x == n.cond).map_or(0, |j| (j + 1) % c.len());
                        n.cond = c[next];
                    }
                }
                changed = true;
            }
            Some(Edit::SetCursor(c)) => self.cursor = c,
            Some(Edit::Tab(t)) => {
                self.tab = t;
                self.scroll = 0.0;
                self.validate(p);
            }
            None => {}
        }

        // Finish a drag (or a click) when the mouse button is let go.
        if let Some(d) = self.drag.filter(|_| ui.release || !ui.down) {
            self.drag = None;
            match (d.active, d.source) {
                // A click on a palette block: add it at the cursor.
                (false, DragSource::Palette(k)) => {
                    if !full {
                        self.insert(p, k, level);
                        changed = true;
                    }
                }
                // A click on a program block: move the cursor there.
                (false, DragSource::Block(_)) => {
                    if let Some(s) = nearest {
                        self.cursor = s.cur.clone();
                    }
                }
                (true, DragSource::Palette(k)) => {
                    if let Some(t) = drop_target.filter(|_| !full) {
                        self.cursor = t;
                        self.insert(p, k, level);
                        changed = true;
                    }
                }
                (true, DragSource::Block(id)) => {
                    if let Some(t) = drop_target {
                        changed |= self.move_node(p, id, &t);
                    } else if !ui.hover(area) {
                        // Dropped outside the program: throw the block away.
                        changed |= take(p, id).is_some();
                    }
                }
            }
        }

        // The block being dragged follows the mouse, drawn on top of everything.
        if let Some(src) = dragging {
            let (kind, label) = match src {
                DragSource::Palette(k) => (k, k.palette_label().to_string()),
                DragSource::Block(id) => {
                    let found = locate_in(p, id).and_then(|(l, i)| list_ref(p, &l).map(|l| (l[i].kind, l[i].body.len() + l[i].els.len())));
                    match found {
                        Some((k, 0)) => (k, k.palette_label().to_string()),
                        Some((k, n)) => (k, format!("{} (+{n})", k.palette_label())),
                        None => (Kind::Forward, String::new()),
                    }
                }
            };
            let trash = matches!(src, DragSource::Block(_)) && !ui.hover(area);
            let g = Rect::new(ui.mouse.x - 20.0, ui.mouse.y - 15.0, 190.0, 30.0);
            let bg = if trash { rgb8(150, 50, 50) } else { kind_color(kind) };
            let tc = text_on(bg);
            bevel(g, bg);
            block_icon(kind, g.x + 4.0, g.y + 4.0, 22.0, tc, art);
            if tc == WHITE {
                shadow_text(&label, g.x + 32.0, g.y + 7.0, 18.0, tc);
            } else {
                text(&label, g.x + 32.0, g.y + 7.0, 18.0, tc);
            }
            if trash {
                shadow_text("let go to delete", g.x, g.y + 36.0, 16.0, rgb8(255, 150, 140));
            }
        }

        out.changed |= changed;
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn prog(s: &str) -> Program {
        Program::deserialize(s).unwrap()
    }

    /// Moves the block at `from` (list path + index) to the slot `to`.
    fn mv(src: &str, from: (LPath, usize), to: Cursor) -> String {
        let mut p = prog(src);
        let id = list_ref(&p, &from.0).unwrap()[from.1].id;
        let mut e = Editor::new();
        assert!(e.move_node(&mut p, id, &to));
        p.serialize()
    }

    fn main_at(i: usize) -> Cursor {
        Cursor { list: LPath::root(Tab::Main), index: i }
    }

    fn all_ids(list: &[Node], out: &mut Vec<u32>) {
        for n in list {
            out.push(n.id);
            all_ids(&n.body, out);
            all_ids(&n.els, out);
        }
    }

    /// Every slot in both tabs, as the editor would offer them.
    fn all_slots(p: &Program) -> Vec<Cursor> {
        let mut out = Vec::new();
        for tab in [Tab::Main, Tab::Func] {
            let (mut rows, mut slots, mut y) = (Vec::new(), Vec::new(), 0.0);
            layout(list_ref(p, &LPath::root(tab)).unwrap(), LPath::root(tab), 0, &mut y, &mut rows, &mut slots);
            out.extend(slots.into_iter().map(|s| s.cur));
        }
        out
    }

    /// Random inserts, moves (including onto slots inside the moved block,
    /// which the drag code refuses but `move_node` must survive) and deletes
    /// never lose or duplicate a block, and always leave a program that saves
    /// and loads the same.
    #[test]
    fn random_edits_keep_the_program_whole() {
        use crate::levels::LEVELS;
        use crate::world::Rng;
        let level = LEVELS.iter().find(|l| l.name == "Master Miner").unwrap();
        let kinds = [Kind::Forward, Kind::Mine, Kind::Repeat, Kind::If, Kind::IfElse, Kind::While, Kind::Call, Kind::Set, Kind::RepeatCount];
        let mut rng = Rng::new(9);
        for _ in 0..300 {
            let mut p = Program::default();
            let mut e = Editor::new();
            for _ in 0..40 {
                let before = p.serialize();
                let count = p.block_count();
                let slots = all_slots(&p);
                let mut ids = Vec::new();
                all_ids(&p.main, &mut ids);
                all_ids(&p.func, &mut ids);
                match rng.below(4) {
                    0 | 1 => {
                        let s = slots[rng.below(slots.len())].clone();
                        e.tab = s.list.tab;
                        e.cursor = s;
                        e.insert(&mut p, kinds[rng.below(kinds.len())], level);
                        assert_eq!(p.block_count(), count + 1);
                    }
                    2 if !ids.is_empty() => {
                        let id = ids[rng.below(ids.len())];
                        let target = slots[rng.below(slots.len())].clone();
                        let moved = e.move_node(&mut p, id, &target);
                        assert_eq!(p.block_count(), count, "a move lost or copied blocks: {before} -> {}", p.serialize());
                        if !moved {
                            assert_eq!(p.serialize(), before, "a refused move changed the program");
                        }
                    }
                    _ => {
                        let s = slots[rng.below(slots.len())].clone();
                        e.tab = s.list.tab;
                        e.cursor = s;
                        e.delete_before_cursor(&mut p);
                    }
                }
                let mut ids = Vec::new();
                all_ids(&p.main, &mut ids);
                all_ids(&p.func, &mut ids);
                let n = ids.len();
                ids.sort();
                ids.dedup();
                assert_eq!(ids.len(), n, "duplicate block ids");
                assert!(!ids.contains(&PLACEHOLDER), "a placeholder was left behind");
                let again = Program::deserialize(&p.serialize()).expect("edited program must load");
                assert_eq!(again.serialize(), p.serialize());
            }
        }
    }

    #[test]
    fn reorder_within_a_list() {
        let m = LPath::root(Tab::Main);
        // Move down: the slot after "R" is index 3 before removing "F".
        assert_eq!(mv("F L R", (m.clone(), 0), main_at(3)), "L R F /");
        assert_eq!(mv("F L R", (m.clone(), 2), main_at(0)), "R F L /");
        // Dropping right where it already is changes nothing.
        assert_eq!(mv("F L R", (m.clone(), 1), main_at(1)), "F L R /");
        assert_eq!(mv("F L R", (m, 1), main_at(2)), "F L R /");
    }

    #[test]
    fn move_into_and_out_of_containers() {
        let m = LPath::root(Tab::Main);
        let body = m.child(0, false);
        // Into a repeat's body.
        assert_eq!(mv("[R3 F ] L", (m.clone(), 1), Cursor { list: body.clone(), index: 0 }), "[R3 L F ] /");
        // Out of it, to the end of main.
        assert_eq!(mv("[R3 L F ] R", (body, 0), main_at(2)), "[R3 F ] R L /");
        // Into the else part of an if/else.
        let els = m.child(0, true);
        assert_eq!(mv("[E3 M | ] P", (m, 1), Cursor { list: els, index: 0 }), "[E3 M | P ] /");
    }

    #[test]
    fn containers_move_with_their_children() {
        let m = LPath::root(Tab::Main);
        assert_eq!(mv("F [R2 M F ] L", (m, 1), main_at(0)), "[R2 M F ] F L /");
    }

    #[test]
    fn move_between_main_and_function() {
        let to_func = Cursor { list: LPath::root(Tab::Func), index: 1 };
        assert_eq!(mv("C M / F", (LPath::root(Tab::Main), 1), to_func), "C /F M ");
    }

    #[test]
    fn ids_stay_unique_after_moves() {
        let mut p = prog("F [R2 M F ] L / R");
        let mut e = Editor::new();
        let id = p.main[2].id;
        e.move_node(&mut p, id, &Cursor { list: LPath::root(Tab::Main).child(1, false), index: 1 });
        let mut ids = Vec::new();
        fn collect(l: &[Node], out: &mut Vec<u32>) {
            for n in l {
                out.push(n.id);
                collect(&n.body, out);
                collect(&n.els, out);
            }
        }
        collect(&p.main, &mut ids);
        collect(&p.func, &mut ids);
        assert!(!ids.contains(&PLACEHOLDER));
        let n = ids.len();
        ids.sort();
        ids.dedup();
        assert_eq!(ids.len(), n);
    }
}
