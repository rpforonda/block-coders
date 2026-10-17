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
const FOOTER_H: f32 = 44.0;

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

pub fn kind_color(k: Kind) -> Color {
    match k {
        Kind::Forward | Kind::TurnLeft | Kind::TurnRight => rgb8(60, 120, 230),
        Kind::Mine | Kind::Place => rgb8(220, 120, 30),
        Kind::Repeat | Kind::While => rgb8(225, 170, 20),
        Kind::If | Kind::IfElse => rgb8(140, 80, 210),
        Kind::Call => rgb8(215, 65, 140),
    }
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

pub struct Editor {
    pub tab: Tab,
    pub cursor: Cursor,
    scroll: f32,
    follow_cursor: bool,
    last_highlight: Option<u32>,
}

impl Editor {
    pub fn new() -> Editor {
        Editor {
            tab: Tab::Main,
            cursor: Cursor { list: LPath::root(Tab::Main), index: usize::MAX },
            scroll: 0.0,
            follow_cursor: true,
            last_highlight: None,
        }
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
    ) -> bool {
        let has_func = level.palette.contains(&Kind::Call);
        if !has_func {
            self.tab = Tab::Main;
        }
        self.validate(p);
        let mut edit = None;

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

        // Scrolling
        if ui.hover(rows_area) && ui.wheel != 0.0 {
            self.scroll -= ui.wheel.signum() * 40.0;
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
        if running.is_some() && running != self.last_highlight {
            if let Some(r) = rows.iter().find(|r| {
                r.kind == RowKind::Block && list_ref(p, &r.list).and_then(|l| l.get(r.index)).map(|n| n.id) == running
            }) {
                if r.y - self.scroll > rows_area.h - ROW_H - 10.0 || r.y - self.scroll < 0.0 {
                    self.scroll = r.y - rows_area.h / 2.0;
                }
            }
        }
        self.last_highlight = running;
        self.scroll = self.scroll.clamp(0.0, (content_h - rows_area.h).max(0.0));

        let top = rows_area.y - self.scroll;
        let visible = |y: f32, h: f32| y >= rows_area.y - 1.0 && y + h <= rows_area.y + rows_area.h + 1.0;
        let bx = |depth: usize| area.x + 8.0 + depth as f32 * INDENT;
        let bw = |depth: usize| area.w - 16.0 - depth as f32 * INDENT;

        if root.is_empty() {
            let msg = if self.tab == Tab::Func {
                "Build your function here. Then use CALL FUNCTION in MAIN to run it!"
            } else {
                "Click a block on the left to add it here!"
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
            block_icon(node.kind, x + 5.0, sy + 6.0, 22.0, art);
            let dark_text = luminance(color) > 0.8;
            let tc = if dark_text { rgb8(40, 30, 0) } else { WHITE };
            let label_x = x + 34.0;
            let ty = sy + 9.0;
            let say = |s: &str, x: f32| if dark_text { text(s, x, ty, 18.0, tc) } else { shadow_text(s, x, ty, 18.0, tc) };
            match node.kind {
                Kind::Repeat => {
                    say("repeat", label_x);
                    let mx = label_x + text_width("repeat", 18.0) + 8.0;
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
                    say("times", plus.x + 26.0);
                }
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
            if Some(node.id) == running {
                let c = Color::new(1.0, 1.0, 1.0, 0.6 + 0.4 * hl_pulse);
                draw_rectangle_lines(rect.x - 4.0, rect.y - 4.0, rect.w + 8.0, rect.h + 8.0, 4.0, c);
                draw_triangle(vec2(area.x + 1.0, sy + 8.0), vec2(area.x + 1.0, sy + 28.0), vec2(area.x + 8.0, sy + 18.0), WHITE);
            }
            if Some(node.id) == error {
                draw_rectangle_lines(rect.x - 4.0, rect.y - 4.0, rect.w + 8.0, rect.h + 8.0, 4.0, rgb8(255, 60, 60));
            }
        }

        // Cursor and hover preview.
        let nearest = if ui.hover(rows_area) {
            slots.iter().min_by(|a, b| {
                let da = (top + a.y - ui.mouse.y).abs();
                let db = (top + b.y - ui.mouse.y).abs();
                da.partial_cmp(&db).unwrap()
            })
        } else {
            None
        };
        if let Some(s) = nearest {
            if s.cur != self.cursor && visible(top + s.y - 2.0, 4.0) {
                draw_rectangle(bx(s.depth), top + s.y - 2.0, bw(s.depth), 4.0, Color::new(1.0, 1.0, 0.3, 0.3));
            }
            if ui.clicked(rows_area) {
                edit = Some(Edit::SetCursor(s.cur.clone()));
            }
        }
        if let Some(s) = slots.iter().find(|s| s.cur == self.cursor) {
            let y = top + s.y;
            if visible(y - 3.0, 6.0) {
                let blink = ((get_time() * 3.0).sin() * 0.3 + 0.7) as f32;
                let c = Color::new(1.0, 0.92, 0.2, blink);
                draw_rectangle(bx(s.depth), y - 3.0, bw(s.depth), 6.0, c);
                draw_triangle(vec2(bx(s.depth) - 8.0, y - 7.0), vec2(bx(s.depth) - 8.0, y + 7.0), vec2(bx(s.depth), y), c);
            }
        }

        // Scroll hints
        if self.scroll > 0.0 {
            text_centered("^ more ^", Rect::new(area.x, rows_area.y - 2.0, area.w, 14.0), 14.0, rgb8(200, 200, 200), false);
        }
        if content_h - self.scroll > rows_area.h + 1.0 {
            text_centered("v more v", Rect::new(area.x, rows_area.y + rows_area.h - 12.0, area.w, 14.0), 14.0, rgb8(200, 200, 200), false);
        }

        // Footer: block counter and clear button.
        let fy = area.y + area.h - FOOTER_H;
        let used = p.block_count();
        let counter = match level.max_blocks {
            Some(m) => format!("Blocks: {used} / {m}"),
            None => format!("Blocks: {used}"),
        };
        let full = level.max_blocks.is_some_and(|m| used >= m);
        shadow_text(&counter, area.x + 12.0, fy + 6.0, 20.0, if full { rgb8(255, 120, 100) } else { WHITE });
        text(&format!("3 stars: {} or fewer", level.par), area.x + 12.0, fy + 26.0, 15.0, rgb8(255, 215, 80));
        let clear = Rect::new(area.x + area.w - 92.0, fy + 6.0, 82.0, 30.0);
        let mut changed = false;
        if ui.button_if(!root.is_empty(), clear, "Clear", rgb8(170, 60, 60), 18.0) {
            self.clear_tab(p);
            changed = true;
        }

        match edit {
            Some(Edit::Delete(list, i)) => {
                if let Some(l) = list_mut(p, &list) {
                    l.remove(i);
                }
                self.cursor = Cursor { list, index: i };
                changed = true;
            }
            Some(Edit::Count(list, i, d)) => {
                if let Some(n) = list_mut(p, &list).and_then(|l| l.get_mut(i)) {
                    n.count = (n.count as i32 + d).clamp(1, 20) as u32;
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
        changed
    }
}
