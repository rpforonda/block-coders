//! Boss fights: the miner's hearts and armor, the boss's health, and the
//! stars for winning. Kept apart from the screen so tests can play a whole
//! fight.
//!
//! - A fight has 3 stages. Clearing one (breaking all its crystals) takes a
//!   third of the boss's health.
//! - The miner has 5 hearts. A run where the miner gets hurt (bumps into
//!   something, steps in lava or water, mines thin air, the program goes
//!   wrong, or a win only worked by luck) costs half a heart, at most once
//!   per run. A program that just stops before it's finished costs nothing,
//!   so testing part of a program is always safe.
//! - Armor from the Wardrobe protects (see `items::protection`): each armor
//!   point blocks one mistake before any hearts are lost, rarer armor gives
//!   hearts back after each cleared stage, and the rarest makes lava (and
//!   water) harmless. Armor is repaired after each fight; points used before
//!   a knockout stay used.
//! - At zero hearts the miner is knocked out and tries the same stage again
//!   with full hearts. Stages already cleared stay cleared.
//! - Stars come from the hearts lost in the whole fight. Mistakes the armor
//!   blocked don't count.

use crate::items::Protection;
use crate::levels::{Boss, Level};

/// Half-hearts the miner has in a boss fight: 5 hearts.
pub const MAX_HEARTS: u8 = 10;
/// Thirds of health a boss has: one per stage.
pub const STAGES: usize = 3;

/// What hurt the miner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Lava,
    Water,
    /// Anything else: a bump, thin air, a program error, a lucky win.
    Other,
}

/// What getting hurt did.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Hurt {
    /// This run already cost something.
    Already,
    /// The armor makes this harmless (lava or water).
    Immune,
    /// An armor point blocked it.
    Blocked,
    /// Lost half a heart.
    Heart,
    /// Lost the last half heart.
    KnockedOut,
}

pub struct Fight {
    pub boss: &'static Boss,
    /// What the worn armor does.
    pub protection: Protection,
    /// The stage being fought: 0, 1 or 2.
    pub stage: usize,
    /// Half-hearts left.
    pub hearts: u8,
    /// Armor points left this fight.
    pub armor: u8,
    /// Half-hearts lost in the whole fight, knockouts included.
    pub lost: u32,
    /// How many times the miner was knocked out.
    pub knockouts: u32,
    hurt_this_run: bool,
}

impl Fight {
    pub fn new(boss: &'static Boss, protection: Protection) -> Fight {
        Fight {
            boss,
            protection,
            stage: 0,
            hearts: MAX_HEARTS,
            armor: protection.points,
            lost: 0,
            knockouts: 0,
            hurt_this_run: false,
        }
    }

    /// The stage being fought, played like any other level.
    pub fn level(&self) -> &'static Level {
        &self.boss.stages[self.stage.min(STAGES - 1)]
    }

    /// A new run starts, so it can cost something again.
    pub fn start_run(&mut self) {
        self.hurt_this_run = false;
    }

    /// The miner got hurt during this run.
    pub fn hurt(&mut self, cause: Cause) -> Hurt {
        if self.hurt_this_run || self.hearts == 0 {
            return Hurt::Already;
        }
        self.hurt_this_run = true;
        let p = self.protection;
        if (cause == Cause::Lava && p.lava) || (cause == Cause::Water && p.water) {
            return Hurt::Immune;
        }
        if self.armor > 0 {
            self.armor -= 1;
            return Hurt::Blocked;
        }
        self.hearts -= 1;
        self.lost += 1;
        if self.hearts == 0 { Hurt::KnockedOut } else { Hurt::Heart }
    }

    /// After a knockout: full hearts, same stage. Used armor stays used.
    pub fn get_up(&mut self) {
        self.hearts = MAX_HEARTS;
        self.knockouts += 1;
        self.hurt_this_run = false;
    }

    /// The stage's crystals are all broken: the boss loses a third of its
    /// health, and good armor gives some hearts back. Returns whether that
    /// defeated the boss.
    pub fn clear_stage(&mut self) -> bool {
        self.stage = (self.stage + 1).min(STAGES);
        self.hurt_this_run = false;
        if !self.defeated() {
            self.hearts = (self.hearts + self.protection.heal).min(MAX_HEARTS);
        }
        self.defeated()
    }

    pub fn defeated(&self) -> bool {
        self.stage >= STAGES
    }

    /// How many thirds of its health the boss has left.
    pub fn boss_health(&self) -> usize {
        STAGES - self.stage.min(STAGES)
    }

    /// Stars for winning: 3 for losing at most 1 heart, 2 for at most half
    /// the hearts (rounded down), otherwise 1.
    pub fn stars(&self) -> u8 {
        if self.lost <= 2 {
            3
        } else if self.lost <= u32::from(MAX_HEARTS) / 2 {
            2
        } else {
            1
        }
    }
}
