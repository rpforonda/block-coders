//! Player profiles. Each player has their own stars and saved programs, so
//! kids can share the game and pick up where they left off.
//!
//! Files (in ~/.local/share/block-coders/):
//!   users.txt          one line per player: id ; shirt colour ; name
//!   user-<id>.txt      that player's progress: level name ; stars ; program

use crate::levels::{LEGACY_ORDER, LEVELS};
use crate::program::Program;
use std::path::PathBuf;

pub const MAX_USERS: usize = 6;
pub const MAX_NAME: usize = 12;

fn data_dir() -> Option<PathBuf> {
    let base = std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|h| PathBuf::from(h).join(".local/share")))
        .or_else(|| std::env::var_os("APPDATA").map(PathBuf::from))?;
    Some(base.join("block-coders"))
}

fn write(name: &str, contents: &str) {
    let Some(dir) = data_dir() else { return };
    let _ = std::fs::create_dir_all(&dir);
    let _ = std::fs::write(dir.join(name), contents);
}

fn read(name: &str) -> Option<String> {
    std::fs::read_to_string(data_dir()?.join(name)).ok()
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
}

impl Users {
    pub fn load() -> Users {
        let mut users = Users { list: Vec::new() };
        match read("users.txt") {
            Some(text) => {
                for line in text.lines() {
                    let mut parts = line.splitn(3, ';');
                    let (Some(id), Some(shirt), Some(name)) = (parts.next(), parts.next(), parts.next()) else {
                        continue;
                    };
                    let (Ok(id), Ok(shirt)) = (id.parse(), shirt.parse()) else { continue };
                    users.list.push(User { id, name: name.to_string(), shirt });
                }
            }
            None => {
                // Older versions had a single shared save; keep it as the first player.
                if let Some(old) = read("progress.txt") {
                    write(&user_file(1), &old);
                    users.list.push(User { id: 1, name: "Player 1".into(), shirt: 0 });
                    users.save();
                }
            }
        }
        users
    }

    fn save(&self) {
        let text: String = self.list.iter().map(|u| format!("{};{};{}\n", u.id, u.shirt, u.name)).collect();
        write("users.txt", &text);
    }

    pub fn add(&mut self, name: &str, shirt: usize) -> u32 {
        let id = self.list.iter().map(|u| u.id).max().unwrap_or(0) + 1;
        self.list.push(User { id, name: name.to_string(), shirt });
        self.save();
        // Make sure no stale file from an old player with this id is picked up.
        Progress::new(Some(id)).save();
        id
    }

    pub fn remove(&mut self, id: u32) {
        self.list.retain(|u| u.id != id);
        self.save();
        if let Some(dir) = data_dir() {
            let _ = std::fs::remove_file(dir.join(user_file(id)));
        }
    }
}

pub struct Progress {
    /// None = a guest whose progress is never saved.
    pub user: Option<u32>,
    pub stars: Vec<u8>,
    pub programs: Vec<Option<Program>>,
    pub unlock_all: bool,
}

impl Progress {
    pub fn new(user: Option<u32>) -> Progress {
        let n = LEVELS.len();
        Progress { user, stars: vec![0; n], programs: vec![None; n], unlock_all: false }
    }

    pub fn load(user: u32) -> Progress {
        let mut p = Progress::new(Some(user));
        let Some(text) = read(&user_file(user)) else { return p };
        for line in text.lines() {
            let mut parts = line.splitn(3, ';');
            let (Some(key), Some(s), Some(prog)) = (parts.next(), parts.next(), parts.next()) else {
                continue;
            };
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
        p
    }

    pub fn save(&self) {
        let Some(user) = self.user else { return };
        let mut out = String::new();
        for (i, level) in LEVELS.iter().enumerate() {
            let prog = self.programs[i].as_ref().map(|p| p.serialize()).unwrap_or_default();
            out.push_str(&format!("{};{};{prog}\n", level.name, self.stars[i]));
        }
        write(&user_file(user), &out);
    }

    /// Wipes all stars and programs so the player starts from level 1 again.
    pub fn reset(&mut self) {
        *self = Progress::new(self.user);
        self.save();
    }

    /// Everything up to (and one past) the furthest level beaten is open. So new
    /// levels added before a player's furthest level never lock them out.
    pub fn unlocked(&self, i: usize) -> bool {
        let furthest = self.stars.iter().rposition(|&s| s > 0);
        self.unlock_all || i == 0 || furthest.is_some_and(|f| i <= f + 1)
    }

    pub fn total_stars(&self) -> u32 {
        self.stars.iter().map(|&s| s as u32).sum()
    }

    /// How many levels have been completed.
    pub fn levels_done(&self) -> usize {
        self.stars.iter().filter(|&&s| s > 0).count()
    }
}
