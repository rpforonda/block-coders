use crate::levels::LEVELS;
use crate::program::{kinds_used, simulate, Program};
use crate::program::{Step, Vm};
use crate::world::{Outcome, World};

fn level(name: &str) -> &'static crate::levels::Level {
    LEVELS.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no level {name}"))
}

#[test]
fn level_list_is_sane() {
    assert_eq!(LEVELS.len(), 60);
    let mut names: Vec<&str> = LEVELS.iter().map(|l| l.name).collect();
    assert!(names.iter().all(|n| !n.contains(';') && !n.is_empty()));
    names.sort();
    names.dedup();
    assert_eq!(names.len(), LEVELS.len(), "level names must be unique (they are save keys)");
    for w in LEVELS.windows(2) {
        assert!(w[1].chapter >= w[0].chapter && w[1].chapter <= w[0].chapter + 1, "{}: chapters out of order", w[1].name);
    }
    for (i, l) in LEVELS.iter().enumerate() {
        if i == 0 || LEVELS[i - 1].chapter != l.chapter {
            assert!(l.intro.is_some(), "{}: first level of a world needs an intro", l.name);
        }
        assert!(!l.hint.is_empty() && !l.lesson.is_empty(), "{}: needs a hint and lesson", l.name);
    }
    for old in crate::levels::LEGACY_ORDER {
        level(old);
    }
}

/// Every check a solution uses must be one the kid can pick in that level.
#[test]
fn solutions_only_use_available_conditions() {
    fn conds(list: &[crate::program::Node], out: &mut Vec<crate::program::Cond>) {
        for n in list {
            if matches!(n.kind, crate::program::Kind::If | crate::program::Kind::IfElse | crate::program::Kind::While) {
                out.push(n.cond);
            }
            conds(&n.body, out);
            conds(&n.els, out);
        }
    }
    for l in LEVELS {
        let p = Program::deserialize(l.solution).unwrap();
        let mut used = Vec::new();
        conds(&p.main, &mut used);
        conds(&p.func, &mut used);
        for c in used {
            assert!(l.conds.contains(&c), "{}: solution checks {c:?}, which isn't selectable", l.name);
        }
    }
}

/// In a dynamic level, copying the exact moves that won one world must fail
/// in some other world. Otherwise kids could win without IF/WHILE.
#[test]
fn dynamic_levels_cannot_be_hardcoded() {
    for l in LEVELS.iter().filter(|l| l.is_random()) {
        let sol = Program::deserialize(l.solution).unwrap();
        let mut w = World::from_level(l, 7);
        let mut vm = Vm::new(&sol);
        let mut moves = String::new();
        loop {
            match vm.next(&w) {
                Step::Act(a, _) => {
                    moves.push_str(match a {
                        crate::program::Action::Forward => "F ",
                        crate::program::Action::TurnLeft => "L ",
                        crate::program::Action::TurnRight => "R ",
                        crate::program::Action::Mine => "M ",
                        crate::program::Action::Place => "P ",
                    });
                    if let Outcome::Won = w.apply(a) {
                        break;
                    }
                }
                _ => panic!("{}: solution didn't win seed 7", l.name),
            }
        }
        let fixed = Program::deserialize(&moves).unwrap();
        let fails = (0..300).any(|s| simulate(&fixed, World::from_level(l, s)).is_err());
        assert!(fails, "{}: the same moves win every world - it isn't really dynamic", l.name);
    }
}

#[test]
fn maps_are_well_formed() {
    for l in LEVELS {
        for map in l.maps {
            let w = map[0].len();
            assert!(map.iter().all(|r| r.len() == w), "{}: ragged rows", l.name);
            let starts: usize = map.iter().map(|r| r.chars().filter(|c| "^>v<".contains(*c)).count()).sum();
            assert_eq!(starts, 1, "{}: needs exactly one start", l.name);
            assert!(w <= 14 && map.len() <= 11, "{}: too big", l.name);
        }
    }
}

#[test]
fn every_solution_wins_in_every_world() {
    let mut broken = Vec::new();
    for l in LEVELS {
        let p = Program::deserialize(l.solution).unwrap_or_else(|| panic!("{}: bad solution", l.name));
        if p.block_count() != l.par {
            broken.push(format!("{}: par is {} but the solution uses {} blocks", l.name, l.par, p.block_count()));
        }
        if let Some(max) = l.max_blocks {
            assert!(p.block_count() <= max, "{}", l.name);
        }
        let mut used = Vec::new();
        kinds_used(&p.main, &mut used);
        kinds_used(&p.func, &mut used);
        for k in used {
            assert!(l.palette.contains(&k), "{}: solution uses {k:?} not in palette", l.name);
        }
        if let Some((seed, e)) =
            (0..300).find_map(|seed| simulate(&p, World::from_level(l, seed)).err().map(|e| (seed, e)))
        {
            broken.push(format!("{} (seed {seed}): {e}", l.name));
        }
    }
    assert!(broken.is_empty(), "solutions that fail:\n{}", broken.join("\n"));
}

#[test]
fn serialize_round_trips() {
    for l in LEVELS {
        let p = Program::deserialize(l.solution).unwrap();
        let again = Program::deserialize(&p.serialize()).unwrap();
        assert_eq!(p.serialize(), again.serialize());
    }
}

#[test]
fn empty_while_is_caught() {
    let p = Program::deserialize("[W7 ]").unwrap();
    let w = World::from_level(level("How Far?"), 1);
    assert!(simulate(&p, w).is_err());
}

/// Naive programs that ignore the lesson must fail in at least one world,
/// so kids can't skip the concept a level is teaching.
#[test]
fn shortcuts_fail_somewhere() {
    let cases = [
        ("Surprise Stones", "[R10 F ]"),                // ignores the random stones
        ("Surprise Stones", "[R10 M F ]"),              // mines air when there's no stone
        ("Stone or Lava?", "[R5 M F F ]"),              // ignores lava
        ("Diamond Detector", "[R7 F L M R ] F"),        // mines when there's no diamond
        ("Winding Road", "[W7 [E0 F | R ] ]"),          // only ever turns right
        ("The Maze", "[W7 [E0 F | [E1 L | R ] ] ]"),    // road-following can't solve the maze
        ("Bigger Maze", "[W7 [E0 F | [E1 L | R ] ] ]"),
        ("Lava Bridges", "C C R C / [W5 P F ] [R3 F ]"), // fixed walking distance
    ];
    for (name, src) in cases {
        let l = level(name);
        let p = Program::deserialize(src).unwrap();
        let fails = (0..300).any(|s| simulate(&p, World::from_level(l, s)).is_err());
        assert!(fails, "{}: shortcut {src} should not always win", l.name);
    }
}

/// Helper used to design the maze level: `cargo test find_mazes -- --ignored --nocapture`
#[test]
#[ignore]
fn find_mazes() {
    use crate::world::Rng;
    let maze = LEVELS.iter().find(|l| l.name == "The Maze").unwrap();
    let good = Program::deserialize(maze.solution).unwrap();
    let env = |k: &str, d: usize| std::env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d);
    let naive = [
        Program::deserialize("[W7 [E0 F | [E1 L | R ] ] ]").unwrap(),
        Program::deserialize("[W7 [E0 F | [E2 R | L ] ] ]").unwrap(),
    ];
    let mut found = 0;
    for seed in 1..200000u64 {
        let mut rng = Rng::new(seed);
        let (cw, ch) = (env("CW", 5), env("CH", 4));
        let (w, h) = (cw * 2 + 1, ch * 2 + 1);
        let mut g = vec![vec![b'B'; w]; h];
        let mut seen = vec![false; cw * ch];
        let mut stack = vec![(0usize, 0usize)];
        seen[0] = true;
        g[1][1] = b'.';
        while let Some(&(x, y)) = stack.last() {
            let mut nb = Vec::new();
            if x > 0 && !seen[y * cw + x - 1] { nb.push((x - 1, y)); }
            if x + 1 < cw && !seen[y * cw + x + 1] { nb.push((x + 1, y)); }
            if y > 0 && !seen[(y - 1) * cw + x] { nb.push((x, y - 1)); }
            if y + 1 < ch && !seen[(y + 1) * cw + x] { nb.push((x, y + 1)); }
            if nb.is_empty() { stack.pop(); continue; }
            let (nx, ny) = nb[rng.below(nb.len())];
            seen[ny * cw + nx] = true;
            g[ny * 2 + 1][nx * 2 + 1] = b'.';
            g[y + ny + 1][x + nx + 1] = b'.';
            stack.push((nx, ny));
        }
        // A couple of extra openings make loops that trap naive strategies.
        for _ in 0..env("HOLES", 2) {
            let x = 1 + rng.below(w - 2);
            let y = 1 + rng.below(h - 2);
            if (x + y) % 2 == 1 { g[y][x] = b'.'; }
        }
        g[1][1] = b'>';
        g[h - 2][w - 2] = b'P';
        let rows: Vec<String> = g.iter().map(|r| String::from_utf8(r.clone()).unwrap()).collect();
        let leaked: Vec<&'static str> = rows.iter().map(|r| &*Box::leak(r.clone().into_boxed_str())).collect();
        let maps: &'static [&'static [&'static str]] = Box::leak(vec![&*Box::leak(leaked.into_boxed_slice())].into_boxed_slice());
        let lvl = crate::levels::Level { maps, ..*maze };
        let w0 = World::from_level(&lvl, 0);
        if simulate(&good, w0.clone()).is_ok() && naive.iter().all(|p| simulate(p, w0.clone()).is_err()) {
            println!("seed {seed}:\n{}\n", rows.iter().map(|r| format!("\"{r}\",")).collect::<Vec<_>>().join(" "));
            found += 1;
            if found == 6 { break; }
        }
    }
}

/// Tests that point XDG_DATA_HOME at a temp dir must not run at the same time.
static SAVE_DIR_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

#[test]
fn players_can_be_created_reset_and_deleted() {
    use crate::save::{Progress, Users};
    let _guard = SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("block-coders-test-{}", std::process::id()));
    let data = dir.join("block-coders");
    std::fs::create_dir_all(&data).unwrap();
    // An old single-player save should become "Player 1".
    std::fs::write(data.join("progress.txt"), "0;3;F F F F /\n1;2;/\n").unwrap();
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };

    let mut users = Users::load();
    assert_eq!(users.list.len(), 1);
    assert_eq!(users.list[0].name, "Player 1");
    let p1 = Progress::load(users.list[0].id);
    assert_eq!(p1.total_stars(), 5);
    assert!(p1.programs[0].is_some());
    // Old saves numbered levels 0-19; they must land on the same level by name.
    std::fs::write(data.join("user-9.txt"), "6;3;[R10 F ] /\n19;2;/\n").unwrap();
    let old = Progress::load(9);
    let idx = |n: &str| LEVELS.iter().position(|l| l.name == n).unwrap();
    assert_eq!(old.stars[idx("The Long Hallway")], 3);
    assert_eq!(old.stars[idx("Deep Dark Gauntlet")], 2);
    assert!(old.programs[idx("The Long Hallway")].is_some());
    // New levels inserted before an old player's furthest level stay open.
    assert!(old.unlocked(idx("Zig Zag")) && old.unlocked(idx("Maybe a Stone")));
    assert!(old.unlocked(idx("Deep Dark Gauntlet") + 1));
    assert!(!old.unlocked(idx("Deep Dark Gauntlet") + 2));
    // ...and saving writes names, which load back the same.
    old.save();
    assert!(std::fs::read_to_string(data.join("user-9.txt")).unwrap().contains("The Long Hallway;3;"));
    assert_eq!(Progress::load(9).total_stars(), 5);

    let sam = users.add("Sam", 2);
    let mut sp = Progress::load(sam);
    assert_eq!(sp.total_stars(), 0);
    sp.stars[0] = 3;
    sp.save();
    assert!(Progress::load(sam).unlocked(1));
    assert_eq!(Users::load().list.len(), 2);

    // Starting over wipes only that player.
    Progress::load(sam).reset();
    let sp = Progress::load(sam);
    assert_eq!(sp.total_stars(), 0);
    assert!(!sp.unlocked(1));
    assert_eq!(Progress::load(users.list[0].id).total_stars(), 5);

    users.remove(sam);
    assert_eq!(Users::load().list.len(), 1);
    // A new player never inherits a deleted player's file.
    let next = users.add("Alex", 0);
    assert_eq!(Progress::load(next).total_stars(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Saves are untrusted (on the web, anyone can edit localStorage), so loading
/// must never panic and must clamp or drop anything malformed.
#[test]
fn tampered_saves_are_rejected_safely() {
    // Program parser: byte-slicing a multi-byte character used to panic.
    for bad in ["[é5 F ]", "[R", "[", "[R99999999999999999999 F ]", "[X1 F ]", "[I99 F ]", "F ] ]", "[E0 F ]"] {
        assert!(Program::deserialize(bad).is_none(), "accepted {bad:?}");
    }
    // Deep nesting must be refused, not overflow the stack.
    let deep = format!("{}F {}", "[R2 ".repeat(5000), "] ".repeat(5000));
    assert!(Program::deserialize(&deep).is_none());
    // ...but ordinary nesting still loads.
    let ok = format!("{}F {}", "[R2 ".repeat(8), "] ".repeat(8));
    assert!(Program::deserialize(&ok).is_some());
    // Huge programs are refused.
    assert!(Program::deserialize(&"F ".repeat(10_000)).is_none());
    // Repeat counts are clamped to what the editor allows.
    let p = Program::deserialize("[R0 F ] [R500 F ] /").unwrap();
    assert_eq!((p.main[0].count, p.main[1].count), (1, 99));

    // Names get the same cleanup as typing one in.
    use crate::save::clean_name;
    assert_eq!(clean_name("Sam"), "Sam");
    assert_eq!(clean_name("<script>alert(1)</script>"), "scriptalert1");
    assert_eq!(clean_name("ABCDEFGHIJKLMNOPQRSTUVWXYZ").chars().count(), crate::save::MAX_NAME);
    assert_eq!(clean_name("\u{0}\u{7}\t;;"), "Player");
    assert_eq!(clean_name("   "), "Player");
}

#[test]
fn tampered_player_list_is_cleaned_on_load() {
    use crate::save::{Progress, Users, MAX_USERS};
    let _guard = SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("block-coders-tamper-{}", std::process::id()));
    let data = dir.join("block-coders");
    std::fs::create_dir_all(&data).unwrap();
    let mut list = String::from("0;1;Guest\n1;999;A very long name indeed\n1;2;Duplicate\nx;1;Bad id\n2;-3;Bad shirt\n");
    for id in 3..20 {
        list.push_str(&format!("{id};1;P{id}\n"));
    }
    std::fs::write(data.join("users.txt"), list).unwrap();
    // Out-of-range stars, unknown levels and garbage programs.
    std::fs::write(data.join("user-1.txt"), "First Steps;250;F F\nNo Such Level;3;F\nFirst Steps;x;[é\n").unwrap();
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };

    let users = Users::load();
    assert!(users.list.len() <= MAX_USERS);
    assert!(users.list.iter().all(|u| u.id != 0), "id 0 is reserved for the guest");
    let mut ids: Vec<u32> = users.list.iter().map(|u| u.id).collect();
    ids.dedup();
    assert_eq!(ids.len(), users.list.len(), "ids must be unique");
    let first = &users.list[0];
    assert_eq!((first.id, first.name.as_str()), (1, "A very long"));
    assert!(first.shirt < crate::art::SHIRTS.len());

    let p = Progress::load(1);
    assert!(p.stars.iter().all(|&s| s <= 3));
    // An oversized save file is ignored rather than read into memory.
    std::fs::write(data.join("user-2.txt"), "x".repeat(crate::save::MAX_SAVE_BYTES + 1)).unwrap();
    assert_eq!(Progress::load(2).total_stars(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}
