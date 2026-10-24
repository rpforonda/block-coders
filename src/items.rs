//! The Wardrobe's items: shirts, armor and tools that stars unlock for the miner.
//!
//! Stars are never spent. An item unlocks once the player's total stars reach
//! its number, so the star count still says how well they've coded, and the
//! coolest items need the most stars. Unlocks come often early on (one of each
//! kind at 10 stars, then about every 10 up to 60), then spread out, and the
//! last one needs all 216: three stars on every level.
//!
//! Like levels, items are saved by key, not position, so the list can be
//! reordered freely. How each one looks is drawn in `art.rs`.

use crate::levels::LEVELS;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Slot {
    Shirt,
    Armor,
    Tool,
}

pub const SLOTS: [Slot; 3] = [Slot::Shirt, Slot::Armor, Slot::Tool];

impl Slot {
    /// The Wardrobe tab's name.
    pub fn name(self) -> &'static str {
        match self {
            Slot::Shirt => "Shirts",
            Slot::Armor => "Armor",
            Slot::Tool => "Tools",
        }
    }
}

pub struct Item {
    /// Saved in the player's file. Never change one.
    pub key: &'static str,
    pub name: &'static str,
    pub slot: Slot,
    /// Total stars needed to unlock it.
    pub stars: u32,
}

const fn item(key: &'static str, name: &'static str, slot: Slot, stars: u32) -> Item {
    Item { key, name, slot, stars }
}

/// The first `CLASSIC` items are the plain shirt colors players pick from when
/// they join, in `art::SHIRTS` order.
pub const CLASSIC: usize = 8;

pub static ITEMS: &[Item] = &[
    item("teal", "Teal Shirt", Slot::Shirt, 0),
    item("red", "Red Shirt", Slot::Shirt, 0),
    item("green", "Green Shirt", Slot::Shirt, 0),
    item("orange", "Orange Shirt", Slot::Shirt, 0),
    item("purple", "Purple Shirt", Slot::Shirt, 0),
    item("pink", "Pink Shirt", Slot::Shirt, 0),
    item("blue", "Blue Shirt", Slot::Shirt, 0),
    item("white", "White Shirt", Slot::Shirt, 0),
    item("stripes", "Sailor Stripes", Slot::Shirt, 10),
    item("leather", "Leather Armor", Slot::Armor, 10),
    item("torch", "Torch", Slot::Tool, 10),
    item("chainmail", "Chainmail", Slot::Armor, 20),
    item("lantern", "Lantern", Slot::Tool, 25),
    item("plaid", "Lumberjack Plaid", Slot::Shirt, 30),
    item("iron", "Iron Armor", Slot::Armor, 40),
    item("gold-pick", "Golden Pickaxe", Slot::Tool, 50),
    item("gold", "Gold Armor", Slot::Armor, 60),
    item("galaxy", "Galaxy Shirt", Slot::Shirt, 75),
    item("diamond-pick", "Diamond Pickaxe", Slot::Tool, 90),
    item("diamond", "Diamond Armor", Slot::Armor, 110),
    item("wand", "Magic Wand", Slot::Tool, 130),
    item("netherite", "Netherite Armor", Slot::Armor, 150),
    item("rainbow", "Rainbow Shirt", Slot::Shirt, 170),
    item("crystal-staff", "Crystal Staff", Slot::Tool, 185),
    item("superstar", "Superstar Armor", Slot::Armor, 216),
];

/// Every star there is: three on every level.
pub fn max_stars() -> u32 {
    LEVELS.len() as u32 * 3
}

/// The items of one slot, in the order the Wardrobe shows them.
pub fn in_slot(slot: Slot) -> impl Iterator<Item = usize> {
    (0..ITEMS.len()).filter(move |&i| ITEMS[i].slot == slot)
}

/// Items that going from `before` to `after` stars unlocked.
pub fn newly_unlocked(before: u32, after: u32) -> Vec<usize> {
    (0..ITEMS.len()).filter(|&i| ITEMS[i].stars > before && ITEMS[i].stars <= after).collect()
}

/// The next item still to unlock with `stars`, if any.
pub fn next_unlock(stars: u32) -> Option<usize> {
    (0..ITEMS.len()).filter(|&i| ITEMS[i].stars > stars).min_by_key(|&i| ITEMS[i].stars)
}

/// What a player chose in the Wardrobe: at most one item per slot (indexes
/// into `ITEMS`). No shirt means the color they picked when they joined.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Outfit([Option<usize>; 3]);

impl Outfit {
    fn at(slot: Slot) -> usize {
        SLOTS.iter().position(|&s| s == slot).unwrap_or(0)
    }

    pub fn get(&self, slot: Slot) -> Option<usize> {
        self.0[Outfit::at(slot)]
    }

    /// Puts on an item, in place of whatever was in its slot.
    pub fn wear(&mut self, item: usize) {
        self.0[Outfit::at(ITEMS[item].slot)] = Some(item);
    }

    pub fn take_off(&mut self, slot: Slot) {
        self.0[Outfit::at(slot)] = None;
    }

    /// The worn items' keys, for the save file.
    pub fn serialize(&self) -> String {
        self.0.iter().flatten().map(|&i| ITEMS[i].key).collect::<Vec<_>>().join(",")
    }

    /// Reads `serialize`'s output. Unknown keys (from an edited save, or an item
    /// since removed) are skipped.
    pub fn parse(s: &str) -> Outfit {
        let mut o = Outfit::default();
        for key in s.split(',') {
            if let Some(i) = ITEMS.iter().position(|it| it.key == key.trim()) {
                o.wear(i);
            }
        }
        o
    }

    /// Just the items `stars` unlocks. After "Start over" the rest stay chosen
    /// but hidden, and come back as the stars are earned again.
    pub fn unlocked(&self, stars: u32) -> Outfit {
        Outfit(self.0.map(|i| i.filter(|&i| ITEMS[i].stars <= stars)))
    }
}

/// Everything needed to draw a player's miner.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub struct Look {
    /// The shirt color picked when they joined (see `art::SHIRTS`).
    pub shirt: usize,
    /// Only unlocked items.
    pub outfit: Outfit,
}

impl Look {
    /// A miner in shirt color `shirt` and nothing else.
    #[cfg(test)]
    pub fn plain(shirt: usize) -> Look {
        Look { shirt, outfit: Outfit::default() }
    }

    /// The plain shirt color the body is drawn in: a classic shirt from the
    /// Wardrobe, or else the one picked when they joined.
    pub fn base_shirt(&self) -> usize {
        self.outfit.get(Slot::Shirt).filter(|&i| i < CLASSIC).unwrap_or(self.shirt)
    }

    /// The worn items with their own art (patterned shirts, armor, tools).
    pub fn extras(&self) -> impl Iterator<Item = usize> + '_ {
        SLOTS.iter().filter_map(|&s| self.outfit.get(s)).filter(|&i| i >= CLASSIC)
    }
}

/// What armor does in boss fights (see `fight.rs`). Rarer armor protects
/// more, but even the best blocks only a few mistakes, so the miner can still
/// be knocked out.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct Protection {
    /// Mistakes it blocks per fight before any hearts are lost.
    pub points: u8,
    /// Half-hearts back after each cleared stage.
    pub heal: u8,
    /// Lava costs no hearts.
    pub lava: bool,
    /// Water costs no hearts.
    pub water: bool,
}

/// The protection of the armor item with `key` (nothing for shirts and tools).
/// The ladder was picked with TypeSafe (see docs/DESIGN.md).
pub fn protection(key: &str) -> Protection {
    let p = |points, heal, lava, water| Protection { points, heal, lava, water };
    match key {
        "leather" => p(1, 0, false, false),
        "chainmail" => p(2, 0, false, false),
        "iron" => p(2, 1, false, false),
        "gold" => p(3, 1, false, false),
        "diamond" => p(3, 2, false, false),
        "netherite" => p(4, 2, true, false),
        "superstar" => p(4, 2, true, true),
        _ => Protection::default(),
    }
}

impl Protection {
    /// What it does, in words: "blocks 2 mistakes and gives half a heart back
    /// after each stage."
    pub fn describe(&self) -> String {
        let mut s = format!("blocks {} mistake{}", self.points, if self.points == 1 { "" } else { "s" });
        match self.heal {
            0 => {}
            1 => s.push_str(" and gives half a heart back after each stage"),
            _ => s.push_str(" and gives a heart back after each stage"),
        }
        s.push('.');
        match (self.lava, self.water) {
            (true, true) => s.push_str(" Lava and water can't hurt you."),
            (true, false) => s.push_str(" Lava can't hurt you."),
            (false, true) => s.push_str(" Water can't hurt you."),
            (false, false) => {}
        }
        s
    }
}

impl Look {
    /// What the worn armor does in boss fights.
    pub fn protection(&self) -> Protection {
        self.outfit.get(Slot::Armor).map_or(Protection::default(), |i| protection(ITEMS[i].key))
    }
}
