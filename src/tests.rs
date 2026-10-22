use crate::levels::LEVELS;
use crate::program::{kinds_used, simulate, Program};
use crate::program::{Step, Vm};
use crate::world::{Outcome, World};

fn level(name: &str) -> &'static crate::levels::Level {
    LEVELS.iter().find(|l| l.name == name).unwrap_or_else(|| panic!("no level {name}"))
}

#[test]
fn level_list_is_sane() {
    assert_eq!(LEVELS.len(), 66);
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
        // Kids only ever see blocks, so Bit talks in block words, not typed code.
        for text in [l.hint, l.lesson, l.intro.map_or("", |i| i.1)] {
            assert!(!text.contains(['{', '}', ';']), "{}: uses typed-code symbols", l.name);
        }
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
                Step::Var(_) => {}
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
        ("Echo Hall", "[W0 F ] R [W0 F ] L [W0 F ]"),       // walks to the walls instead of counting
        ("Echo Hall", "[W0 F ] R [R3 F ] L [R6 F ] F"),     // a fixed distance
        ("Stone Counter", "[W3 M F ] R [W0 F ] L [W0 F ]"),
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
    let listed = std::fs::read_to_string(data.join("users.txt")).unwrap();
    std::fs::write(data.join("users.txt"), format!("{listed}9;1;Old Timer\n")).unwrap();
    let mut old = Progress::load(9);
    let idx = |n: &str| LEVELS.iter().position(|l| l.name == n).unwrap();
    assert_eq!(old.stars[idx("The Long Hallway")], 3);
    assert_eq!(old.stars[idx("Deep Dark Gauntlet")], 2);
    assert!(old.programs[idx("The Long Hallway")].is_some());
    // New levels inserted before an old player's furthest level stay open.
    assert!(old.unlocked(idx("Zig Zag")) && old.unlocked(idx("Maybe a Stone")));
    // The next level is the first of the last world, which needs every World Check.
    assert!(!old.unlocked(idx("Deep Dark Gauntlet") + 1));
    let mut checked = Progress::load(9);
    checked.checks.iter_mut().for_each(|c| *c = true);
    assert!(checked.unlocked(idx("Deep Dark Gauntlet") + 1));
    assert!(!checked.unlocked(idx("Deep Dark Gauntlet") + 2));
    // ...and saving writes names, which load back the same.
    old.save();
    assert!(std::fs::read_to_string(data.join("user-9.txt")).unwrap().contains("The Long Hallway;3;"));
    assert_eq!(Progress::load(9).total_stars(), 5);

    let sam = users.add("Sam", 2).unwrap();
    let mut sp = Progress::load(sam);
    assert_eq!(sp.total_stars(), 0);
    sp.stars[0] = 3;
    sp.save();
    assert!(Progress::load(sam).unlocked(1));
    assert_eq!(Users::load().list.len(), 3);
    // Saves are swapped in whole (no temp files left over) and kept private.
    let leftovers: Vec<_> = std::fs::read_dir(&data).unwrap().flatten().filter(|e| e.path().extension().is_some_and(|x| x == "tmp")).collect();
    assert!(leftovers.is_empty(), "temp files left behind: {leftovers:?}");
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(std::fs::metadata(&data).unwrap().permissions().mode() & 0o777, 0o700);
    }

    // Starting over wipes only that player.
    Progress::load(sam).reset();
    let sp = Progress::load(sam);
    assert_eq!(sp.total_stars(), 0);
    assert!(!sp.unlocked(1));
    assert_eq!(Progress::load(users.list[0].id).total_stars(), 5);

    users.remove(sam);
    assert_eq!(Users::load().list.len(), 2);
    // A new player never inherits a deleted player's file.
    let next = users.add("Alex", 0).unwrap();
    assert_eq!(Progress::load(next).total_stars(), 0);
    let _ = std::fs::remove_dir_all(&dir);
}

/// The screenshot autopilot shows real players; nothing it does may change a save.
#[test]
fn read_only_saves_never_change() {
    use crate::save::{set_read_only, Progress, Users};
    let _guard = SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("block-coders-readonly-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
    let mut users = Users::load();
    let ana = users.add("Ana", 0).unwrap();
    let mut p = Progress::load(ana);
    p.stars[0] = 3;
    p.save();

    set_read_only(true);
    Progress::load(ana).reset();
    let _ = users.add("Sam", 2);
    users.remove(ana);
    set_read_only(false);

    let names: Vec<String> = Users::load().list.iter().map(|u| u.name.clone()).collect();
    assert_eq!(names, ["Ana"]);
    assert_eq!(Progress::load(ana).total_stars(), 3);
    let _ = std::fs::remove_dir_all(&dir);

    // Loading can write too: an old single-player save becomes "Player 1".
    // The autopilot turns read-only on first, so even that leaves no trace.
    let data = dir.join("block-coders");
    std::fs::create_dir_all(&data).unwrap();
    std::fs::write(data.join("progress.txt"), "0;3;F F F F /\n").unwrap();
    set_read_only(true);
    assert_eq!(Users::load().list.len(), 1);
    set_read_only(false);
    let files: Vec<_> = std::fs::read_dir(&data).unwrap().flatten().map(|e| e.file_name()).collect();
    assert_eq!(files, ["progress.txt"]);
    let _ = std::fs::remove_dir_all(&dir);
}

/// Two browser tabs share one save store. A tab that loaded the player list
/// earlier must not drop players the other tab added, or reuse their ids and
/// wipe their stars.
#[test]
fn a_stale_tab_keeps_other_tabs_players() {
    use crate::save::{Progress, Users, MAX_USERS};
    let _guard = SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("block-coders-tabs-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };

    let mut tab_a = Users::load();
    tab_a.add("Ana", 0).unwrap();
    let mut tab_b = Users::load();
    let ben = tab_a.add("Ben", 1).unwrap();
    let mut p = Progress::load(ben);
    p.stars[0] = 3;
    p.save();

    let cat = tab_b.add("Cat", 2).unwrap();
    assert_ne!(cat, ben);
    let names: Vec<String> = Users::load().list.iter().map(|u| u.name.clone()).collect();
    assert_eq!(names, ["Ana", "Ben", "Cat"]);
    assert_eq!(Progress::load(ben).total_stars(), 3);
    // A name another tab already took is refused, whatever case it's typed in.
    assert!(tab_a.add("cat", 3).is_none());
    // Deleting from a stale tab keeps everyone else.
    let mut tab_c = Users::load();
    tab_a.add("Dee", 3).unwrap();
    tab_c.remove(cat);
    let names: Vec<String> = Users::load().list.iter().map(|u| u.name.clone()).collect();
    assert_eq!(names, ["Ana", "Ben", "Dee"]);
    // A full list (filled by another tab) refuses new players.
    for n in ["Eve", "Fay", "Gus"] {
        tab_a.add(n, 0).unwrap();
    }
    assert_eq!(Users::load().list.len(), MAX_USERS);
    assert!(tab_c.add("Hal", 0).is_none());
    let _ = std::fs::remove_dir_all(&dir);
}

/// A stale tab playing the same player as another tab must not erase the
/// other tab's stars, undo a "Start over", or write a deleted player's
/// progress into a new player.
#[test]
fn a_stale_tab_never_overwrites_newer_progress() {
    use crate::save::{Progress, Saved, Users};
    let _guard = SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    let dir = std::env::temp_dir().join(format!("block-coders-stale-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };
    let mut users = Users::load();
    let ana = users.add("Ana", 0).unwrap();
    let prog = |s: &str| crate::program::Program::deserialize(s);

    // Both tabs open Ana. A earns stars; B then edits a different level.
    let mut a = Progress::load(ana);
    let mut b = Progress::load(ana);
    a.stars[0] = 3;
    a.programs[0] = prog("F F F F /");
    a.checks[0] = true;
    a.save();
    b.programs[1] = prog("F /");
    b.save();
    let now = Progress::load(ana);
    assert_eq!(now.stars[0], 3, "B erased A's stars");
    assert!(now.checks[0], "B erased A's World Check");
    assert!(now.programs[0].is_some() && now.programs[1].is_some());
    assert_eq!(b.stars[0], 3, "B should catch up on A's stars when it saves");
    // B's own edit to a level wins over the stored one.
    b.programs[0] = prog("R F /");
    b.save();
    assert_eq!(Progress::load(ana).programs[0].as_ref().map(|p| p.serialize()), prog("R F /").map(|p| p.serialize()));

    // A starts over; B, still showing the old stars, must not bring them back.
    let mut b = Progress::load(ana);
    Progress::load(ana).reset();
    b.stars[2] = 2;
    b.programs[2] = prog("F F /");
    assert_eq!(b.save(), Saved::StartedOverElsewhere);
    // The play screen then shows the stored program, so its next edit can't
    // bring the old one back.
    assert!(b.programs[2].is_none());
    b.programs[2] = prog("L /");
    assert_eq!(b.save(), Saved::Done);
    assert_eq!(Progress::load(ana).programs[2].as_ref().map(|p| p.serialize()), prog("L /").map(|p| p.serialize()));
    b.programs[2] = None;
    b.save();
    let now = Progress::load(ana);
    assert_eq!(now.total_stars(), 0, "a stale tab undid Start over");
    assert!(now.programs.iter().all(|p| p.is_none()));
    assert_eq!(b.total_stars(), 0, "B should pick up the reset");
    // ...but once caught up, B saves normally again.
    b.stars[0] = 1;
    b.save();
    assert_eq!(Progress::load(ana).stars[0], 1);

    // A deletes Ana and adds Ben; B, still playing Ana, saves.
    let mut b = Progress::load(ana);
    users.remove(ana);
    let ben = users.add("Ben", 1).unwrap();
    assert_ne!(ben, ana, "ids must never be reused");
    b.stars[0] = 3;
    b.save();
    b.reset();
    assert_eq!(Progress::load(ben).total_stars(), 0);
    assert!(!dir.join("block-coders").join(format!("user-{ana}.txt")).exists(), "deleted player came back");
    // Deleting the newest player still doesn't free their id.
    users.remove(ben);
    let cat = users.add("Cat", 2).unwrap();
    assert!(cat > ben);

    // A save that fails (storage full, disk error) mustn't make the next save
    // think the edit was stored: the win after it keeps the solution.
    let mut p = Progress::load(cat);
    p.programs[0] = prog("F F F F /");
    let blocker = dir.join("block-coders").join(format!("user-{cat}.txt.tmp"));
    std::fs::create_dir_all(&blocker).unwrap(); // the temp file can't be written
    p.save();
    std::fs::remove_dir(&blocker).unwrap();
    assert!(Progress::load(cat).programs[0].is_none(), "the failed save shouldn't have landed");
    p.stars[0] = 3;
    p.save();
    let now = Progress::load(cat);
    assert_eq!(now.stars[0], 3);
    assert_eq!(now.programs[0].as_ref().map(|p| p.serialize()), prog("F F F F /").map(|p| p.serialize()), "the solution was lost");
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
    // The depth limit on its own, well under the token limit: 32 deep loads, 33 doesn't.
    let nest = |n: usize| format!("{}F {}", "[R2 ".repeat(n), "] ".repeat(n));
    assert!(Program::deserialize(&nest(32)).is_some());
    assert!(Program::deserialize(&nest(33)).is_none());
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

// ------------------------------ World Checks ------------------------------

mod world_checks {
    use crate::check::{changes, pick, run, world_for, Run, ASKED, CHECKS, MAX_CHANGING, PASS};
    use crate::levels::{CHAPTERS, LEVELS};
    use crate::program::Program;
    use crate::questions::{Choices, End, Proof, Question, BANKS};
    use crate::save::Progress;
    use crate::world::Rng;

    fn parse(q: &Question, src: &str) -> Program {
        Program::deserialize(src).unwrap_or_else(|| panic!("{}: can't parse program {src:?}", q.id))
    }

    /// Everything about how a run ended, for comparing two programs.
    fn result(r: &Run) -> (End, (i32, i32), crate::world::Dir, u32, Vec<crate::world::Tile>) {
        (r.end, r.world.pos, r.world.dir, r.world.blocks, r.world.tiles.clone())
    }

    fn the_program(q: &'static Question) -> Run {
        assert!(!q.program.is_empty(), "{}: needs a program", q.id);
        assert!(!changes(q), "{}: this kind of question needs a map that never changes", q.id);
        run(&parse(q, q.program), world_for(q, 0))
    }

    fn texts(q: &Question) -> &'static [&'static str] {
        match q.choices {
            Choices::Text(t) => t,
            _ => panic!("{}: needs text choices", q.id),
        }
    }

    /// Choices are shuffled before they're shown, so an explanation that points
    /// at "the middle program" can end up criticising the right answer.
    #[test]
    fn explanations_never_point_at_a_position() {
        let words = ["first", "second", "third", "middle", "last", "left", "right", "top", "bottom", "above", "below", "other"];
        for bank in BANKS.iter() {
            for q in bank.iter() {
                let lower = q.explain.to_lowercase();
                for w in words {
                    for noun in ["program", "one", "answer", "choice", "option", "square"] {
                        let phrase = format!("{w} {noun}");
                        assert!(!lower.contains(&phrase), "{}: explanation says \"{phrase}\", but choices are shuffled", q.id);
                    }
                }
                for letter in ["a", "b", "c", "d"] {
                    for noun in ["program", "square", "choice", "answer"] {
                        let phrase = format!("{noun} {letter} ");
                        assert!(!format!("{lower} ").contains(&phrase), "{}: explanation names {noun} {letter}, but choices are shuffled", q.id);
                    }
                }
            }
        }
    }

    #[test]
    fn banks_are_big_enough_and_well_formed() {
        assert_eq!(CHECKS, CHAPTERS.len() - 1, "one World Check per world except the last");
        assert!(PASS as f32 >= ASKED as f32 * 0.75, "passing needs at least 75%");
        let mut ids = std::collections::HashSet::new();
        for (c, bank) in BANKS.iter().enumerate() {
            // Each retry has to include at least 3 questions from outside the last attempt,
            // and at most MAX_CHANGING of an attempt can be on a changing map.
            let changing = bank.iter().filter(|q| changes(q)).count();
            let usable = bank.len() - changing + changing.min(MAX_CHANGING);
            assert!(usable >= ASKED + 3, "bank {c} has only {usable} usable questions");
            for q in *bank {
                assert!(ids.insert(q.id), "duplicate question id {}", q.id);
                assert!(!q.id.contains([',', ';']), "{}: ids are saved, so no , or ;", q.id);
                let n = match q.choices {
                    Choices::Text(t) => t.len(),
                    Choices::Programs(p) => p.len(),
                    Choices::Spots(s) => s.len(),
                };
                assert!((2..=4).contains(&n), "{}: needs 2 to 4 choices", q.id);
                if let Choices::Text(t) = q.choices {
                    let mut seen = std::collections::HashSet::new();
                    assert!(t.iter().all(|x| seen.insert(*x)), "{}: two choices are the same", q.id);
                }
                if let Choices::Spots(s) = q.choices {
                    let w = world_for(q, 0);
                    for &(x, y) in s {
                        assert!(x >= 0 && y >= 0 && x < w.w && y < w.h, "{}: spot ({x},{y}) is off the map", q.id);
                    }
                }
                assert!(!q.explain.is_empty(), "{}: needs an explanation", q.id);
                if !q.program.is_empty() {
                    parse(q, q.program);
                }
                if let Choices::Programs(ps) = q.choices {
                    for p in ps {
                        parse(q, p);
                    }
                }
                // The map must build (this panics on unknown symbols).
                world_for(q, 0);
            }
        }
    }

    /// Runs every question's programs and checks the marked answer (always
    /// the first choice) really is right, and the others really are wrong.
    #[test]
    fn every_answer_is_right() {
        for q in BANKS.iter().flat_map(|b| b.iter()) {
            match q.proof {
                Proof::Count(what) => {
                    let r = the_program(q);
                    assert!(matches!(r.end, End::Wins | End::Unfinished), "{}: program fails with {:?}", q.id, r.end);
                    let n = r.actions.iter().filter(|a| what.contains(a)).count();
                    let t = texts(q);
                    assert_eq!(t[0], n.to_string(), "{}: the count is {n}", q.id);
                    assert!(t[1..].iter().all(|x| *x != n.to_string()), "{}: a wrong choice is also {n}", q.id);
                }
                Proof::Variable => {
                    let r = the_program(q);
                    assert!(matches!(r.end, End::Wins | End::Unfinished), "{}: program fails with {:?}", q.id, r.end);
                    let t = texts(q);
                    assert_eq!(t[0], r.count.to_string(), "{}: count ends at {}", q.id, r.count);
                    assert!(t[1..].iter().all(|x| *x != r.count.to_string()), "{}: a wrong choice is also {}", q.id, r.count);
                }
                Proof::Backpack => {
                    let r = the_program(q);
                    assert_eq!(texts(q)[0], r.world.blocks.to_string(), "{}: backpack ends with {}", q.id, r.world.blocks);
                }
                Proof::Ends(end) => {
                    let r = the_program(q);
                    assert_eq!(r.end, end, "{}: program ends with {:?}", q.id, r.end);
                }
                Proof::EndsAt => {
                    let r = the_program(q);
                    assert!(matches!(r.end, End::Wins | End::Unfinished), "{}: program fails with {:?}", q.id, r.end);
                    let Choices::Spots(s) = q.choices else { panic!("{}: needs spot choices", q.id) };
                    assert_eq!(s[0], r.world.pos, "{}: miner ends at {:?}", q.id, r.world.pos);
                    assert!(!s[1..].contains(&r.world.pos), "{}: a wrong spot is the real end", q.id);
                }
                Proof::SameResult => {
                    let target = result(&the_program(q));
                    let Choices::Programs(ps) = q.choices else { panic!("{}: needs program choices", q.id) };
                    for (i, p) in ps.iter().enumerate() {
                        let same = result(&run(&parse(q, p), world_for(q, 0))) == target;
                        assert_eq!(same, i == 0, "{}: choice {i} ({p}) {} the same", q.id, if same { "ends" } else { "doesn't end" });
                    }
                }
                Proof::OnlyWinner => {
                    let Choices::Programs(ps) = q.choices else { panic!("{}: needs program choices", q.id) };
                    let seeds = if changes(q) { 300 } else { 1 };
                    for (i, p) in ps.iter().enumerate() {
                        let prog = parse(q, p);
                        let wins = (0..seeds).filter(|&s| run(&prog, world_for(q, s)).end == End::Wins).count();
                        if i == 0 {
                            assert_eq!(wins, seeds as usize, "{}: the answer ({p}) loses in some versions", q.id);
                        } else {
                            assert!(wins < seeds as usize, "{}: wrong choice {i} ({p}) wins every time", q.id);
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn retries_get_new_questions() {
        let mut rng = Rng::new(7);
        for bank in BANKS {
            let mut last: Vec<String> = Vec::new();
            for _ in 0..50 {
                let qs = pick(bank, &last, &mut rng);
                assert_eq!(qs.len(), ASKED);
                let mut ids: Vec<&str> = qs.iter().map(|q| q.id).collect();
                ids.sort();
                ids.dedup();
                assert_eq!(ids.len(), ASKED, "a question came up twice");
                assert!(qs.iter().filter(|q| changes(q)).count() <= MAX_CHANGING, "too many changing-map questions");
                if !last.is_empty() {
                    let new = qs.iter().filter(|q| !last.iter().any(|id| id == q.id)).count();
                    assert!(new >= 3, "only {new} new questions on a retry");
                }
                last = qs.iter().map(|q| q.id.to_string()).collect();
            }
        }
    }

    fn first_of(c: usize) -> usize {
        LEVELS.iter().position(|l| l.chapter == c).unwrap()
    }
    fn last_of(c: usize) -> usize {
        LEVELS.iter().rposition(|l| l.chapter == c).unwrap()
    }

    #[test]
    fn checks_gate_the_next_world() {
        let master = CHAPTERS.len() - 1;
        // A new player who has beaten all of world 1 must pass its check first.
        let mut p = Progress::new(None);
        for i in 0..=last_of(0) {
            p.stars[i] = 3;
        }
        assert!(p.check_open(0));
        assert!(!p.check_open(1));
        assert!(!p.unlocked(first_of(1)));
        p.checks[0] = true;
        assert!(p.unlocked(first_of(1)));

        // An old save already in world 3 keeps worlds 2 and 3 without the checks...
        let mut old = Progress::new(None);
        for i in 0..=first_of(2) {
            old.stars[i] = 2;
        }
        assert!(old.unlocked(first_of(1)) && old.unlocked(first_of(2)));
        assert!(old.check_open(0) && old.check_open(1), "they can still take the checks they skipped");
        // ...but not a world it hasn't reached yet.
        for i in 0..=last_of(2) {
            old.stars[i] = 2;
        }
        assert!(!old.unlocked(first_of(3)), "world 4 still needs the world 3 check");
        old.checks[2] = true;
        assert!(old.unlocked(first_of(3)));

        // The last world needs every check, even for a save that already beat it.
        let mut done = Progress::new(None);
        done.stars.iter_mut().for_each(|s| *s = 3);
        assert!(done.unlocked(last_of(master - 1)), "earlier worlds stay open");
        assert!(!done.unlocked(first_of(master)));
        for c in 1..CHECKS {
            done.checks[c] = true;
        }
        assert!(!done.unlocked(first_of(master)), "one check still missing");
        done.checks[0] = true;
        assert!(done.unlocked(first_of(master)) && done.unlocked(last_of(master)));

        // Players already in the Nether Fortress before the Counting Caverns
        // world (and its check) were added keep it...
        done.checks[CHECKS - 1] = false;
        assert!(done.unlocked(first_of(master)), "an old save that had reached the last world keeps it");
        // ...but a player who hasn't got there yet needs the new check too.
        let mut new = Progress::new(None);
        for i in 0..first_of(master) {
            new.stars[i] = 3;
        }
        new.checks.iter_mut().for_each(|c| *c = true);
        new.checks[CHECKS - 1] = false;
        assert!(!new.unlocked(first_of(master)));
        new.checks[CHECKS - 1] = true;
        assert!(new.unlocked(first_of(master)));

        // Parent mode (Ctrl+U) opens everything.
        let mut parent = Progress::new(None);
        parent.unlock_all = true;
        assert!(parent.unlocked(first_of(master)) && parent.check_open(CHECKS - 1));
    }

    #[test]
    fn check_results_are_saved() {
        let _guard = super::SAVE_DIR_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("block-coders-checks-{}", std::process::id()));
        let data = dir.join("block-coders");
        std::fs::create_dir_all(&data).unwrap();
        // Only listed players are saved (a deleted one never comes back).
        std::fs::write(data.join("users.txt"), "5;0;Kim\n").unwrap();
        unsafe { std::env::set_var("XDG_DATA_HOME", &dir) };

        let mut p = Progress::new(Some(5));
        p.stars[0] = 3;
        p.checks[1] = true;
        p.check_last[1] = vec!["w2-01".into(), "w2-07".into()];
        p.save();
        let back = Progress::load(5);
        assert_eq!(back.checks, p.checks);
        assert_eq!(back.check_last[1], p.check_last[1]);
        assert_eq!(back.stars[0], 3);

        // Hand-edited or unknown entries are ignored, not trusted.
        let text = std::fs::read_to_string(data.join("user-5.txt")).unwrap();
        let text = text + "check:99;1;w1-01\ncheck:x;1;\ncheck:2;1;nope,w3-02,w1-01\n";
        std::fs::write(data.join("user-5.txt"), text).unwrap();
        let back = Progress::load(5);
        assert!(back.checks[2]);
        assert_eq!(back.check_last[2], vec!["w3-02".to_string()], "only real questions from that world");
        let _ = std::fs::remove_dir_all(&dir);
    }

    fn json(s: &str) -> String {
        let mut out = String::from("\"");
        for c in s.chars() {
            match c {
                '"' => out.push_str("\\\""),
                '\\' => out.push_str("\\\\"),
                '\n' => out.push_str("\\n"),
                c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
                c => out.push(c),
            }
        }
        out.push('"');
        out
    }

    fn json_list(items: impl IntoIterator<Item = String>) -> String {
        format!("[{}]", items.into_iter().collect::<Vec<_>>().join(","))
    }

    /// Programs as the Rust code kids see in "Show Rust code", which reads
    /// better than the save format.
    fn rust(q: &Question, src: &str) -> String {
        parse(q, src).to_rust().join("\n")
    }

    /// Writes every question to `target/question-audit/questions.json` for
    /// `tools/audit_questions.py`. Skipped by default; run it with
    /// `cargo test export_questions -- --ignored`.
    #[test]
    #[ignore]
    fn export_questions() {
        let mut out = Vec::new();
        for (c, bank) in BANKS.iter().enumerate() {
            for q in bank.iter() {
                let (kind, choices) = match q.choices {
                    Choices::Text(t) => ("text", t.iter().map(|s| s.to_string()).collect::<Vec<_>>()),
                    Choices::Programs(p) => ("programs", p.iter().map(|s| rust(q, s)).collect()),
                    Choices::Spots(s) => ("spots", s.iter().map(|(x, y)| format!("the square at column {x}, row {y}")).collect()),
                };
                let program = if q.program.is_empty() { "null".to_string() } else { json(&rust(q, q.program)) };
                out.push(format!(
                    "{{\"id\":{},\"world\":{},\"prompt\":{},\"program\":{},\"map\":{},\"blocks\":{},\"kind\":{},\"choices\":{},\"explain\":{}}}",
                    json(q.id),
                    c + 1,
                    json(q.prompt),
                    program,
                    json_list(q.map.iter().map(|r| json(r))),
                    q.blocks,
                    json(kind),
                    json_list(choices.iter().map(|s| json(s))),
                    json(q.explain),
                ));
            }
        }
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("target/question-audit");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("questions.json"), json_list(out) + "\n").unwrap();
    }
}

mod smart_hints {
    use super::level;
    use crate::hints::{failing_test_world, ladder, Hint};
    use crate::levels::LEVELS;
    use crate::program::{contains, simulate, Node, Program};
    use crate::world::World;

    fn hints(name: &str, program: &str) -> (Program, Vec<Hint>) {
        let l = level(name);
        let p = Program::deserialize(program).unwrap();
        let h = ladder(l, &p, &World::from_level(l, 1), 1);
        (p, h)
    }

    #[test]
    fn a_missing_block_is_found() {
        let (p, h) = hints("First Steps", "F F F");
        assert!(h[0].text.contains("ONE change"), "{h:?}");
        assert_eq!(h[2].text, "Add a FORWARD block right AFTER the outlined block.");
        assert_eq!(h[2].block, Some(p.main[2].id));
    }

    #[test]
    fn a_wrong_turn_is_found() {
        let (p, h) = hints("Turn the Corner", "F F L F F L F F");
        assert!(h[0].text.starts_with("The miner walks into a wall"), "{h:?}");
        assert!(h[1].text.contains("wrong way"));
        assert_eq!(h[2].text, "Change the outlined TURN LEFT into TURN RIGHT.");
        assert_eq!(h[2].block, Some(p.main[2].id));
        assert_eq!(h.len(), 3, "the exact change is the last hint");
    }

    #[test]
    fn a_wrong_repeat_count_is_found() {
        let (p, h) = hints("The Long Hallway", "[R8 F ]");
        assert!(h[1].text.contains("MORE times than 8"), "{h:?}");
        assert_eq!(h[2].text, "Change the outlined REPEAT from 8 to 10.");
        assert_eq!(h[2].block, Some(p.main[0].id));
    }

    #[test]
    fn a_wrong_check_is_found_and_works_in_every_world() {
        let (p, h) = hints("Stone or Lava?", "[R5 [E5 M | P ] F F ]");
        assert_eq!(h[2].text, "Change the outlined IF / ELSE to check \"stone ahead\" instead of \"lava ahead\".");
        assert_eq!(h[2].block, Some(p.main[0].body[0].id));
    }

    #[test]
    fn far_from_done_says_which_way_to_go() {
        // The game has just said the miner isn't in the portal, so Bit starts
        // with where to go instead of repeating it.
        let (_, h) = hints("Turn the Corner", "F");
        assert_eq!(h[0].text, "From where the miner stops, the portal is 5 squares away. The way there starts straight AHEAD.");
        assert!(h.last().unwrap().text.starts_with("Hint: "), "the level's own hint comes last");
    }

    #[test]
    fn an_extra_block_is_removed_not_swapped_for_an_empty_function() {
        // CALL FUNCTION is in this palette, but the function is empty, so
        // "change it into CALL FUNCTION" would only be a confusing "delete it".
        let (p, h) = hints("Lava Maybe", "[R7 [I5 P F ] F ]");
        assert!(h[1].text.contains("extra"), "{h:?}");
        assert_eq!(h[2].text, "Remove the outlined block. It doesn't belong there.");
        assert_eq!(h[2].block, Some(p.main[0].body[0].body[1].id));
    }

    #[test]
    fn fixes_never_win_just_because_the_game_stops_at_the_portal() {
        // Swapping PLACE BLOCK for FORWARD walks 20 squares down a 10-square
        // hallway; it "wins" only because the game stops at the portal.
        let (p, h) = hints("The Long Hallway", "[R10 F P ]");
        assert_eq!(h[2].text, "Remove the outlined block. It doesn't belong there.", "{h:?}");
        assert_eq!(h[2].block, Some(p.main[0].body[1].id));
        // Two steps are missing after the loop: a third lap would reach the
        // portal and then walk into the wall, so that's not a fix.
        let (_, h) = hints("Crossroads", "[R2 F F [E1 L | R ] ]");
        assert!(h.iter().all(|x| !x.text.contains("REPEAT")), "{h:?}");
    }

    #[test]
    fn add_hints_point_next_to_the_outlined_block() {
        let (_, h) = hints("First Steps", "F F F");
        assert!(h[1].text.starts_with("Something is missing right AFTER the outlined block"), "{h:?}");
        let (p, h) = hints("Rocky Stairs", "[R4 M F R M F L ]");
        assert_eq!(h[2].text, "Add a FORWARD block right AFTER the outlined REPEAT block - below it, not inside it.", "{h:?}");
        assert_eq!(h[2].block, Some(p.main[0].id));
    }

    #[test]
    fn winning_programs_with_wasted_steps() {
        // The fifth FORWARD comes after the win, so it never runs.
        let (p, h) = hints("First Steps", "F F F F F");
        assert!(h[0].text.contains("never runs"), "{h:?}");
        assert_eq!(h[1].block, Some(p.main[4].id));
        // REPEAT 11 wins on the 10th step; the 11th would hit the wall.
        let (_, h) = hints("The Long Hallway", "[R11 F ]");
        assert!(h[0].text.contains("only because the game stops it"), "{h:?}");
        assert!(h[0].text.contains("REPEAT"), "{h:?}");
        // Repeated steps in a function level get the FUNCTION tip.
        let (_, h) = hints("Teach a New Word", "M F M F M F M F F");
        assert!(h.iter().any(|x| x.text.contains("FUNCTION once")), "{h:?}");
    }

    #[test]
    fn checks_are_never_changed_to_repeat_an_outer_check() {
        // The empty "IF path ahead" sits in the ELSE of "IF path to right", so
        // changing it to check "path to right" would make it always false.
        // Any other check would only steer around the empty part, so Bit says
        // to fill it.
        let (p, h) = hints("Rubble Maze", "[W7 [E2 R F | [E0 | [E3 M | L ] ] ] ]");
        assert_eq!(h[2].text, "Add a FORWARD block inside the outlined IF / ELSE block.", "{h:?}");
        assert_eq!(h[2].block, Some(p.main[0].body[0].els[0].id));
        // Checking "path to right" inside "IF path to right" is never offered either.
        let (_, h) = hints("The Maze", "[W7 [E2 R F | [E0 F R | L ] ] ]");
        assert!(h.iter().all(|x| !x.text.contains("instead of \"path ahead\"")), "{h:?}");
    }

    #[test]
    fn working_programs_skip_the_levels_general_hint() {
        let (_, h) = hints("The Long Hallway", "[R11 F ]");
        assert!(h.iter().all(|x| !x.text.starts_with("Hint: ")), "{h:?}");
    }

    #[test]
    fn changing_levels_suggest_checking_before_mining() {
        // A plain MINE would swing at thin air in versions without the stone.
        let l = level("Maybe a Stone");
        let p = Program::deserialize("F F F F").unwrap();
        let seed = (0..500).find(|&s| simulate(&p, World::from_level(l, s)).is_err()).expect("a world with the stone");
        let h = ladder(l, &p, &World::from_level(l, seed), seed);
        assert!(h[0].text.contains("IF \"stone ahead\""), "{h:?}");
        // Already inside "WHILE stone ahead": a plain MINE is the right advice.
        let (_, h) = hints("Stone Tunnel", "[W3 F ]");
        assert!(h[0].text.starts_with("The outlined block walks into a block the miner could MINE"), "{h:?}");
    }

    #[test]
    fn luck_in_one_world_is_not_enough() {
        // Walks straight through: wins only in worlds without stones.
        let l = level("Surprise Stones");
        let p = Program::deserialize("[R10 F ]").unwrap();
        let seed = (0..500).find(|&s| simulate(&p, World::from_level(l, s)).is_ok()).expect("a stone-free world");
        let h = ladder(l, &p, &World::from_level(l, seed), seed);
        assert!(h[0].text.contains("not in every version"), "{h:?}");
        assert!(h[1].text.contains("\"stone ahead\""), "{h:?}");
    }

    #[test]
    fn long_winning_programs_get_a_pattern() {
        let (p, h) = hints("The Long Hallway", "F F F F F F F F F F");
        assert!(h[0].text.contains("3 stars"), "{h:?}");
        assert_eq!(h[1].text, "Starting at the outlined block, the same block happens 10 times in a row.");
        assert_eq!(h[2].text, "Put that block inside a REPEAT 10 block. That saves 8 blocks!");
        assert_eq!(h[2].block, Some(p.main[0].id));

        let (_, h) = hints("Crystal Staircase", "F R F L F R F L F R F L F R F L F");
        assert!(h[2].text.starts_with("Put those 4 blocks inside a REPEAT 4 block"), "{h:?}");
    }

    #[test]
    fn empty_and_solved_programs() {
        let (_, h) = hints("First Steps", "");
        assert!(h[0].text.starts_with("Let's start!"));
        let (_, h) = hints("Teach a New Word", "/ M F");
        assert!(h[0].text.contains("MAIN is empty"));
        let (_, h) = hints("First Steps", "F F F F");
        assert!(h[0].text.contains("already solves"));
    }

    /// Every solution with one simple block taken out is one change from
    /// working, so Bit must always find a fix for it, and every hint must
    /// point at a block that's really in the program.
    #[test]
    fn every_level_finds_a_removed_block() {
        fn simple_ids(list: &[Node], out: &mut Vec<u32>) {
            for n in list {
                if !n.kind.is_container() {
                    out.push(n.id);
                }
                simple_ids(&n.body, out);
                simple_ids(&n.els, out);
            }
        }
        fn without(list: &[Node], id: u32) -> Vec<Node> {
            list.iter()
                .filter(|n| n.id != id)
                .map(|n| Node { body: without(&n.body, id), els: without(&n.els, id), ..n.clone() })
                .collect()
        }
        for l in LEVELS {
            let sol = Program::deserialize(l.solution).unwrap();
            let mut ids = Vec::new();
            simple_ids(&sol.main, &mut ids);
            simple_ids(&sol.func, &mut ids);
            for id in ids {
                let p = Program { main: without(&sol.main, id), func: without(&sol.func, id) };
                let start = World::from_level(l, 7);
                if p.main.is_empty() || (simulate(&p, start.clone()).is_ok() && failing_test_world(l, &p, 7).is_none()) {
                    continue; // nothing to fix
                }
                let h = ladder(l, &p, &start, 7);
                assert!(h[0].text.contains("ONE change"), "{}: {} -> {h:?}", l.name, p.serialize());
                for hint in &h {
                    if let Some(b) = hint.block {
                        assert!(contains(&p.main, b) || contains(&p.func, b), "{}: hint points at a missing block", l.name);
                    }
                }
            }
        }
    }
}

mod guided_tour {
    use crate::levels::LEVELS;
    use crate::play::spot_rect;
    use crate::program::Kind;
    use crate::tour::{allows, bubble, Facts, Out, Spot, Tour, Wait, BUBBLE_MAX_H, STEPS};
    use macroquad::math::{vec2, Rect};

    /// Plays the tour the way a child would and checks it ends after RUN.
    #[test]
    fn the_tour_can_be_finished() {
        let mut t = Tour::new(false);
        let mut blocks = 0;
        for _ in 0..STEPS.len() {
            let out = match t.current().wait {
                Wait::Next => t.next(),
                Wait::Blocks(n) => {
                    assert_eq!(t.update(&Facts { main_blocks: n - 1, running: false }), Out::Stay);
                    assert_eq!(t.current().wait, Wait::Blocks(n), "moved on too early");
                    blocks = n;
                    t.update(&Facts { main_blocks: n, running: false })
                }
                Wait::Run => {
                    assert_eq!(t.update(&Facts { main_blocks: blocks, running: false }), Out::Stay);
                    t.update(&Facts { main_blocks: blocks, running: true })
                }
            };
            if out == Out::Done {
                assert_eq!(t.step, STEPS.len(), "only the last step may finish the tour");
                return;
            }
        }
        panic!("the tour never finished");
    }

    /// The tour talks about level 1: a FORWARD block first in the palette,
    /// and a portal 4 squares away.
    #[test]
    fn the_tour_matches_level_one() {
        let l = &LEVELS[0];
        assert_eq!(l.palette[0], Kind::Forward);
        assert_eq!(l.solution, "F F F F");
        assert!(STEPS.iter().any(|s| s.wait == Wait::Blocks(4)));
        assert_eq!(STEPS.last().unwrap().wait, Wait::Run);
    }

    /// On every screen shape: the bubble is on screen, beside (never on top
    /// of) what it points at, and at least as wide as the tour needs.
    #[test]
    fn bubbles_never_cover_what_they_point_at() {
        for &(name, sw, sh) in crate::layout::SAMPLE_SCREENS {
            let (cw, ch, _) = crate::layout::canvas(sw, sh);
            let lay = crate::layout::Layout::new(cw, ch, false);
            let on_screen = |r: Rect| r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= cw && r.y + r.h <= ch;
            for (i, s) in STEPS.iter().enumerate() {
                let spot = spot_rect(&lay, s.spot);
                assert_eq!(spot.is_none(), s.spot == Spot::None, "{name} step {i}");
                let b = bubble(spot, BUBBLE_MAX_H, (cw, ch));
                assert!(on_screen(b), "{name} step {i}: bubble off screen: {b:?}");
                assert!(b.w >= 360.0, "{name} step {i}: bubble too narrow ({})", b.w);
                if let Some(r) = spot {
                    assert!(on_screen(r), "{name} step {i}: spot off screen");
                    assert!(!b.overlaps(&r), "{name} step {i}: bubble covers the spot");
                }
                // Steps that wait for an action must point at what to click.
                assert_eq!(s.wait == Wait::Next, !allows(s, spot, spot.map_or(vec2(0.0, 0.0), |r| r.center())), "step {i}");
            }
        }
    }

    #[test]
    fn touch_screens_say_tap() {
        for (step, s) in STEPS.iter().enumerate() {
            let t = Tour { step, touch: true };
            let words = t.text().to_lowercase();
            assert!(!words.contains("click") && !words.contains("mouse"), "step {step}: {words}");
            // "clicking" would become "taping".
            assert!(!s.text.to_lowercase().contains("clicking"));
            // Long enough to fit Bit's tallest bubble with room to spare.
            assert!(t.text().len() <= 300, "step {step} is too long for its bubble");
        }
    }
}

/// The `count` variable: saves are untrusted, numbers stay in bounds, and
/// endless loops are caught. Nothing here can reach outside the game: the VM
/// only ever hands back one of five miner actions.
mod variables {
    use super::level;
    use crate::program::{simulate, Kind, Program, Step, Vm, MAX_VALUE};
    use crate::world::{Rng, World};

    fn run_count(src: &str) -> (u32, Option<String>) {
        let p = Program::deserialize(src).unwrap();
        let w = World::from_level(level("The Number Box"), 0);
        let mut vm = Vm::new(&p);
        let mut w2 = w.clone();
        loop {
            match vm.next(&w2) {
                Step::Act(a, _) => {
                    if !matches!(w2.apply(a), crate::world::Outcome::Ok) {
                        return (vm.count, None);
                    }
                }
                Step::Var(_) => {}
                Step::Done => return (vm.count, None),
                Step::Error(m, _) => return (vm.count, Some(m)),
            }
        }
    }

    #[test]
    fn numbers_outside_the_editor_s_range_are_rejected() {
        for bad in ["S21", "S-1", "S", "A0", "A10", "A99999999999", "S1x", "[V1 F ]", "[V F ]", "[V0 F", "A1 ]"] {
            assert!(Program::deserialize(bad).is_none(), "{bad} should not load");
        }
        for good in ["S0", "S20", "A1", "A9", "[V0 ]", "S3 [V0 [V0 F ] ]"] {
            assert!(Program::deserialize(good).is_some(), "{good} should load");
        }
    }

    #[test]
    fn count_never_goes_past_its_limit() {
        // 9 x 99 would be 891; the box stops at MAX_VALUE.
        let (count, err) = run_count("[R99 A9 ]");
        assert_eq!(count, MAX_VALUE);
        assert!(err.is_none());
        assert_eq!(run_count("S20 S0").0, 0);
    }

    #[test]
    fn loops_that_only_change_count_are_caught() {
        let (_, err) = run_count("[W0 A1 ]");
        assert!(err.is_some_and(|m| m.contains("count")), "an endless ADD loop must stop with a message");
        // A big REPEAT COUNT still stops at the action limit.
        let p = Program::deserialize("S20 [V0 [V0 [V0 L ] ] ]").unwrap();
        assert!(simulate(&p, World::from_level(level("The Number Box"), 0)).is_err());
    }

    #[test]
    fn repeat_count_runs_zero_times_when_count_is_zero() {
        let p = Program::deserialize("[V0 F ]").unwrap();
        let mut vm = Vm::new(&p);
        let w = World::from_level(level("The Number Box"), 0);
        assert!(matches!(vm.next(&w), Step::Done), "count starts at 0, so nothing runs");
    }

    /// Random junk and random mutations of real programs never panic the
    /// loader, and whatever loads runs to an end within the limits.
    #[test]
    fn random_saves_never_crash() {
        let toks = ["F", "L", "R", "M", "P", "C", "S", "S3", "S20", "S21", "A1", "A9", "A0", "[V0", "[V1", "[R3", "[R0", "[I3", "[E1", "[W0", "[W9", "|", "]", "/", "é", "[V0é", "S٣"];
        let mut rng = Rng::new(42);
        let world = World::from_level(level("Echo Hall"), 1);
        for _ in 0..3000 {
            let n = rng.below(14);
            let s: Vec<&str> = (0..n).map(|_| toks[rng.below(toks.len())]).collect();
            let src = s.join(" ");
            if let Some(p) = Program::deserialize(&src) {
                let _ = simulate(&p, world.clone());
                // Saving and loading again gives the same program.
                assert_eq!(Program::deserialize(&p.serialize()).map(|q| q.serialize()), Some(p.serialize()), "{src}");
            }
        }
    }

    #[test]
    fn rust_view_declares_and_borrows_count() {
        let p = Program::deserialize("S2 C [V0 F ] / A1").unwrap();
        let code = p.to_rust().join("\n");
        assert!(code.contains("let mut count = 0;"), "{code}");
        assert!(code.contains("my_function(&mut count);"), "{code}");
        assert!(code.contains("fn my_function(count: &mut u32) {"), "{code}");
        assert!(code.contains("*count += 1;"), "{code}");
        assert!(code.contains("for _ in 0..count {"), "{code}");
        // Programs without variables look exactly as before.
        let plain = Program::deserialize("F C / L").unwrap().to_rust().join("\n");
        assert!(!plain.contains("count") && plain.contains("fn my_function() {"), "{plain}");
    }

    /// Each level in the variables world needs the variable: the plain ways
    /// fail somewhere or don't fit in the block limit.
    #[test]
    fn variables_levels_need_the_variable() {
        let too_long = [
            ("The Number Box", "F F F F F F F"),
            ("Two Long Halls", "F F F F F R F F F F F"),
            ("Growing Stairs", "F L F R F F L F R F F F L F R F F F F"),
            // A lawnmower sweep finds the portal without counting, but needs more blocks.
            ("Echo Hall", "[R3 [W0 F ] R F R [W0 F ] L F L ]"),
            ("Stone Counter", "R [R3 [W0 F ] L F L [W0 F ] R F R ]"),
        ];
        for (name, src) in too_long {
            let l = level(name);
            let p = Program::deserialize(src).unwrap();
            assert!(p.block_count() > l.max_blocks.unwrap(), "{name}: {src} fits in the block limit");
        }
        for l in crate::levels::LEVELS.iter().filter(|l| l.chapter == 5) {
            let p = Program::deserialize(l.solution).unwrap();
            let mut used = Vec::new();
            crate::program::kinds_used(&p.main, &mut used);
            assert!(used.contains(&Kind::RepeatCount), "{}: the solution should use the variable", l.name);
        }
    }
}

mod panels {
    use crate::levels::LEVELS;
    use crate::ui::{Scrollbar, Ui, VH, VW};
    use macroquad::math::{vec2, Rect};

    fn ui_at(x: f32, y: f32, click: bool, down: bool) -> Ui {
        Ui { mouse: vec2(x, y), click, rclick: false, release: !down, down, wheel: 0.0, enabled: true, clicked_any: false }
    }

    /// The run controls have their own panel, so no block can hide behind them,
    /// and every level's blocks can be reached by scrolling.
    #[test]
    fn blocks_never_hide_behind_the_run_controls() {
        let lay = crate::layout::Layout::new(VW, VH, false);
        let (palette, list, controls) = (lay.palette.unwrap(), lay.palette_list().unwrap(), lay.controls);
        assert!(!palette.overlaps(&controls), "the blocks and the run controls must not overlap");
        for r in [palette, controls] {
            assert!(r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= VW && r.y + r.h <= VH);
        }
        for b in [lay.run, lay.step, lay.reset, lay.speed] {
            assert!(controls.contains(b.point()) && controls.contains(b.point() + b.size()), "{b:?} is outside the controls panel");
        }
        let rows = crate::play::palette_rows(list);
        for i in 0..rows {
            let b = crate::play::palette_button(list, i);
            assert!(b.y >= list.y && b.y + b.h <= list.y + list.h, "block {i} doesn't fit");
        }
        let most = LEVELS.iter().map(|l| l.palette.len()).max().unwrap();
        assert!(most > rows, "test the scrolling case: the biggest palette ({most}) should need it");
    }

    #[test]
    fn scrollbar_drags_pages_and_hides() {
        let track = Rect::new(100.0, 100.0, 14.0, 400.0);
        let mut bar = Scrollbar::default();
        let mut scroll = 0.0;
        // Everything fits: no bar, no scroll.
        assert!(bar.handle(&mut ui_at(107.0, 150.0, true, true), track, 300.0, 400.0, &mut scroll).is_none());
        // Click below the thumb: down a page.
        let thumb = bar.handle(&mut ui_at(0.0, 0.0, false, false), track, 1000.0, 400.0, &mut scroll).unwrap();
        assert_eq!(thumb.y, track.y);
        bar.handle(&mut ui_at(107.0, 490.0, true, true), track, 1000.0, 400.0, &mut scroll);
        assert_eq!(scroll, 320.0);
        // Grab the thumb and drag it to the bottom: the end of the list.
        let thumb = bar.handle(&mut ui_at(0.0, 0.0, false, false), track, 1000.0, 400.0, &mut scroll).unwrap();
        let grab_y = thumb.y + 5.0;
        bar.handle(&mut ui_at(107.0, grab_y, true, true), track, 1000.0, 400.0, &mut scroll);
        bar.handle(&mut ui_at(107.0, grab_y + 1000.0, false, true), track, 1000.0, 400.0, &mut scroll);
        assert_eq!(scroll, 600.0, "can't scroll past the end");
        // Let go, and the thumb stops following the mouse.
        bar.handle(&mut ui_at(107.0, 100.0, false, false), track, 1000.0, 400.0, &mut scroll);
        bar.handle(&mut ui_at(107.0, 100.0, false, false), track, 1000.0, 400.0, &mut scroll);
        assert_eq!(scroll, 600.0);
    }
}

mod layouts {
    use crate::layout::{canvas, Layout, Shape, MIN_H, MIN_SCALE, MIN_W, SAMPLE_SCREENS};
    use crate::play::palette_rows;
    use crate::ui::{VH, VW};
    use macroquad::math::Rect;

    /// At 1280x800 the layout is exactly the desktop one it replaced.
    #[test]
    fn desktop_layout_is_unchanged() {
        assert_eq!(canvas(1280.0, 800.0), (VW, VH, 1.0));
        let l = Layout::new(VW, VH, false);
        assert_eq!(l.shape, Shape::Wide);
        let eq = |a: Rect, b: (f32, f32, f32, f32)| assert_eq!((a.x, a.y, a.w, a.h), b);
        eq(l.world, (16.0, 68.0, 720.0, 480.0));
        eq(l.hud, (16.0, 558.0, 720.0, 38.0));
        eq(l.guide, (16.0, 604.0, 720.0, 180.0));
        eq(l.palette.unwrap(), (752.0, 68.0, 196.0, 548.0));
        eq(l.controls, (752.0, 628.0, 196.0, 156.0));
        eq(l.editor, (962.0, 68.0, 302.0, 716.0));
        eq(l.levels, (14.0, 10.0, 110.0, 36.0));
        eq(l.code, (800.0, 10.0, 150.0, 36.0));
        eq(l.help, (962.0, 10.0, 110.0, 36.0));
        eq(l.sound, (1084.0, 10.0, 180.0, 36.0));
        eq(l.hint, (120.0, 742.0, 110.0, 32.0));
        eq(l.goal, (242.0, 742.0, 110.0, 32.0));
        eq(l.run, (762.0, 638.0, 176.0, 50.0));
        // A running program doesn't move anything on the desktop.
        assert_eq!(format!("{l:?}"), format!("{:?}", Layout::new(VW, VH, true)));
    }

    /// Phones, tablets and small windows get readable text and a layout where
    /// everything fits, nothing overlaps, and buttons are big enough to tap.
    #[test]
    fn every_screen_gets_a_readable_layout() {
        for &(name, sw, sh) in SAMPLE_SCREENS {
            let (cw, ch, scale) = canvas(sw, sh);
            // Only screens smaller than the smallest canvas may go below the readable scale.
            // Only screens smaller than the smallest canvas go below the readable
            // scale, and never far below it.
            assert!(scale >= MIN_SCALE - 1e-4 || sw < MIN_W * MIN_SCALE || sh < MIN_H * MIN_SCALE, "{name}: scale {scale}");
            assert!(scale >= 0.7, "{name}: text would be too small (scale {scale})");
            assert!((cw * scale - sw).abs() < 1.0 || (ch * scale - sh).abs() < 1.0, "{name}: the canvas should fill the screen one way");
            for running in [false, true] {
                let l = Layout::new(cw, ch, running);
                let inside = |r: Rect| r.x >= 0.0 && r.y >= 0.0 && r.x + r.w <= cw + 0.01 && r.y + r.h <= ch + 0.01;
                let mut panels = vec![("world", l.world), ("hud", l.hud), ("guide", l.guide), ("controls", l.controls), ("editor", l.editor)];
                if let Some(p) = l.palette {
                    panels.push(("palette", p));
                }
                let top = [("levels", l.levels), ("code", l.code), ("help", l.help), ("sound", l.sound)];
                for (n, r) in panels.iter().chain(top.iter()) {
                    assert!(inside(*r), "{name}: {n} is off the canvas: {r:?}");
                }
                for (i, (a, ra)) in panels.iter().chain(top.iter()).enumerate() {
                    for (b, rb) in panels.iter().chain(top.iter()).skip(i + 1) {
                        assert!(!ra.overlaps(rb), "{name}: {a} overlaps {b}");
                    }
                }
                for (n, b) in top.iter().chain([("run", l.run), ("step", l.step), ("reset", l.reset), ("speed", l.speed), ("hint", l.hint)].iter()) {
                    assert!(b.h >= 32.0 && b.w >= 64.0, "{name}: {n} is too small to tap: {b:?}");
                    let holder = if ["run", "step", "reset", "speed"].contains(n) { l.controls } else if *n == "hint" { l.guide } else { Rect::new(0.0, 0.0, cw, l.top_h) };
                    assert!(holder.contains(b.point()) && holder.contains(b.point() + b.size()), "{name}: {n} sticks out of its panel");
                }
                // Popups fit.
                for (w, h, y) in [(760.0, 540.0, 120.0), (840.0, 660.0, 70.0), (680.0, 600.0, 130.0)] {
                    assert!(inside(l.popup(w, h, y)), "{name}: popup {w}x{h} doesn't fit");
                }
                // Screens narrower than today's phones (360 points) are best effort:
                // everything fits without overlapping, but panels may be cramped.
                if sw < 360.0 {
                    continue;
                }
                assert!(l.world.w >= 300.0 && l.world.h >= 160.0, "{name}: world too small {:?}", l.world);
                assert!(l.editor.h >= 250.0 && l.editor.w >= 260.0, "{name}: program panel too small {:?}", l.editor);
                match l.palette_list() {
                    Some(list) => assert!(palette_rows(list) >= 3, "{name}: only {} blocks fit", palette_rows(list)),
                    None => assert!(running && l.shape == Shape::Tall, "{name}: the blocks only step aside while running on a tall screen"),
                }
            }
        }
    }

    /// Phones held upright get the stacked layout; sideways and bigger get columns.
    #[test]
    fn shapes_follow_the_screen() {
        let shape = |sw, sh| {
            let (cw, ch, _) = canvas(sw, sh);
            Layout::new(cw, ch, false).shape
        };
        assert_eq!(shape(390.0, 844.0), Shape::Tall);
        assert_eq!(shape(844.0, 390.0), Shape::Wide);
        assert_eq!(shape(1024.0, 600.0), Shape::Wide);
        assert_eq!(shape(1920.0, 1080.0), Shape::Wide);
    }
}

/// Prints the real sizes of each layout, for the TypeSafe re-check:
/// `cargo test print_layouts -- --ignored --nocapture`.
#[test]
#[ignore]
fn print_layouts() {
    use crate::layout::{canvas, Layout};
    use crate::play::palette_rows;
    for (name, sw, sh) in [("phone_portrait", 390.0, 844.0), ("phone_landscape", 844.0, 390.0), ("small_tablet", 1024.0, 600.0)] {
        let (cw, ch, scale) = canvas(sw, sh);
        for running in [false, true] {
            let l = Layout::new(cw, ch, running);
            let rows = ((l.editor.h - 128.0) / 36.0).floor();
            println!(
                "{name} running={running} canvas={cw:.0}x{ch:.0} scale={scale:.2} shape={:?} world={:.0}x{:.0} blocks_visible={} program_w={:.0} program_rows={rows} message={:.0}x{:.0}",
                l.shape, l.world.w, l.world.h, l.palette_list().map_or(0, palette_rows), l.editor.w, l.guide.w - 116.0, l.guide.h - 24.0
            );
        }
    }
}
