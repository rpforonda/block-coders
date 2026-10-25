//! Player profiles. Each player has their own stars and saved programs, so
//! kids can share the game and pick up where they left off.
//!
//! Two entries, stored as files on desktop (in ~/.local/share/block-coders/) and
//! in the browser's localStorage on the web (see `storage` below):
//!   users.txt          one line per player: id ; shirt colour ; name
//!                      plus "next;<id>", the next id to hand out
//!   user-<id>.txt      that player's progress: level name ; stars ; program
//!                      plus "gen;<n>", how many times they've started over,
//!                      and "look;<item keys>", what they wear (see `items.rs`)
//!
//! Two browser tabs (or game windows) can have the same saves open at once, so
//! every write re-reads what's stored and merges into it rather than writing
//! back a stale copy. See `Users::add` and `Progress::save`.
//!
//! Everything read back is treated as untrusted. On the web, anyone can edit
//! localStorage from the browser's dev tools, so loading validates and clamps
//! every field and never panics on bad data.

use crate::check::CHECKS;
use crate::items::{Look, Outfit};
use crate::levels::{CHAPTERS, LEGACY_ORDER, LEVELS};
use crate::questions::BANKS;
use crate::program::Program;
use std::sync::atomic::{AtomicBool, Ordering};
use storage::read;

pub const MAX_USERS: usize = 6;
/// How many World Checks there were before the Counting Caverns world.
const CHECKS_BEFORE_VARIABLES: usize = 5;
pub const MAX_NAME: usize = 12;
/// Saves larger than this are ignored. Real ones are a few KB.
pub const MAX_SAVE_BYTES: usize = 256 * 1024;

/// Desktop: one small text file per entry.
#[cfg(not(target_arch = "wasm32"))]
mod storage {
    use std::path::PathBuf;

    fn data_dir() -> Option<PathBuf> {
        let base = std::env::var_os("XDG_DATA_HOME")
            .map(PathBuf::from)
            .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
            .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
        Some(base.join("block-coders"))
    }

    /// Returns whether the save was written.
    pub fn write(name: &str, contents: &str) -> bool {
        let Some(dir) = data_dir() else { return false };
        if std::fs::create_dir_all(&dir).is_err() {
            return false;
        }
        // Saves hold children's names, so keep them private to this account.
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o700));
        }
        // Write a temporary file, then rename it over the old one, so a crash or
        // power cut mid-save leaves either the old save or the new one, never half.
        let tmp = dir.join(format!("{name}.tmp"));
        let ok = std::fs::write(&tmp, contents).and_then(|_| std::fs::rename(&tmp, dir.join(name))).is_ok();
        if !ok {
            let _ = std::fs::remove_file(&tmp);
        }
        ok
    }

    pub fn read(name: &str) -> Option<String> {
        let path = data_dir()?.join(name);
        if std::fs::metadata(&path).ok()?.len() > super::MAX_SAVE_BYTES as u64 {
            return None;
        }
        std::fs::read_to_string(path).ok()
    }

    pub fn remove(name: &str) {
        if let Some(dir) = data_dir() {
            let _ = std::fs::remove_file(dir.join(name));
        }
    }
}

/// Web: the browser's localStorage, through the tiny JS plugin in web/storage.js.
/// Data never leaves the player's device. Keys are prefixed with "block-coders/"
/// on the JS side so they can't collide with other pages on the same origin.
#[cfg(target_arch = "wasm32")]
mod storage {
    unsafe extern "C" {
        /// Copies up to `buf_len` bytes of the value into `buf` and returns the
        /// value's full length in bytes, or -1 if missing or unavailable.
        fn bc_storage_get(key: *const u8, key_len: usize, buf: *mut u8, buf_len: usize) -> i32;
        /// Returns 1 on success, 0 on failure (storage full or disabled).
        fn bc_storage_set(key: *const u8, key_len: usize, val: *const u8, val_len: usize) -> i32;
        fn bc_storage_remove(key: *const u8, key_len: usize);
    }

    /// miniquad compares this with the plugin's version on startup.
    #[unsafe(no_mangle)]
    pub extern "C" fn bc_storage_crate_version() -> u32 {
        // 2: the plugin also answers bc_touch_screen (see keyboard.rs).
        2
    }

    /// Returns whether the save was written.
    pub fn write(name: &str, contents: &str) -> bool {
        if contents.len() > super::MAX_SAVE_BYTES {
            return false;
        }
        // SAFETY: both pointers come from live &str values with their exact
        // lengths, and the JS side only reads that many bytes.
        unsafe { bc_storage_set(name.as_ptr(), name.len(), contents.as_ptr(), contents.len()) == 1 }
    }

    pub fn read(name: &str) -> Option<String> {
        // Ask for the length first, then copy into a buffer of exactly that size.
        // SAFETY: a zero-length buffer is never written to.
        let len = unsafe { bc_storage_get(name.as_ptr(), name.len(), std::ptr::null_mut(), 0) };
        let len = usize::try_from(len).ok().filter(|&l| l <= super::MAX_SAVE_BYTES)?;
        let mut buf = vec![0u8; len];
        // SAFETY: `buf` is valid for `len` bytes, and JS writes at most `buf_len` bytes.
        let got = unsafe { bc_storage_get(name.as_ptr(), name.len(), buf.as_mut_ptr(), buf.len()) };
        if usize::try_from(got).ok()? != len {
            return None; // changed in between (another tab); treat as unreadable this time
        }
        String::from_utf8(buf).ok()
    }

    pub fn remove(name: &str) {
        // SAFETY: pointer and length come from a live &str.
        unsafe { bc_storage_remove(name.as_ptr(), name.len()) };
    }
}

/// Set while the screenshot autopilot runs: it shows the real players, but its
/// popups and runs must never change their saves (a stray real click on its
/// "Start over" popup would otherwise wipe a player).
static READ_ONLY: AtomicBool = AtomicBool::new(false);

pub fn set_read_only(on: bool) {
    READ_ONLY.store(on, Ordering::Relaxed);
}

/// Returns whether the save was written (never while read-only).
fn write(name: &str, contents: &str) -> bool {
    !READ_ONLY.load(Ordering::Relaxed) && storage::write(name, contents)
}

fn remove(name: &str) {
    if !READ_ONLY.load(Ordering::Relaxed) {
        storage::remove(name);
    }
}

/// Applies the same rules as typing a name in the New Player dialog, so a
/// hand-edited save can't smuggle in odd characters or overlong names.
pub fn clean_name(raw: &str) -> String {
    let name: String = raw
        .chars()
        .filter(|&c| c.is_alphanumeric() || c == ' ' || c == '-' || c == '_')
        .take(MAX_NAME)
        .collect();
    let name = name.trim();
    if name.is_empty() { "Player".to_string() } else { name.to_string() }
}

fn user_file(id: u32) -> String {
    format!("user-{id}.txt")
}

#[derive(Clone)]
pub struct User {
    pub id: u32,
    pub name: String,
    pub shirt: usize,
}

pub struct Users {
    pub list: Vec<User>,
    /// Ids are never reused, so a stale tab still playing a deleted player
    /// can't write their progress into a new player who got the same id.
    next_id: u32,
}

/// Ids beyond this in a save are treated as junk rather than trusted.
const MAX_ID: u32 = 1_000_000;

impl Users {
    pub fn load() -> Users {
        let mut users = Users { list: Vec::new(), next_id: 1 };
        match read("users.txt") {
            Some(text) => {
                for line in text.lines() {
                    if let Some(next) = line.strip_prefix("next;") {
                        if let Some(next) = next.trim().parse::<u32>().ok().filter(|&n| n <= MAX_ID) {
                            users.next_id = users.next_id.max(next);
                        }
                        continue;
                    }
                    let mut parts = line.splitn(3, ';');
                    let (Some(id), Some(shirt), Some(name)) = (parts.next(), parts.next(), parts.next()) else {
                        continue;
                    };
                    let (Ok(id), Ok(shirt)) = (id.trim().parse::<u32>(), shirt.trim().parse::<usize>()) else { continue };
                    // id 0 is the guest; ids must be unique; never more than MAX_USERS.
                    if id == 0 || id > MAX_ID || users.list.iter().any(|u| u.id == id) || users.list.len() >= MAX_USERS {
                        continue;
                    }
                    let shirt = shirt % crate::art::SHIRTS.len();
                    users.list.push(User { id, name: clean_name(name), shirt });
                }
            }
            None => {
                // Older versions had a single shared save; keep it as the first player.
                if let Some(old) = read("progress.txt") {
                    let _ = write(&user_file(1), &old);
                    users.list.push(User { id: 1, name: "Player 1".into(), shirt: 0 });
                    users.next_id = 2;
                    users.save();
                }
            }
        }
        // Saves from before "next;" only have the ids themselves to go on.
        users.next_id = users.next_id.max(users.list.iter().map(|u| u.id + 1).max().unwrap_or(1));
        users
    }

    fn save(&self) {
        let mut text: String = self.list.iter().map(|u| format!("{};{};{}\n", u.id, u.shirt, u.name)).collect();
        text.push_str(&format!("next;{}\n", self.next_id));
        let _ = write("users.txt", &text);
    }

    /// Is this player still in the stored list? Another tab may have deleted them.
    fn listed(id: u32) -> bool {
        Users::load().list.iter().any(|u| u.id == id)
    }

    /// Adds a player and returns their id, or None if the list is full or the
    /// name is taken. Re-reads the list first: another browser tab (or game
    /// window) may have added players since this one loaded it, and writing
    /// back an old copy would drop them and reuse their ids.
    pub fn add(&mut self, name: &str, shirt: usize) -> Option<u32> {
        *self = Users::load();
        if self.list.len() >= MAX_USERS || self.list.iter().any(|u| u.name.eq_ignore_ascii_case(name)) {
            return None;
        }
        // Never reuse an id, or one that still has a save this list doesn't know.
        let mut id = self.next_id;
        while self.list.iter().any(|u| u.id == id) || read(&user_file(id)).is_some() {
            id += 1;
        }
        if id > MAX_ID {
            return None;
        }
        self.next_id = id + 1;
        self.list.push(User { id, name: name.to_string(), shirt });
        self.save();
        Progress::new(Some(id)).save();
        Some(id)
    }

    pub fn remove(&mut self, id: u32) {
        *self = Users::load(); // keep players other tabs have added
        self.list.retain(|u| u.id != id);
        self.save();
        remove(&user_file(id));
    }
}

/// What happened to a `Progress::save`.
#[derive(Debug, PartialEq)]
pub enum Saved {
    /// Saved, or skipped (a guest, a player deleted elsewhere, storage failing).
    Done,
    /// The player started over in another tab or window since this copy was
    /// loaded. Nothing was written, and this copy now holds the stored
    /// progress, so a screen showing one of its programs must show that instead.
    StartedOverElsewhere,
}

pub struct Progress {
    /// None = a guest whose progress is never saved.
    pub user: Option<u32>,
    pub stars: Vec<u8>,
    pub programs: Vec<Option<Program>>,
    pub unlock_all: bool,
    /// Which World Checks are passed (one per world except the last).
    pub checks: Vec<bool>,
    /// The question ids of each check's most recent attempt, so a retry can
    /// pick different ones.
    pub check_last: Vec<Vec<String>>,
    /// What they chose in the Wardrobe, including items locked again after
    /// "Start over" (see `look`).
    pub outfit: Outfit,
    /// Bumped by every "Start over", so a tab that loaded the player before a
    /// reset elsewhere knows its stars are out of date.
    generation: u32,
    /// Programs and check questions as last loaded or saved, to tell which
    /// ones this tab changed (and so should win) when merging on save.
    base_programs: Vec<String>,
    base_check_last: Vec<Vec<String>>,
    base_outfit: Outfit,
}

impl Progress {
    pub fn new(user: Option<u32>) -> Progress {
        let n = LEVELS.len();
        Progress {
            user,
            stars: vec![0; n],
            programs: vec![None; n],
            unlock_all: false,
            checks: vec![false; CHECKS],
            check_last: vec![Vec::new(); CHECKS],
            outfit: Outfit::default(),
            generation: 0,
            base_programs: vec![String::new(); n],
            base_check_last: vec![Vec::new(); CHECKS],
            base_outfit: Outfit::default(),
        }
    }

    fn program_text(&self, i: usize) -> String {
        self.programs[i].as_ref().map(|p| p.serialize()).unwrap_or_default()
    }

    /// Remembers the current programs and questions as the merge baseline.
    fn rebase(&mut self) {
        self.base_programs = (0..LEVELS.len()).map(|i| self.program_text(i)).collect();
        self.base_check_last = self.check_last.clone();
        self.base_outfit = self.outfit;
    }

    pub fn load(user: u32) -> Progress {
        let mut p = Progress::new(Some(user));
        let Some(text) = read(&user_file(user)) else { return p };
        for line in text.lines() {
            if let Some(g) = line.strip_prefix("gen;") {
                p.generation = g.trim_end_matches(';').trim().parse().unwrap_or(0);
                continue;
            }
            if let Some(keys) = line.strip_prefix("look;") {
                p.outfit = Outfit::parse(keys);
                continue;
            }
            let mut parts = line.splitn(3, ';');
            let (Some(key), Some(s), Some(prog)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
            // World Checks: "check:<n>;<passed 0/1>;<last question ids>".
            if let Some(c) = key.strip_prefix("check:") {
                let Some(c) = c.trim().parse::<usize>().ok().filter(|&c| c < CHECKS) else { continue };
                p.checks[c] = s.trim() == "1";
                p.check_last[c] = prog
                    .split(',')
                    .filter(|id| BANKS[c].iter().any(|q| q.id == *id))
                    .map(str::to_string)
                    .collect();
                continue;
            }
            // Levels are saved by name. Old saves used the level's number back
            // when there were only 20 levels.
            let name = match key.trim().parse::<usize>() {
                Ok(i) => LEGACY_ORDER.get(i).copied().unwrap_or_default(),
                Err(_) => key,
            };
            let Some(i) = LEVELS.iter().position(|l| l.name == name) else { continue };
            p.stars[i] = s.trim().parse::<u8>().unwrap_or(0).min(3);
            p.programs[i] = Program::deserialize(prog).filter(|pr| !pr.is_empty());
        }
        p.rebase();
        p
    }

    /// Saves this player, merged with whatever another tab or window saved for
    /// them since this one loaded: stars and passed checks only ever go up, and
    /// programs this tab didn't touch keep the stored version. If the player
    /// started over or was deleted elsewhere, nothing is written and this copy
    /// is brought up to date instead, so a stale tab can't undo either.
    pub fn save(&mut self) -> Saved {
        let Some(user) = self.user else { return Saved::Done };
        if !Users::listed(user) {
            return Saved::Done;
        }
        let stored = Progress::load(user);
        if stored.generation != self.generation {
            let unlock_all = self.unlock_all;
            *self = stored;
            self.unlock_all = unlock_all;
            return Saved::StartedOverElsewhere;
        }
        for i in 0..LEVELS.len() {
            self.stars[i] = self.stars[i].max(stored.stars[i]);
            if self.program_text(i) == self.base_programs[i] {
                self.programs[i] = stored.programs[i].clone();
            }
        }
        for c in 0..CHECKS {
            self.checks[c] |= stored.checks[c];
            if self.check_last[c] == self.base_check_last[c] {
                self.check_last[c] = stored.check_last[c].clone();
            }
        }
        if self.outfit == self.base_outfit {
            self.outfit = stored.outfit;
        }
        self.write_out(user);
        Saved::Done
    }

    /// Writes this copy as it is. The merge baseline only moves on when the
    /// write worked: otherwise the next save would think this tab's unsaved
    /// changes were already stored and take the older stored programs instead.
    fn write_out(&mut self, user: u32) -> bool {
        let mut out = format!("gen;{}\nlook;{}\n", self.generation, self.outfit.serialize());
        for (i, level) in LEVELS.iter().enumerate() {
            out.push_str(&format!("{};{};{}\n", level.name, self.stars[i], self.program_text(i)));
        }
        for c in 0..CHECKS {
            out.push_str(&format!("check:{c};{};{}\n", self.checks[c] as u8, self.check_last[c].join(",")));
        }
        let ok = write(&user_file(user), &out);
        if ok {
            self.rebase();
        }
        ok
    }

    /// Wipes all stars and programs so the player starts from level 1 again.
    /// The outfit stays, and its items come back as the stars are earned again.
    pub fn reset(&mut self) {
        let mut outfit = self.outfit;
        let changed = self.outfit != self.base_outfit;
        *self = Progress::new(self.user);
        self.outfit = outfit;
        let Some(user) = self.user else { return };
        if Users::listed(user) {
            let stored = Progress::load(user);
            if !changed {
                outfit = stored.outfit;
            }
            self.outfit = outfit;
            self.generation = stored.generation.wrapping_add(1);
            if !self.write_out(user) {
                *self = Progress::load(user); // didn't happen; don't pretend it did
            }
        }
    }

    /// Everything up to (and one past) the furthest level beaten is open, as
    /// long as its world is open. So new levels added before a player's
    /// furthest level never lock them out. A world's first level opens with
    /// the world, so passing a World Check early opens the next world.
    pub fn unlocked(&self, i: usize) -> bool {
        let furthest = self.stars.iter().rposition(|&s| s > 0);
        let first_of_world = i == 0 || LEVELS[i - 1].chapter != LEVELS[i].chapter;
        let reached = first_of_world || furthest.is_some_and(|f| i <= f + 1);
        self.unlock_all || (reached && self.world_open(LEVELS[i].chapter))
    }

    /// Has the player beaten any level in world `c` or a later one?
    fn reached_world(&self, c: usize) -> bool {
        LEVELS.iter().zip(&self.stars).any(|(l, &s)| s > 0 && l.chapter >= c)
    }

    /// Is world `c` open? Each world needs the World Check before it, except
    /// that saves from before World Checks keep the worlds they'd already
    /// reached. The last world needs every check passed, except that players
    /// who were already in it before the Counting Caverns world was added
    /// (and had passed every check there was then) keep it.
    pub fn world_open(&self, c: usize) -> bool {
        if self.unlock_all || c == 0 {
            true
        } else if c == CHAPTERS.len() - 1 {
            self.checks.iter().all(|&p| p) || (self.reached_world(c) && self.checks[..CHECKS_BEFORE_VARIABLES].iter().all(|&p| p))
        } else {
            self.checks[c - 1] || self.reached_world(c)
        }
    }

    /// Can the player take World Check `c`? As soon as its world is open, so
    /// a player who already knows the idea can skip ahead.
    pub fn check_open(&self, c: usize) -> bool {
        self.world_open(c)
    }

    /// Has the player got to the end of world `c`: beaten its last level (or
    /// got past the world some other way)? Its World Check before then is
    /// taken early.
    pub fn finished_world(&self, c: usize) -> bool {
        let last = LEVELS.iter().rposition(|l| l.chapter == c);
        last.is_some_and(|i| self.stars[i] > 0) || self.reached_world(c + 1)
    }

    pub fn total_stars(&self) -> u32 {
        self.stars.iter().map(|&s| s as u32).sum()
    }

    /// The stars that count in the Wardrobe (all of them with the unlock-all key).
    pub fn wardrobe_stars(&self) -> u32 {
        if self.unlock_all { crate::items::max_stars() } else { self.total_stars() }
    }

    /// How their miner looks: shirt color `shirt` (picked when they joined)
    /// plus whatever Wardrobe items they've unlocked and are wearing.
    pub fn look(&self, shirt: usize) -> Look {
        Look { shirt, outfit: self.outfit.unlocked(self.wardrobe_stars()) }
    }

    /// How many levels have been completed.
    pub fn levels_done(&self) -> usize {
        self.stars.iter().filter(|&&s| s > 0).count()
    }
}
