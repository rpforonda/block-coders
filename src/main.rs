//! Block Coders - a blocky coding adventure that teaches kids to think like programmers.

mod art;
mod check;
mod editor;
mod fight;
mod hints;
mod items;
mod keyboard;
mod layout;
mod levels;
mod play;
mod program;
mod questions;
mod save;
mod sound;
mod tour;
mod ui;
mod users;
mod wardrobe;
mod world;
#[cfg(test)]
mod tests;

use art::{draw_tex, Art};
use check::{CheckOut, CheckScreen, CHECKS};
use levels::{CHAPTERS, LEVELS};
use macroquad::prelude::*;
use play::{Nav, Play};
use save::{Progress, User, Users};
use sound::{Sfx, Sounds};
use layout::Canvas;
use ui::*;
use users::{UsersOut, UsersScreen};
use wardrobe::{Wardrobe, WardrobeOut};
use items::Look;
use world::{Dir, Floor};

/// The window size: the desktop canvas, or BC_WINDOW's size in debug builds.
fn dev_window() -> (i32, i32) {
    dev_env("BC_WINDOW")
        .and_then(|s| {
            let (w, h) = s.split_once('x')?;
            Some((w.parse().ok()?, h.parse().ok()?))
        })
        .unwrap_or((VW as i32, VH as i32))
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Block Coders".to_string(),
        // BC_WINDOW=390x844 (developer setting) opens a phone-sized window.
        window_width: dev_window().0,
        window_height: dev_window().1,
        window_resizable: true,
        // Render at the device's real pixel density in browsers, so phones and
        // tablets (2-3x screens) get sharp text instead of an upscaled blur.
        // The main loop converts its letterbox viewport to physical pixels with
        // screen_dpi_scale() (macroquad does the same for text); input stays in
        // logical pixels.
        // Desktop keeps its current behaviour.
        high_dpi: cfg!(target_arch = "wasm32"),
        icon: Some(miniquad::conf::Icon {
            small: art::icon_rgba(16).try_into().unwrap(),
            medium: art::icon_rgba(32).try_into().unwrap(),
            big: art::icon_rgba(64).try_into().unwrap(),
        }),
        platform: miniquad::conf::Platform {
            // Must match StartupWMClass in packaging/block-coders.desktop.
            linux_wm_class: "block-coders",
            ..Default::default()
        },
        ..Default::default()
    }
}

enum Screen {
    Title,
    Users(UsersScreen),
    Select,
    Play(Box<Play>),
    Check(Box<CheckScreen>),
    Wardrobe(Wardrobe),
}

/// What the level select screen asks for.
enum SelectOut {
    Back,
    Wardrobe,
    Level(usize),
    Check(usize),
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h.wrapping_mul(0x85eb_ca6b)
}

/// A scrolling field of grass and trees behind the menus, filling the canvas.
fn menu_background(art: &Art, c: Canvas) {
    let ts = 64.0;
    let off = (get_time() * 12.0) as f32 % ts;
    for y in 0..(c.1 / ts) as i32 + 2 {
        for x in -1..(c.0 / ts) as i32 + 2 {
            let px = x as f32 * ts + off;
            let py = y as f32 * ts;
            draw_tex(art.floor(Floor::Grass), px, py, ts, ts, WHITE);
            let wx = x - (get_time() * 12.0 / ts as f64) as i32;
            let tile = match hash(wx, y) % 23 {
                0 | 1 => Some(world::Tile::Tree),
                2 => Some(world::Tile::Stone),
                3 => Some(world::Tile::Ore),
                _ => None,
            };
            if let Some(t) = tile.and_then(|t| art.tile(t, Floor::Grass, 0.0, false)) {
                draw_tex(t, px, py, ts, ts, WHITE);
            }
        }
    }
    draw_rectangle(0.0, 0.0, c.0, c.1, Color::new(0.0, 0.0, 0.0, 0.35));
}

fn title_screen(ui: &mut Ui, art: &Art, sfx: &Sounds, c: Canvas) -> bool {
    menu_background(art, c);
    let l = layout::title(c);
    let bob = (get_time() * 2.0).sin() as f32 * 6.0;
    let t = "BLOCK CODERS";
    let size = fit_size(t, c.0 - 40.0, l.title_size, 40.0);
    let w = text_width(t, size);
    let x = (c.0 - w) / 2.0;
    let shadow = 7.0 * size / 120.0;
    text(t, x + shadow, l.title_y + shadow + bob, size, rgb8(40, 25, 10));
    text(t, x, l.title_y + bob, size, rgb8(255, 210, 60));
    let sub = "A blocky coding adventure";
    text_centered(sub, l.subtitle, fit_size(sub, c.0 - 40.0, 34.0, 24.0), WHITE, true);
    draw_tex(art.player(0, Dir::Down), l.miner.x, l.miner.y, l.miner.w, l.miner.h, WHITE);
    draw_tex(&art.bit, l.bit.x, l.bit.y + bob, l.bit.w, l.bit.h, WHITE);
    let go = ui.button(l.play, "PLAY", rgb8(60, 170, 60), 40.0);
    let tagline = "Program a miner to dig, build and explore. Learn loops, functions, if/else and more!";
    // One line on wide screens (where it sits as it always did), wrapped on narrow ones.
    let t = l.tagline;
    for (i, line) in wrap(tagline, t.w, 22.0).iter().enumerate() {
        text_centered(line, Rect::new(0.0, t.y + i as f32 * 28.0, c.0, 30.0), 22.0, WHITE, true);
    }
    if go {
        sfx.play(Sfx::Click);
    }
    go
}

/// The world to show first: the one holding the furthest level the player can open.
fn current_world(progress: &Progress) -> usize {
    (0..LEVELS.len()).rev().find(|&i| progress.unlocked(i)).map_or(0, |i| LEVELS[i].chapter)
}

/// One world's button: its name, what it teaches and its stars, or why it's
/// locked. Short buttons get one line. Returns whether it was clicked (open worlds only).
fn world_button(ui: &mut Ui, progress: &Progress, ci: usize, r: Rect, chosen: bool) -> bool {
    let ch = &CHAPTERS[ci];
    let first_of = |ci: usize| LEVELS.iter().position(|l| l.chapter == ci).unwrap_or(0);
    let c = rgb8(ch.color.0, ch.color.1, ch.color.2);
    let open = progress.unlocked(first_of(ci));
    let levels: Vec<usize> = (0..LEVELS.len()).filter(|&i| LEVELS[i].chapter == ci).collect();
    let stars: u32 = levels.iter().map(|&i| progress.stars[i] as u32).sum();
    // The chosen world is drawn in its colour, made readable, with text to match.
    let fill = readable_bg(if chosen { c } else { art::shade(c, 0.5) });
    let ink = if !open { LOCKED_TEXT } else { text_on(fill) };
    // Accent colours only where they read: on the dark, unchosen worlds.
    let accent = |c: Color| if open && !chosen { c } else { ink };
    let mut clicked = false;
    if open {
        clicked = ui.button(r, "", fill, 20.0) && !chosen;
    } else {
        bevel(r, rgb8(60, 60, 60));
    }
    if chosen {
        draw_rectangle_lines(r.x - 5.0, r.y - 5.0, r.w + 10.0, r.h + 10.0, 4.0, rgb8(255, 230, 90));
    }
    let say = |s: &str, x: f32, y: f32, size: f32, c: Color| if c == DARK_TEXT { text(s, x, y, size, c) } else { shadow_text(s, x, y, size, c) };
    let why = if open {
        ch.concept.to_string()
    } else if ci == CHAPTERS.len() - 1 && progress.check_open(ci - 1) {
        "Pass every World Check".to_string()
    } else if ci > 0 && progress.check_open(ci - 1) {
        format!("Pass the World {ci} Check")
    } else {
        "Locked".to_string()
    };
    let why_color = if open { accent(rgb8(255, 230, 140)) } else { LOCKED_TEXT };
    if open {
        star(r.x + r.w - 92.0, r.y + 20.0, 10.0, true);
        say(&format!("{stars}/{}", levels.len() * 3), r.x + r.w - 78.0, r.y + 11.0, 18.0, accent(rgb8(255, 215, 40)));
    }
    if r.h >= 88.0 {
        say(&format!("World {}", ci + 1), r.x + 16.0, r.y + 10.0, 18.0, ink);
        say(ch.name, r.x + 16.0, r.y + 32.0, fit_size(ch.name, r.w - 120.0, 28.0, 20.0), ink);
        say(&why, r.x + 16.0, r.y + 64.0, 18.0, why_color);
    } else {
        // Short: the name on one line, then what it teaches (or why it's locked).
        let name = format!("{}. {}", ci + 1, ch.name);
        say(&name, r.x + 12.0, r.y + 6.0, fit_size(&name, r.w - 112.0, 22.0, 16.0), ink);
        if r.h >= 48.0 {
            say(&why, r.x + 12.0, r.y + r.h - 22.0, 16.0, why_color);
        }
    }
    clicked
}

/// One level's tile, scaled to its size (148 is the desktop's).
fn level_tile(ui: &mut Ui, progress: &Progress, i: usize, b: Rect) -> bool {
    let l = &LEVELS[i];
    let ch = &CHAPTERS[l.chapter];
    let t = b.w / 148.0;
    let boss = l.boss.is_some();
    // A boss level's red BOSS banner across the top (on locked tiles too, so
    // kids can see it coming). Its number and die move down out of the way.
    let banner = Rect::new(b.x + 6.0 * t, b.y + 6.0 * t, b.w - 12.0 * t, (30.0 * t).max(22.0));
    let below_banner = banner.y + banner.h - b.y;
    let draw_banner = || {
        draw_rectangle(banner.x, banner.y, banner.w, banner.h, BOSS_BANNER);
        draw_rectangle_lines(banner.x, banner.y, banner.w, banner.h, 2.0, BLACK);
        text_centered("BOSS", banner, (22.0 * t).max(16.0), WHITE, true);
    };
    if progress.unlocked(i) {
        let c = readable_bg(rgb8(ch.color.0, ch.color.1, ch.color.2));
        let clicked = ui.button(b, "", c, 20.0);
        let ink = text_on(c);
        let number_y = if boss { below_banner - 2.0 } else { 14.0 * t };
        text_centered(&format!("{}", i + 1), Rect::new(b.x, b.y + number_y, b.w, 60.0 * t), 56.0 * t, ink, ink == WHITE);
        for s in 0..3 {
            star(b.x + (40.0 + s as f32 * 34.0) * t, b.y + 108.0 * t, 14.0 * t, s < progress.stars[i] as usize);
        }
        if boss {
            draw_banner();
        }
        if l.is_random() {
            // A little die marks levels that change every run.
            let die_y = if boss { below_banner + 4.0 } else { 8.0 * t };
            let d = Rect::new(b.x + b.w - 30.0 * t, b.y + die_y, 22.0 * t, 22.0 * t);
            draw_rectangle(d.x, d.y, d.w, d.h, WHITE);
            draw_rectangle_lines(d.x, d.y, d.w, d.h, 2.0, BLACK);
            for (dx, dy) in [(6.0, 6.0), (11.0, 11.0), (16.0, 16.0)] {
                draw_rectangle(d.x + (dx - 2.0) * t, d.y + (dy - 2.0) * t, 4.0 * t, 4.0 * t, BLACK);
            }
        }
        clicked
    } else {
        bevel(b, rgb8(70, 70, 70));
        draw_rectangle_lines(b.x + 56.0 * t, b.y + 30.0 * t, 36.0 * t, 34.0 * t, 6.0 * t, rgb8(150, 150, 150));
        draw_rectangle(b.x + 48.0 * t, b.y + 56.0 * t, 52.0 * t, 44.0 * t, rgb8(150, 150, 150));
        text_centered(&format!("{}", i + 1), Rect::new(b.x, b.y + 112.0 * t, b.w, 24.0 * t), (22.0 * t).max(16.0), LOCKED_TEXT, false);
        if boss {
            draw_banner();
        }
        false
    }
}

/// A world's World Check tile, after its last level. Returns (clicked, hover text).
fn check_tile(ui: &mut Ui, progress: &Progress, world: usize, b: Rect) -> (bool, Option<String>) {
    let t = b.w / 148.0;
    let open = progress.check_open(world);
    let passed = progress.checks[world];
    let fill = readable_bg(if passed { rgb8(60, 150, 70) } else { rgb8(225, 175, 40) });
    let clicked = if open {
        ui.button(b, "", fill, 20.0)
    } else {
        bevel(b, rgb8(70, 70, 70));
        false
    };
    // A clipboard with ticks, in the tile's text colour.
    let ink = if open { text_on(fill) } else { LOCKED_TEXT };
    let (cx, cy) = (b.x + b.w / 2.0, b.y + 50.0 * t);
    draw_rectangle_lines(cx - 24.0 * t, cy - 30.0 * t, 48.0 * t, 60.0 * t, 4.0 * t, ink);
    draw_rectangle(cx - 10.0 * t, cy - 36.0 * t, 20.0 * t, 10.0 * t, ink);
    for k in 0..3 {
        let y = cy + (-14.0 + k as f32 * 15.0) * t;
        draw_line(cx - 14.0 * t, y, cx - 9.0 * t, y + 5.0 * t, 3.0 * t, ink);
        draw_line(cx - 9.0 * t, y + 5.0 * t, cx - 1.0 * t, y - 5.0 * t, 3.0 * t, ink);
        draw_rectangle(cx + 3.0 * t, y - 1.0 * t, 13.0 * t, 3.0 * t, ink);
    }
    let label = if passed { "Passed!" } else { "World Check" };
    let size = fit_size(label, b.w - 8.0, 22.0 * t.max(0.75), 14.0);
    text_centered(label, Rect::new(b.x, b.y + 100.0 * t, b.w, 30.0 * t), size, ink, ink == WHITE);
    let info = ui.hover(b).then(|| {
        let concept = CHAPTERS[world].concept.to_lowercase();
        if !open {
            "World Check: open this world first.".to_string()
        } else if passed {
            "World Check passed! Take it again any time for practice.".to_string()
        } else if !progress.finished_world(world) {
            // Wording picked with TypeSafe (see docs/DESIGN.md).
            format!(
                "World Check: answer {} questions about {concept} to move on. You can take it before finishing the levels!",
                check::ASKED
            )
        } else {
            format!("World Check: answer {} questions about {concept} to move on.", check::ASKED)
        }
    });
    (clicked && open, info)
}

/// The bottom line: what a hovered tile is, or how levels unlock.
fn level_info(hovered: Option<usize>, check_info: Option<String>) -> String {
    match hovered {
        _ if check_info.is_some() => check_info.unwrap_or_default(),
        Some(i) if LEVELS[i].is_random() => format!("Level {}: {}  (changes every time you play!)", i + 1, LEVELS[i].name),
        Some(i) => format!("Level {}: {}", i + 1, LEVELS[i].name),
        None => "Finish a level to unlock the next one.".to_string(),
    }
}

/// `levels_page`: on phones held upright, whether the chosen world's levels
/// are showing (or the list of worlds).
#[allow(clippy::too_many_arguments)]
fn select_screen(
    ui: &mut Ui,
    art: &Art,
    sfx: &Sounds,
    progress: &mut Progress,
    user: &User,
    world: &mut usize,
    levels_page: &mut bool,
    c: Canvas,
) -> Option<SelectOut> {
    menu_background(art, c);
    if is_key_pressed(KeyCode::U) && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) {
        progress.unlock_all = !progress.unlock_all;
    }
    let levels: Vec<usize> = (0..LEVELS.len()).filter(|&i| LEVELS[i].chapter == *world).collect();
    let tiles = levels.len() + usize::from(*world < CHECKS);
    let mut out = None;
    let total = progress.total_stars();
    if layout::is_tall(c) {
        // Phones held upright: a page of worlds, then a page of one world's levels.
        let l = layout::select_tall(c, CHAPTERS.len(), tiles);
        let ch = &CHAPTERS[*world];
        if !*levels_page {
            if ui.button(l.back, "< Players", rgb8(110, 110, 120), 20.0) {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Back);
            }
            if player_chip(ui, art, &progress.look(user.shirt), &user.name, l.player) {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Wardrobe);
            }
            star(c.0 - 100.0, 32.0, 14.0, true);
            shadow_text(&format!("{total}"), c.0 - 80.0, 20.0, 24.0, rgb8(255, 215, 40));
            shadow_text("Choose a world", 12.0, 66.0, 32.0, WHITE);
            for ci in 0..CHAPTERS.len() {
                if world_button(ui, progress, ci, l.worlds[ci], false) {
                    sfx.play(Sfx::Click);
                    *world = ci;
                    *levels_page = true;
                }
            }
            return out;
        }
        if ui.button(l.back, "< Worlds", rgb8(110, 110, 120), 20.0) {
            sfx.play(Sfx::Click);
            *levels_page = false;
        }
        let name = format!("World {}: {}", *world + 1, ch.name);
        shadow_text(&name, l.back.x + l.back.w + 14.0, 14.0, fit_size(&name, c.0 - l.back.w - 40.0, 26.0, 16.0), WHITE);
        shadow_text(&format!("Learn: {}", ch.concept), l.back.x + l.back.w + 14.0, 46.0, 18.0, rgb8(255, 230, 140));
        draw_rectangle(12.0, 74.0, c.0 - 24.0, 6.0, rgb8(ch.color.0, ch.color.1, ch.color.2));
        let mut hovered = None;
        for (n, &i) in levels.iter().enumerate() {
            let b = l.tile_rect(n);
            if level_tile(ui, progress, i, b) {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Level(i));
            }
            if ui.hover(b) && progress.unlocked(i) {
                hovered = Some(i);
            }
        }
        let mut check_info = None;
        if *world < CHECKS {
            let (clicked, info) = check_tile(ui, progress, *world, l.tile_rect(levels.len()));
            if clicked {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Check(*world));
            }
            check_info = info;
        }
        centered_paragraph(&level_info(hovered, check_info), l.info.x, l.info.y, l.info.w, 18.0, WHITE);
        return out;
    }

    let l = layout::select_wide(c, CHAPTERS.len(), tiles);
    if ui.button(l.back, "< Switch player", rgb8(110, 110, 120), 20.0) {
        sfx.play(Sfx::Click);
        out = Some(SelectOut::Back);
    }
    shadow_text("Choose a level", 214.0, 16.0, fit_size("Choose a level", l.player.x - 226.0, 44.0, 26.0), WHITE);
    if player_chip(ui, art, &progress.look(user.shirt), &user.name, l.player) {
        sfx.play(Sfx::Click);
        out = Some(SelectOut::Wardrobe);
    }
    star(c.0 - 210.0, 38.0, 18.0, true);
    shadow_text(&format!("{total} / {}", LEVELS.len() * 3), c.0 - 184.0, 24.0, 30.0, rgb8(255, 215, 40));

    // Worlds down the left side.
    for ci in 0..CHAPTERS.len() {
        if world_button(ui, progress, ci, l.worlds[ci], *world == ci) {
            sfx.play(Sfx::Click);
            *world = ci;
        }
    }

    // The chosen world's levels in a grid.
    let ch = &CHAPTERS[*world];
    let panel = l.panel;
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::new(0.0, 0.0, 0.0, 0.55));
    draw_rectangle(panel.x, panel.y, panel.w, 6.0, rgb8(ch.color.0, ch.color.1, ch.color.2));
    shadow_text(&format!("{}  -  Learn: {}", ch.name, ch.concept), panel.x + 24.0, panel.y + 22.0 * l.header_h / 80.0, 26.0, WHITE);
    let mut hovered = None;
    for (n, &i) in levels.iter().enumerate() {
        let b = l.tile_rect(n);
        if level_tile(ui, progress, i, b) {
            sfx.play(Sfx::Click);
            out = Some(SelectOut::Level(i));
        }
        if ui.hover(b) && progress.unlocked(i) {
            hovered = Some(i);
        }
    }
    // The World Check tile, after the world's last level.
    let mut check_info = None;
    if *world < CHECKS {
        let (clicked, info) = check_tile(ui, progress, *world, l.tile_rect(levels.len()));
        if clicked {
            sfx.play(Sfx::Click);
            out = Some(SelectOut::Check(*world));
        }
        check_info = info;
    }
    let info = level_info(hovered, check_info);
    text_centered(&info, l.info, fit_size(&info, l.info.w - 24.0, 24.0, 16.0), WHITE, true);
    out
}

/// The player's miner and name, as a button to the Wardrobe.
fn player_chip(ui: &mut Ui, art: &Art, look: &Look, name: &str, r: Rect) -> bool {
    let hover = ui.hover(r);
    draw_rectangle(r.x, r.y, r.w, r.h, if hover { Color::new(1.0, 1.0, 1.0, 0.16) } else { Color::new(0.0, 0.0, 0.0, 0.35) });
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, rgb8(255, 215, 40));
    let mh = r.h - 12.0;
    art.miner(look, Dir::Down, Rect::new(r.x + 10.0, r.y + 6.0, mh * 0.75, mh), WHITE);
    let x = r.x + mh * 0.75 + 22.0;
    let w = r.w - (x - r.x) - 8.0;
    let name_size = fit_size(name, w, r.h * 0.42, 14.0);
    shadow_text(name, x, r.y + r.h * 0.1, name_size, rgb8(120, 230, 220));
    let label = "Wardrobe >";
    shadow_text(label, x, r.y + r.h * 0.58, fit_size(label, w, r.h * 0.3, 12.0), rgb8(255, 215, 40));
    ui.clicked(r)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("--version") => println!("block-coders {}", env!("CARGO_PKG_VERSION")),
        Some("--write-icon") => {
            // Used by packaging/make-package.sh to make the menu icon.
            let path = args.get(2).map_or("block-coders.png", String::as_str);
            // export_png flips rows (it expects upside-down GPU captures), so pre-flip.
            let rgba = art::icon_rgba(256);
            let bytes = rgba.chunks(256 * 4).rev().flatten().copied().collect();
            let img = Image { bytes, width: 256, height: 256 };
            img.export_png(path);
        }
        Some(other) => {
            eprintln!("Block Coders - a coding adventure game.\n\nUsage: block-coders [--version | --write-icon <file.png>]");
            if other != "--help" {
                std::process::exit(2);
            }
        }
        None => macroquad::Window::from_config(window_conf(), run()),
    }
}

/// Reads a developer-only setting. Release builds (the installed game) ignore
/// these, so nobody can make a player's copy drive itself or write screenshots.
pub fn dev_env(name: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(name).ok() } else { None }
}

async fn run() {
    keyboard::detect_touch();
    let art = Art::new();
    load_fonts();
    let mut sfx = Sounds::load().await;
    // Developer autopilot: BC_SHOT=<dir> saves screenshots of a level being solved.
    let shot_dir = dev_env("BC_SHOT").filter(|dir| {
        // Say so loudly: a BC_SHOT left in the shell (e.g. a fish universal
        // variable) would otherwise turn every `cargo run` into the autopilot.
        eprintln!("BC_SHOT={dir}: running the screenshot autopilot. Saves are read-only. Unset BC_SHOT to play normally.");
        std::fs::create_dir_all(dir).map_err(|e| eprintln!("BC_SHOT: can't create {dir}: {e}")).is_ok()
    });
    // Before anything loads: loading can write too (an old single-player save
    // is migrated to "Player 1" on the first load).
    if shot_dir.is_some() {
        save::set_read_only(true);
    }
    let mut users = Users::load();
    let mut user: Option<User> = None;
    let mut progress = Progress::new(None);
    let mut screen = Screen::Title;
    let mut speed = 1;
    let mut world_sel = 0;
    // On phones held upright: whether the level select shows a world's levels (or the worlds).
    let mut levels_page = true;

    // BC_CHECK=<n> opens World Check n (0-based) instead of a level.
    // BC_WARDROBE=<tab> opens the Wardrobe on that tab (0-based) instead.
    let shot_wardrobe: Option<usize> = dev_env("BC_WARDROBE").and_then(|s| s.parse().ok());
    let shot_check: Option<usize> = dev_env("BC_CHECK").and_then(|s| s.parse().ok()).filter(|&c| c < CHECKS);
    let shot_level: usize = dev_env("BC_LEVEL").and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut frame: u32 = 0;
    let mut finished_at: Option<u32> = None;
    // BC_INPUT="x,y,act;..." plays one mouse step per frame once the level's
    // program is loaded. act = move | down | up | shot | dump.
    let input_script: Vec<(f32, f32, String)> = dev_env("BC_INPUT")
        .unwrap_or_default()
        .split(';')
        .filter_map(|step| {
            let mut it = step.split(',');
            Some((it.next()?.trim().parse().ok()?, it.next()?.trim().parse().ok()?, it.next()?.trim().to_string()))
        })
        .collect();
    let mut held = false;
    let mut pending_shot = None;
    let shot_click: Option<(f32, f32)> = dev_env("BC_CLICK").and_then(|s| {
        let (x, y) = s.split_once(',')?;
        Some((x.parse().ok()?, y.parse().ok()?))
    });

    loop {
        let (sw, sh) = (screen_width(), screen_height());
        // Every screen reshapes for small screens (see layout.rs).
        let (cw, ch, scale) = layout::canvas(sw, sh);
        let (vw, vh) = (cw * scale, ch * scale);
        let (ox, oy) = ((sw - vw) / 2.0, (sh - vh) / 2.0);
        // screen_width() and mouse_position() are in logical pixels, but the
        // viewport works in physical pixels (2-3x on phones with high_dpi). dpi
        // is 1.0 on desktop. Text gets only `scale`: macroquad already multiplies
        // font sizes by dpi, and doing it here too rasterized every glyph at dpi²,
        // overfilling the font atlas (its regrowth trips a macroquad bug).
        let dpi = screen_dpi_scale();
        set_scale(scale);
        set_default_camera();
        clear_background(BLACK);
        set_camera(&Camera2D {
            target: vec2(cw / 2.0, ch / 2.0),
            zoom: vec2(2.0 / cw, 2.0 / ch),
            viewport: Some(((ox * dpi) as i32, (oy * dpi) as i32, (vw * dpi) as i32, (vh * dpi) as i32)),
            ..Default::default()
        });
        let (mx, my) = mouse_position();
        let mut ui = Ui::new(vec2((mx - ox) / scale, (my - oy) / scale));
        keyboard::note_touches();
        if let Some((x, y, act)) = frame.checked_sub(50).and_then(|i| input_script.get(i as usize)) {
            ui.mouse = vec2(*x, *y);
            match act.as_str() {
                "down" => {
                    ui.click = true;
                    held = true;
                }
                "up" => {
                    ui.release = true;
                    held = false;
                }
                "shot" => pending_shot = Some(format!("input-{}", frame - 50)),
                "dump" => {
                    if let Screen::Play(p) = &screen {
                        println!("program: {}", p.program.serialize());
                    }
                }
                _ => {}
            }
            ui.down = held;
        }
        // Autopilot: BC_CLICK=x,y clicks there 10 frames after the run finishes.
        if let (Some(f), Some((x, y))) = (finished_at, &shot_click) {
            if frame == f + 10 {
                ui.mouse = vec2(*x, *y);
                ui.click = true;
            }
        }

        let mut next = None;
        match &mut screen {
            Screen::Title => {
                if title_screen(&mut ui, &art, &sfx, (cw, ch)) {
                    next = Some(Screen::Users(UsersScreen::new(&mut users)));
                }
            }
            Screen::Users(u) => {
                menu_background(&art, (cw, ch));
                match u.frame(&mut ui, &art, &sfx, &mut users, (cw, ch)) {
                    UsersOut::Stay => {}
                    UsersOut::Back => next = Some(Screen::Title),
                    UsersOut::Picked(id) => {
                        user = users.list.iter().find(|u| u.id == id).cloned();
                        progress = Progress::load(id);
                        world_sel = current_world(&progress);
                        levels_page = true;
                        next = Some(Screen::Select);
                    }
                }
            }
            Screen::Select => {
                let me = user.clone().unwrap_or(User { id: 0, name: "Guest".into(), shirt: 0 });
                let look = progress.look(me.shirt);
                match select_screen(&mut ui, &art, &sfx, &mut progress, &me, &mut world_sel, &mut levels_page, (cw, ch)) {
                    Some(SelectOut::Back) => next = Some(Screen::Users(UsersScreen::new(&mut users))),
                    Some(SelectOut::Wardrobe) => next = Some(Screen::Wardrobe(Wardrobe::new())),
                    Some(SelectOut::Level(i)) => {
                        next = Some(Screen::Play(Box::new(Play::new(i, progress.programs[i].clone(), speed, look))))
                    }
                    Some(SelectOut::Check(c)) => next = Some(Screen::Check(Box::new(CheckScreen::new(c, &mut progress, look)))),
                    None => {}
                }
            }
            Screen::Wardrobe(w) => {
                menu_background(&art, (cw, ch));
                let me = user.clone().unwrap_or(User { id: 0, name: "Guest".into(), shirt: 0 });
                match w.frame(&mut ui, &art, &sfx, &mut progress, &me, (cw, ch)) {
                    WardrobeOut::Stay => {}
                    WardrobeOut::Back => next = Some(Screen::Select),
                }
            }
            Screen::Play(p) => match p.frame(&mut ui, &art, &mut sfx, &mut progress, (cw, ch)) {
                Nav::Stay => {}
                Nav::Back => {
                    speed = p.speed();
                    world_sel = LEVELS[p.idx].chapter;
                    next = Some(Screen::Select);
                }
                Nav::Goto(i) => {
                    speed = p.speed();
                    let look = progress.look(user.as_ref().map_or(0, |u| u.shirt));
                    next = Some(Screen::Play(Box::new(Play::new(i, progress.programs[i].clone(), speed, look))));
                }
                Nav::Check(c) => {
                    speed = p.speed();
                    let look = progress.look(user.as_ref().map_or(0, |u| u.shirt));
                    next = Some(Screen::Check(Box::new(CheckScreen::new(c, &mut progress, look))));
                }
            },
            Screen::Check(c) => match c.frame(&mut ui, &art, &sfx, &mut progress, (cw, ch)) {
                CheckOut::Stay => {}
                CheckOut::Back => {
                    world_sel = c.chapter;
                    next = Some(Screen::Select);
                }
                CheckOut::World(w) => {
                    world_sel = w;
                    next = Some(Screen::Select);
                }
            },
        }
        if let Some(s) = next {
            screen = s;
        }

        if let Some(dir) = &shot_dir {
            frame += 1;
            let shot = |name: &str| get_screen_data().export_png(&format!("{dir}/{name}.png"));
            match frame {
                15 => {
                    shot("1-title");
                    screen = Screen::Users(UsersScreen::new(&mut users));
                }
                22 => {
                    shot("2a-users");
                    if let Screen::Users(u) = &mut screen {
                        u.mode = users::Mode::New { name: "Sam".into(), shirt: 2 };
                    }
                }
                26 => {
                    shot("2b-new-player");
                    if let (Screen::Users(u), Some(first)) = (&mut screen, users.list.first()) {
                        u.mode = users::Mode::ConfirmReset(first.id);
                    }
                }
                28 => {
                    shot("2c-confirm-reset");
                    user = users.list.first().cloned();
                    if let Some(u) = &user {
                        progress = Progress::load(u.id);
                        progress.user = None; // never save autopilot runs
                        world_sel = current_world(&progress);
                    }
                    if let Some(c) = shot_check {
                        world_sel = c;
                    }
                    screen = Screen::Select;
                }
                30 => {
                    shot("2d-select");
                    let look = progress.look(4);
                    screen = match shot_check {
                        _ if shot_wardrobe.is_some() => Screen::Wardrobe(Wardrobe::with_tab(shot_wardrobe.unwrap_or(0))),
                        Some(c) => {
                            let mut check = CheckScreen::new(c, &mut progress, look);
                            // BC_QUESTION=<id> starts the check on that question.
                            if let Some(id) = dev_env("BC_QUESTION") {
                                check.debug_question(&id);
                            }
                            Screen::Check(Box::new(check))
                        }
                        None => Screen::Play(Box::new(Play::new(shot_level, None, 2, look))),
                    };
                }
                45 => {
                    shot("3-intro");
                    match &mut screen {
                        // BC_TOUR=1 keeps level 1's guided tour up for BC_INPUT to click through.
                        Screen::Play(p) if dev_env("BC_TOUR").is_none() => p.debug_solve(),
                        // A World Check has nothing else to show unless BC_INPUT drives it.
                        Screen::Check(_) | Screen::Wardrobe(_) if input_script.is_empty() => break,
                        _ => {}
                    }
                }
                f if f >= 50 && !input_script.is_empty() => {
                    if let Some(name) = pending_shot.take() {
                        shot(&name);
                    }
                    if f as usize >= 50 + input_script.len() {
                        break;
                    }
                }
                60 => {
                    shot("4-editor");
                    if let Screen::Play(p) = &mut screen {
                        p.debug_code();
                    }
                }
                75 => {
                    shot("5-code");
                    if let Screen::Play(p) = &mut screen {
                        p.debug_run(&sfx);
                    }
                }
                90 => shot("6-running"),
                f if f > 90 && f < 3000 => {
                    if let Screen::Play(p) = &screen {
                        match finished_at {
                            None if p.debug_finished() => {
                                shot("7-finished");
                                finished_at = Some(f);
                                if shot_click.is_none() {
                                    break;
                                }
                            }
                            Some(done) if f == done + 30 => {
                                shot("8-after-click");
                                break;
                            }
                            _ => {}
                        }
                    }
                }
                3000 => break,
                _ => {}
            }
        }

        next_frame().await;
    }
}
