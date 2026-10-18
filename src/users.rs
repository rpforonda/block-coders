//! "Who's playing?" - pick, create, reset or delete a player.

use crate::art::{draw_tex, Art, SHIRTS};
use crate::keyboard::{self, Key, Keyboard};
use crate::levels::LEVELS;
use crate::save::{Progress, Users, MAX_NAME, MAX_USERS};
use crate::sound::{Sfx, Sounds};
use crate::ui::*;
use crate::world::Dir;
use macroquad::prelude::*;

pub enum Mode {
    List,
    New { name: String, shirt: usize },
    ConfirmReset(u32),
    ConfirmDelete(u32),
}

pub enum UsersOut {
    Stay,
    Back,
    Picked(u32),
}

pub struct UsersScreen {
    pub mode: Mode,
    /// (stars, levels done) for each player, in the same order as `Users::list`.
    stats: Vec<(u32, usize)>,
    /// The on-screen keyboard for typing a name (open by default on touch screens).
    kb: Keyboard,
}

const CARD_W: f32 = 360.0;
const CARD_H: f32 = 250.0;

fn card_rect(i: usize) -> Rect {
    let left = (VW - 3.0 * CARD_W - 2.0 * 30.0) / 2.0;
    Rect::new(left + (i % 3) as f32 * (CARD_W + 30.0), 130.0 + (i / 3) as f32 * (CARD_H + 30.0), CARD_W, CARD_H)
}

impl UsersScreen {
    pub fn new(users: &Users) -> UsersScreen {
        let mut s = UsersScreen { mode: Mode::List, stats: Vec::new(), kb: Keyboard::new() };
        s.refresh(users);
        s
    }

    fn refresh(&mut self, users: &Users) {
        self.stats = users
            .list
            .iter()
            .map(|u| {
                let p = Progress::load(u.id);
                (p.total_stars(), p.levels_done())
            })
            .collect();
    }

    pub fn frame(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, users: &mut Users) -> UsersOut {
        let mut out = UsersOut::Stay;
        if !matches!(self.mode, Mode::New { .. }) {
            // Throw away typing that happened before the name box was open.
            while get_char_pressed().is_some() {}
        }
        let popup = !matches!(self.mode, Mode::List);
        ui.enabled = !popup;

        if ui.button(Rect::new(20.0, 18.0, 110.0, 40.0), "< Back", rgb8(110, 110, 120), 22.0) {
            sfx.play(Sfx::Click);
            out = UsersOut::Back;
        }
        text_centered("Who's playing?", Rect::new(0.0, 40.0, VW, 60.0), 52.0, WHITE, true);

        for (i, u) in users.list.iter().enumerate() {
            let r = card_rect(i);
            let hover = ui.hover(r);
            bevel(r, if hover { rgb8(95, 95, 110) } else { rgb8(70, 70, 85) });
            let bob = if hover { (get_time() * 8.0).sin() as f32 * 3.0 } else { 0.0 };
            draw_tex(art.player(u.shirt, Dir::Down), r.x + 22.0, r.y + 30.0 + bob, 84.0, 112.0, WHITE);
            shadow_text(&u.name, r.x + 126.0, r.y + 30.0, 34.0, WHITE);
            let (stars, done) = self.stats.get(i).copied().unwrap_or((0, 0));
            text(&format!("Levels done: {done} / {}", LEVELS.len()), r.x + 128.0, r.y + 80.0, 20.0, rgb8(210, 210, 220));
            star(r.x + 140.0, r.y + 122.0, 12.0, true);
            text(&format!("{stars} / {}", LEVELS.len() * 3), r.x + 160.0, r.y + 112.0, 22.0, rgb8(255, 215, 40));

            let reset = Rect::new(r.x + 20.0, r.y + r.h - 58.0, 150.0, 40.0);
            let delete = Rect::new(r.x + r.w - 150.0, r.y + r.h - 58.0, 130.0, 40.0);
            if ui.button(reset, "Start over", rgb8(200, 120, 30), 20.0) {
                sfx.play(Sfx::Click);
                self.mode = Mode::ConfirmReset(u.id);
            } else if ui.button(delete, "Delete", rgb8(170, 50, 50), 20.0) {
                sfx.play(Sfx::Click);
                self.mode = Mode::ConfirmDelete(u.id);
            } else if ui.clicked(r) {
                sfx.play(Sfx::Click);
                out = UsersOut::Picked(u.id);
            }
        }

        let n = users.list.len();
        if n < MAX_USERS {
            let r = card_rect(n);
            if ui.button(r, "", rgb8(50, 120, 60), 20.0) {
                sfx.play(Sfx::Click);
                self.mode = Mode::New { name: String::new(), shirt: n % SHIRTS.len() };
                self.kb = Keyboard::new();
            }
            let (cx, cy) = (r.x + r.w / 2.0, r.y + 90.0);
            for (o, c) in [(4.0, rgb8(20, 60, 25)), (0.0, WHITE)] {
                draw_rectangle(cx - 40.0 + o, cy - 11.0 + o, 80.0, 22.0, c);
                draw_rectangle(cx - 11.0 + o, cy - 40.0 + o, 22.0, 80.0, c);
            }
            text_centered("New player", Rect::new(r.x, r.y + 160.0, r.w, 40.0), 32.0, WHITE, true);
        }
        let tip = if n == 0 {
            "Make a player to start playing! Each player saves their own stars and programs."
        } else {
            "Click your player to play. \"Start over\" erases that player's progress so they begin again at level 1."
        };
        text_centered(tip, Rect::new(0.0, 742.0, VW, 30.0), 20.0, WHITE, true);

        ui.enabled = true;
        if popup {
            draw_rectangle(0.0, 0.0, VW, VH, Color::new(0.0, 0.0, 0.0, 0.6));
        }
        if let Some(id) = self.popup(ui, art, sfx, users) {
            out = UsersOut::Picked(id);
        }
        out
    }

    /// Draws the open popup. Returns a player id if a new player was created.
    fn popup(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, users: &mut Users) -> Option<u32> {
        let name_of = |id: u32| users.list.iter().find(|u| u.id == id).map(|u| u.name.clone()).unwrap_or_default();
        match &mut self.mode {
            Mode::List => None,
            Mode::New { name, shirt } => {
                let push = |name: &mut String, c: char| {
                    if (c.is_alphanumeric() || c == ' ' || c == '-' || c == '_') && name.chars().count() < MAX_NAME {
                        if !(c == ' ' && name.is_empty()) {
                            name.push(c);
                        }
                    }
                };
                while let Some(c) = get_char_pressed() {
                    push(name, c);
                }
                if is_key_pressed(KeyCode::Backspace) {
                    name.pop();
                }
                // With the keyboard showing, the popup moves up out of its way.
                let r = if self.kb.open {
                    Rect::new(VW / 2.0 - 380.0, keyboard::AREA.y - 470.0, 760.0, 460.0)
                } else {
                    Rect::new(VW / 2.0 - 380.0, 150.0, 760.0, 480.0)
                };
                if self.kb.open {
                    match self.kb.frame(ui, sfx, name.is_empty()) {
                        Some(Key::Char(c)) => push(name, c),
                        Some(Key::Backspace) => {
                            name.pop();
                        }
                        Some(Key::Hide) | None => {}
                    }
                }
                panel(r);
                shadow_text("New player", r.x + 30.0, r.y + 24.0, 38.0, rgb8(255, 215, 40));
                text(if self.kb.open { "Tap the letters to type your name:" } else { "Type your name:" }, r.x + 32.0, r.y + 86.0, 22.0, rgb8(40, 40, 40));
                let field = Rect::new(r.x + 30.0, r.y + 116.0, r.w - 60.0, 56.0);
                draw_rectangle(field.x, field.y, field.w, field.h, rgb8(30, 30, 38));
                draw_rectangle_lines(field.x, field.y, field.w, field.h, 3.0, rgb8(255, 215, 40));
                text(name, field.x + 16.0, field.y + 12.0, 34.0, WHITE);
                if (get_time() * 2.0).fract() < 0.5 {
                    let cx = field.x + 18.0 + text_width(name, 34.0);
                    draw_rectangle(cx, field.y + 12.0, 4.0, 32.0, WHITE);
                }
                if !self.kb.open {
                    // Brings the on-screen keyboard back (also works without a touch screen).
                    let b = Rect::new(field.x + field.w - 66.0, field.y + 8.0, 58.0, 40.0);
                    if ui.button(b, "", rgb8(70, 110, 170), 20.0) {
                        sfx.play(Sfx::Click);
                        self.kb.open = true;
                    }
                    keyboard::keyboard_icon(b, WHITE);
                }

                text("Pick your miner:", r.x + 32.0, r.y + 196.0, 22.0, rgb8(40, 40, 40));
                for i in 0..SHIRTS.len() {
                    let b = Rect::new(r.x + 30.0 + i as f32 * 88.0, r.y + 228.0, 78.0, 100.0);
                    let selected = *shirt == i;
                    bevel(b, if selected { rgb8(255, 215, 40) } else if ui.hover(b) { rgb8(150, 150, 160) } else { rgb8(110, 110, 120) });
                    draw_tex(art.player(i, Dir::Down), b.x + 9.0, b.y + 8.0, 60.0, 80.0, WHITE);
                    if ui.clicked(b) {
                        sfx.play(Sfx::Click);
                        *shirt = i;
                    }
                }

                let clean = name.trim().to_string();
                let taken = users.list.iter().any(|u| u.name.eq_ignore_ascii_case(&clean));
                if taken {
                    text("Someone already has that name - try another!", r.x + 32.0, r.y + 350.0, 20.0, rgb8(180, 40, 40));
                }
                let ok = !clean.is_empty() && !taken;
                let create = ui.button_if(ok, Rect::new(r.x + r.w - 250.0, r.y + r.h - 80.0, 220.0, 54.0), "Let's play!", rgb8(60, 170, 60), 26.0);
                if create || (ok && is_key_pressed(KeyCode::Enter)) {
                    sfx.play(Sfx::Add);
                    let id = users.add(&clean, *shirt);
                    self.mode = Mode::List;
                    self.refresh(users);
                    return Some(id);
                }
                if ui.button(Rect::new(r.x + 30.0, r.y + r.h - 80.0, 160.0, 54.0), "Cancel", rgb8(110, 110, 120), 24.0)
                    || is_key_pressed(KeyCode::Escape)
                {
                    sfx.play(Sfx::Click);
                    self.mode = Mode::List;
                }
                None
            }
            Mode::ConfirmReset(id) | Mode::ConfirmDelete(id) => {
                let id = *id;
                let reset = matches!(self.mode, Mode::ConfirmReset(_));
                let name = name_of(id);
                let r = Rect::new(VW / 2.0 - 340.0, 230.0, 680.0, 330.0);
                panel(r);
                draw_tex(&art.bit, r.x + 24.0, r.y + 24.0, 72.0, 72.0, WHITE);
                let (title, body, yes) = if reset {
                    (
                        format!("Start over for {name}?"),
                        format!(
                            "This erases all of {name}'s stars and saved programs. {name} will start again from level 1. This can't be undone!"
                        ),
                        "Yes, start over",
                    )
                } else {
                    (
                        format!("Delete {name}?"),
                        format!("This removes {name} and all of their stars and programs. This can't be undone!"),
                        "Yes, delete",
                    )
                };
                shadow_text(&title, r.x + 116.0, r.y + 44.0, 32.0, rgb8(255, 200, 80));
                paragraph(&body, r.x + 30.0, r.y + 120.0, r.w - 60.0, 21.0, rgb8(30, 30, 30));
                if ui.button(Rect::new(r.x + r.w - 270.0, r.y + r.h - 76.0, 240.0, 50.0), yes, rgb8(190, 50, 50), 22.0) {
                    sfx.play(Sfx::Bonk);
                    if reset {
                        Progress::new(Some(id)).reset();
                    } else {
                        users.remove(id);
                    }
                    self.mode = Mode::List;
                    self.refresh(users);
                } else if ui.button(Rect::new(r.x + 30.0, r.y + r.h - 76.0, 200.0, 50.0), "No, keep it", rgb8(60, 170, 60), 22.0)
                    || is_key_pressed(KeyCode::Escape)
                {
                    sfx.play(Sfx::Click);
                    self.mode = Mode::List;
                }
                None
            }
        }
    }
}
