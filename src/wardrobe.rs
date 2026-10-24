//! The Wardrobe screen: the player's miner on show, and a tab each for
//! shirts, armor and tools. Items unlock as the player's stars add up (see
//! `items.rs`); locked ones show how many stars they need.

use crate::art::{outfits::REACH, Art};
use crate::items::{self, Look, Outfit, Slot, CLASSIC, ITEMS, SLOTS};
use crate::keyboard::is_touch;
use crate::layout::{self, Canvas};
use crate::save::{Progress, Saved, User};
use crate::sound::{Sfx, Sounds};
use crate::ui::*;
use crate::world::Dir;
use macroquad::prelude::*;

pub enum WardrobeOut {
    Stay,
    Back,
}

/// One card in a tab: an item, or nothing in that slot.
#[derive(Clone, Copy, PartialEq, Debug)]
pub enum Card {
    Item(usize),
    Nothing(Slot),
}

impl Card {
    fn put_on(self, o: &mut Outfit) {
        match self {
            Card::Item(i) => o.wear(i),
            Card::Nothing(slot) => o.take_off(slot),
        }
    }

    fn stars(self) -> u32 {
        match self {
            Card::Item(i) => ITEMS[i].stars,
            Card::Nothing(_) => 0,
        }
    }

    fn name(self) -> &'static str {
        match self {
            Card::Item(i) => ITEMS[i].name,
            Card::Nothing(Slot::Armor) => "No Armor",
            Card::Nothing(_) => "Empty Hands",
        }
    }
}

/// A slot's cards. A shirt is always on; armor and tools can come off.
pub fn cards(slot: Slot) -> Vec<Card> {
    let nothing = (slot != Slot::Shirt).then_some(Card::Nothing(slot));
    nothing.into_iter().chain(items::in_slot(slot).map(Card::Item)).collect()
}

/// The most cards any tab has, so the cards stay the same size between tabs.
pub fn max_cards() -> usize {
    SLOTS.iter().map(|&s| cards(s).len()).max().unwrap_or(0)
}

/// The card for what `look` has in `slot`.
fn wearing(look: &Look, slot: Slot) -> Card {
    match look.outfit.get(slot) {
        Some(i) => Card::Item(i),
        None if slot == Slot::Shirt => Card::Item(look.shirt % CLASSIC),
        None => Card::Nothing(slot),
    }
}

/// What the line under the grid says about `card`. Armor also says what it
/// does in boss fights.
fn describe(card: Card, look: &Look, stars: u32) -> String {
    let said = describe_look(card, look, stars);
    match card {
        Card::Item(i) if ITEMS[i].slot == Slot::Armor => {
            format!("{said} In boss fights it {}", items::protection(ITEMS[i].key).describe())
        }
        _ => said,
    }
}

fn describe_look(card: Card, look: &Look, stars: u32) -> String {
    let name = card.name();
    if card.stars() > stars {
        let more = card.stars() - stars;
        let s = if more == 1 { "" } else { "s" };
        format!("{name} unlocks at {} stars. Earn {more} more star{s} to get it!", card.stars())
    } else if wearing(look, slot_of(card)) == card {
        match card {
            Card::Nothing(_) => format!("{name}. Pick something to wear!"),
            Card::Item(_) => format!("Looking good in the {name}!"),
        }
    } else {
        format!("{} to wear the {name}.", if is_touch() { "Tap" } else { "Click" })
    }
}

fn slot_of(card: Card) -> Slot {
    match card {
        Card::Item(i) => ITEMS[i].slot,
        Card::Nothing(slot) => slot,
    }
}

/// The line under the grid when no card is pointed at: what's coming next.
fn next_goal(stars: u32) -> String {
    match items::next_unlock(stars) {
        Some(i) => {
            let more = ITEMS[i].stars - stars;
            let s = if more == 1 { "" } else { "s" };
            format!("Next up: the {} at {} stars. {more} more star{s} to go!", ITEMS[i].name, ITEMS[i].stars)
        }
        None => "You unlocked everything. You're a superstar coder!".to_string(),
    }
}

pub struct Wardrobe {
    tab: usize,
    /// The card last clicked or tapped, described under the grid (touch
    /// screens have nothing to point at).
    picked: Option<Card>,
}

impl Wardrobe {
    pub fn new() -> Wardrobe {
        Wardrobe::with_tab(0)
    }

    /// Opens on tab `tab` (for the screenshot autopilot).
    pub fn with_tab(tab: usize) -> Wardrobe {
        Wardrobe { tab: tab.min(SLOTS.len() - 1), picked: None }
    }

    pub fn frame(&mut self, ui: &mut Ui, art: &Art, sfx: &Sounds, progress: &mut Progress, user: &User, c: Canvas) -> WardrobeOut {
        let tall = layout::is_tall(c);
        let l = layout::wardrobe(c, SLOTS.len(), max_cards());
        let stars = progress.wardrobe_stars();
        let look = progress.look(user.shirt);
        let mut out = WardrobeOut::Stay;

        if ui.button(l.back, "< Levels", rgb8(110, 110, 120), 20.0) {
            sfx.play(Sfx::Click);
            out = WardrobeOut::Back;
        }
        let size = fit_size("Wardrobe", l.title.w, if tall { 32.0 } else { 44.0 }, 20.0);
        shadow_text("Wardrobe", l.title.x, l.title.y + (l.title.h - size) / 2.0 - 2.0, size, WHITE);
        let total = progress.total_stars();
        if tall {
            star(c.0 - 100.0, 32.0, 14.0, true);
            shadow_text(&format!("{total}"), c.0 - 80.0, 20.0, 24.0, rgb8(255, 215, 40));
        } else {
            star(c.0 - 210.0, 38.0, 18.0, true);
            shadow_text(&format!("{total} / {}", items::max_stars()), c.0 - 184.0, 24.0, 30.0, rgb8(255, 215, 40));
        }

        self.preview(art, &look, user, l.preview, tall);

        for (i, &slot) in SLOTS.iter().enumerate() {
            let color = if i == self.tab { rgb8(225, 175, 40) } else { rgb8(80, 80, 95) };
            if ui.button(l.tabs[i], slot.name(), color, if tall { 20.0 } else { 24.0 }) && i != self.tab {
                sfx.play(Sfx::Click);
                self.tab = i;
                self.picked = None;
            }
        }

        let slot = SLOTS[self.tab];
        let worn = wearing(&look, slot);
        let mut hovered = None;
        for (n, &card) in cards(slot).iter().enumerate() {
            let r = l.card_rect(n);
            let unlocked = card.stars() <= stars;
            // The miner in their shirt with just this card on, so it's easy to see.
            let mut try_on = Look { shirt: look.shirt, outfit: Outfit::default() };
            if let Some(shirt) = look.outfit.get(Slot::Shirt) {
                try_on.outfit.wear(shirt);
            }
            card.put_on(&mut try_on.outfit);
            draw_card(art, &try_on, card, r, unlocked, card == worn, ui.hover(r));
            if ui.hover(r) {
                hovered = Some(card);
            }
            if ui.clicked(r) {
                self.picked = Some(card);
                if !unlocked {
                    sfx.play(Sfx::Bonk);
                } else if card != worn {
                    sfx.play(Sfx::Collect);
                    card.put_on(&mut progress.outfit);
                    // Started over in another tab: that tab's progress came in, so wear it again.
                    if progress.save() == Saved::StartedOverElsewhere {
                        card.put_on(&mut progress.outfit);
                        progress.save();
                    }
                }
            }
        }

        let info = match hovered.or(self.picked) {
            Some(card) => describe(card, &progress.look(user.shirt), stars),
            None => next_goal(stars),
        };
        let size = fit_size(&info, l.info.w - 16.0, if tall { 18.0 } else { 24.0 }, 13.0);
        text_centered(&info, l.info, size, WHITE, true);
        out
    }

    /// The miner turning around to show off what it's wearing, with the
    /// player's name and their outfit.
    fn preview(&self, art: &Art, look: &Look, user: &User, r: Rect, tall: bool) {
        draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.0, 0.0, 0.0, 0.55));
        let dir = [Dir::Down, Dir::Right, Dir::Up, Dir::Left][(get_time() / 1.5) as usize % 4];
        let bob = ((get_time() * 3.0).sin() * 3.0) as f32;
        // Room for a tool held out either side as it turns.
        let (miner, text_x, text_y, text_w) = if tall {
            let h = r.h - 30.0;
            let reach = REACH * h / 16.0;
            let text_x = r.x + 2.0 * reach + h * 0.75 + 24.0;
            (Rect::new(r.x + reach + 8.0, r.y + 15.0 + bob, h * 0.75, h), text_x, r.y + 16.0, r.x + r.w - 12.0 - text_x)
        } else {
            let h = (r.h * 0.48).min(300.0).min((r.w - 20.0) / (12.0 + 2.0 * REACH) * 16.0);
            let m = Rect::new(r.x + (r.w - h * 0.75) / 2.0, r.y + 36.0 + bob, h * 0.75, h);
            (m, r.x + 20.0, r.y + 36.0 + h + 34.0, r.w - 40.0)
        };
        art.miner(look, dir, miner, WHITE);
        let name_size = fit_size(&user.name, text_w, if tall { 24.0 } else { 32.0 }, 14.0);
        shadow_text(&user.name, text_x, text_y, name_size, rgb8(120, 230, 220));
        let line = if tall { 22.0 } else { 30.0 };
        let mut y = text_y + name_size + if tall { 10.0 } else { 18.0 };
        for slot in SLOTS {
            let what = wearing(look, slot).name();
            let size = fit_size(what, text_w, if tall { 16.0 } else { 20.0 }, 12.0);
            shadow_text(what, text_x, y, size, WHITE);
            y += line;
        }
    }
}

fn draw_card(art: &Art, look: &Look, card: Card, r: Rect, unlocked: bool, worn: bool, hover: bool) {
    let t = r.w / 150.0;
    let bg = match (unlocked, hover) {
        (false, _) => rgb8(38, 38, 46),
        (true, false) => rgb8(62, 62, 80),
        (true, true) => rgb8(82, 82, 104),
    };
    bevel(r, bg);
    if worn {
        draw_rectangle_lines(r.x, r.y, r.w, r.h, 6.0 * t.max(0.6), rgb8(255, 215, 40));
    }
    // Locked items show dimmed: still clear enough to want.
    let tint = if unlocked { WHITE } else { Color::new(0.5, 0.5, 0.56, 1.0) };
    let h = r.h * 0.56;
    art.miner(look, Dir::Down, Rect::new(r.x + (r.w - h * 0.75) / 2.0, r.y + 10.0 * t, h * 0.75, h), tint);
    let name = card.name();
    let size = fit_size(name, r.w - 12.0, 18.0 * t.max(0.8), 11.0);
    let name_y = r.y + h + 16.0 * t;
    text_centered(name, Rect::new(r.x, name_y, r.w, size + 4.0), size, if unlocked { WHITE } else { LOCKED_TEXT }, unlocked);
    let low = Rect::new(r.x, name_y + size + 6.0 * t, r.w, (r.y + r.h - 6.0 * t) - (name_y + size + 6.0 * t));
    let size = (17.0 * t).clamp(12.0, 17.0);
    if !unlocked {
        // A star and how many it needs.
        let label = format!("{}", card.stars());
        let w = text_width(&label, size) + size * 1.4;
        let x = low.x + (low.w - w) / 2.0;
        star(x + size * 0.5, low.y + low.h / 2.0, size * 0.55, true);
        text(&label, x + size * 1.2, low.y + (low.h - size) / 2.0 - 2.0, size, rgb8(255, 215, 40));
    } else if worn {
        text_centered("Wearing", low, size, rgb8(255, 215, 40), true);
    }
}
