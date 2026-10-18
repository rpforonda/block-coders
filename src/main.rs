//! Block Coders - a blocky coding adventure that teaches kids to think like programmers.

mod art;
mod check;
mod editor;
mod keyboard;
mod levels;
mod play;
mod program;
mod questions;
mod save;
mod sound;
mod ui;
mod users;
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
use ui::*;
use users::{UsersOut, UsersScreen};
use world::{Dir, Floor};

fn window_conf() -> Conf {
    Conf {
        window_title: "Block Coders".to_string(),
        window_width: VW as i32,
        window_height: VH as i32,
        window_resizable: true,
        // Render at the device's real pixel density in browsers, so phones and
        // tablets (2-3x screens) get sharp text instead of an upscaled blur.
        // The main loop converts its letterbox viewport and text size to physical
        // pixels with screen_dpi_scale(); input stays in logical pixels.
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
}

/// What the level select screen asks for.
enum SelectOut {
    Back,
    Level(usize),
    Check(usize),
}

fn hash(x: i32, y: i32) -> u32 {
    let mut h = (x as u32).wrapping_mul(0x27d4_eb2d) ^ (y as u32).wrapping_mul(0x1656_67b1);
    h ^= h >> 15;
    h.wrapping_mul(0x85eb_ca6b)
}

/// A scrolling field of grass and trees behind the menus.
fn menu_background(art: &Art) {
    let ts = 64.0;
    let off = (get_time() * 12.0) as f32 % ts;
    for y in 0..14 {
        for x in -1..22 {
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
    draw_rectangle(0.0, 0.0, VW, VH, Color::new(0.0, 0.0, 0.0, 0.35));
}

fn title_screen(ui: &mut Ui, art: &Art, sfx: &Sounds) -> bool {
    menu_background(art);
    let bob = (get_time() * 2.0).sin() as f32 * 6.0;
    let t = "BLOCK CODERS";
    let size = 120.0;
    let w = text_width(t, size);
    let x = (VW - w) / 2.0;
    text(t, x + 7.0, 147.0 + bob, size, rgb8(40, 25, 10));
    text(t, x, 140.0 + bob, size, rgb8(255, 210, 60));
    text_centered("A blocky coding adventure", Rect::new(0.0, 290.0, VW, 40.0), 34.0, WHITE, true);
    draw_tex(art.player(0, Dir::Down), VW / 2.0 - 230.0, 360.0, 96.0, 128.0, WHITE);
    draw_tex(&art.bit, VW / 2.0 + 140.0, 380.0 + bob, 96.0, 96.0, WHITE);
    let go = ui.button(Rect::new(VW / 2.0 - 120.0, 400.0, 240.0, 70.0), "PLAY", rgb8(60, 170, 60), 40.0);
    text_centered(
        "Program a miner to dig, build and explore. Learn loops, functions, if/else and more!",
        Rect::new(0.0, 540.0, VW, 30.0),
        22.0,
        WHITE,
        true,
    );
    if go {
        sfx.play(Sfx::Click);
    }
    go
}

/// The world to show first: the one holding the furthest level the player can open.
fn current_world(progress: &Progress) -> usize {
    (0..LEVELS.len()).rev().find(|&i| progress.unlocked(i)).map_or(0, |i| LEVELS[i].chapter)
}

fn select_screen(
    ui: &mut Ui,
    art: &Art,
    sfx: &Sounds,
    progress: &mut Progress,
    user: &User,
    world: &mut usize,
) -> Option<SelectOut> {
    menu_background(art);
    let mut out = None;
    if ui.button(Rect::new(20.0, 18.0, 170.0, 40.0), "< Switch player", rgb8(110, 110, 120), 20.0) {
        sfx.play(Sfx::Click);
        out = Some(SelectOut::Back);
    }
    shadow_text("Choose a level", 214.0, 16.0, 44.0, WHITE);
    draw_tex(art.player(user.shirt, Dir::Down), 640.0, 14.0, 36.0, 48.0, WHITE);
    shadow_text(&user.name, 686.0, 24.0, 30.0, rgb8(120, 230, 220));
    let total = progress.total_stars();
    star(VW - 210.0, 38.0, 18.0, true);
    shadow_text(&format!("{total} / {}", LEVELS.len() * 3), VW - 184.0, 24.0, 30.0, rgb8(255, 215, 40));
    if is_key_pressed(KeyCode::U) && (is_key_down(KeyCode::LeftControl) || is_key_down(KeyCode::RightControl)) {
        progress.unlock_all = !progress.unlock_all;
    }

    // Worlds down the left side.
    let first_of = |ci: usize| LEVELS.iter().position(|l| l.chapter == ci).unwrap_or(0);
    for (ci, ch) in CHAPTERS.iter().enumerate() {
        let r = Rect::new(20.0, 84.0 + ci as f32 * 112.0, 330.0, 100.0);
        let c = rgb8(ch.color.0, ch.color.1, ch.color.2);
        let open = progress.unlocked(first_of(ci));
        let levels: Vec<usize> = (0..LEVELS.len()).filter(|&i| LEVELS[i].chapter == ci).collect();
        let stars: u32 = levels.iter().map(|&i| progress.stars[i] as u32).sum();
        let chosen = *world == ci;
        if open {
            let fill = if chosen { c } else { art::shade(c, 0.55) };
            if ui.button(r, "", fill, 20.0) && !chosen {
                sfx.play(Sfx::Click);
                *world = ci;
            }
        } else {
            bevel(r, rgb8(60, 60, 60));
        }
        if chosen {
            draw_rectangle_lines(r.x - 5.0, r.y - 5.0, r.w + 10.0, r.h + 10.0, 4.0, rgb8(255, 230, 90));
        }
        let tc = if open { WHITE } else { rgb8(150, 150, 150) };
        shadow_text(&format!("World {}", ci + 1), r.x + 16.0, r.y + 10.0, 18.0, tc);
        shadow_text(ch.name, r.x + 16.0, r.y + 32.0, 28.0, tc);
        if open {
            shadow_text(ch.concept, r.x + 16.0, r.y + 68.0, 18.0, rgb8(255, 230, 140));
            star(r.x + r.w - 92.0, r.y + 20.0, 10.0, true);
            shadow_text(&format!("{stars}/{}", levels.len() * 3), r.x + r.w - 78.0, r.y + 11.0, 18.0, rgb8(255, 215, 40));
        } else {
            // Say why: a World Check to pass, or just levels still to beat.
            let why = if ci == CHAPTERS.len() - 1 && progress.check_open(ci - 1) {
                "Pass every World Check".to_string()
            } else if ci > 0 && progress.check_open(ci - 1) {
                format!("Pass the World {ci} Check")
            } else {
                "Locked".to_string()
            };
            shadow_text(&why, r.x + 16.0, r.y + 68.0, 18.0, rgb8(150, 150, 150));
        }
    }

    // The chosen world's levels in a grid.
    let ch = &CHAPTERS[*world];
    let panel = Rect::new(380.0, 84.0, 880.0, 660.0);
    draw_rectangle(panel.x, panel.y, panel.w, panel.h, Color::new(0.0, 0.0, 0.0, 0.55));
    draw_rectangle(panel.x, panel.y, panel.w, 6.0, rgb8(ch.color.0, ch.color.1, ch.color.2));
    shadow_text(&format!("{}  -  Learn: {}", ch.name, ch.concept), panel.x + 24.0, panel.y + 22.0, 26.0, WHITE);
    let mut hovered = None;
    let levels: Vec<usize> = (0..LEVELS.len()).filter(|&i| LEVELS[i].chapter == *world).collect();
    for (n, &i) in levels.iter().enumerate() {
        let l = &LEVELS[i];
        let b = Rect::new(panel.x + 30.0 + (n % 5) as f32 * 168.0, panel.y + 80.0 + (n / 5) as f32 * 180.0, 148.0, 148.0);
        if progress.unlocked(i) {
            let c = rgb8(ch.color.0, ch.color.1, ch.color.2);
            if ui.button(b, "", c, 20.0) {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Level(i));
            }
            text_centered(&format!("{}", i + 1), Rect::new(b.x, b.y + 14.0, b.w, 60.0), 56.0, WHITE, true);
            for s in 0..3 {
                star(b.x + 40.0 + s as f32 * 34.0, b.y + 108.0, 14.0, s < progress.stars[i] as usize);
            }
            if l.is_random() {
                // A little die marks levels that change every run.
                let d = Rect::new(b.x + b.w - 30.0, b.y + 8.0, 22.0, 22.0);
                draw_rectangle(d.x, d.y, d.w, d.h, WHITE);
                draw_rectangle_lines(d.x, d.y, d.w, d.h, 2.0, BLACK);
                for (dx, dy) in [(6.0, 6.0), (11.0, 11.0), (16.0, 16.0)] {
                    draw_rectangle(d.x + dx - 2.0, d.y + dy - 2.0, 4.0, 4.0, BLACK);
                }
            }
            if ui.hover(b) {
                hovered = Some(i);
            }
        } else {
            bevel(b, rgb8(70, 70, 70));
            draw_rectangle_lines(b.x + 56.0, b.y + 30.0, 36.0, 34.0, 6.0, rgb8(150, 150, 150));
            draw_rectangle(b.x + 48.0, b.y + 56.0, 52.0, 44.0, rgb8(150, 150, 150));
            text_centered(&format!("{}", i + 1), Rect::new(b.x, b.y + 112.0, b.w, 24.0), 22.0, rgb8(180, 180, 180), false);
        }
    }
    // The World Check tile, after the world's last level.
    let mut check_info = None;
    if *world < CHECKS {
        let n = levels.len();
        let b = Rect::new(panel.x + 30.0 + (n % 5) as f32 * 168.0, panel.y + 80.0 + (n / 5) as f32 * 180.0, 148.0, 148.0);
        let open = progress.check_open(*world);
        let passed = progress.checks[*world];
        if open {
            let fill = if passed { rgb8(60, 150, 70) } else { rgb8(225, 175, 40) };
            if ui.button(b, "", fill, 20.0) {
                sfx.play(Sfx::Click);
                out = Some(SelectOut::Check(*world));
            }
        } else {
            bevel(b, rgb8(70, 70, 70));
        }
        // A clipboard with ticks.
        let ink = if open { WHITE } else { rgb8(150, 150, 150) };
        let (cx, cy) = (b.x + b.w / 2.0, b.y + 50.0);
        draw_rectangle_lines(cx - 24.0, cy - 30.0, 48.0, 60.0, 4.0, ink);
        draw_rectangle(cx - 10.0, cy - 36.0, 20.0, 10.0, ink);
        for k in 0..3 {
            let y = cy - 14.0 + k as f32 * 15.0;
            draw_line(cx - 14.0, y, cx - 9.0, y + 5.0, 3.0, ink);
            draw_line(cx - 9.0, y + 5.0, cx - 1.0, y - 5.0, 3.0, ink);
            draw_rectangle(cx + 3.0, y - 1.0, 13.0, 3.0, ink);
        }
        let label = if passed { "Passed!" } else { "World Check" };
        text_centered(label, Rect::new(b.x, b.y + 100.0, b.w, 30.0), 22.0, ink, open);
        if ui.hover(b) {
            check_info = Some(if !open {
                "World Check: beat every level in this world first.".to_string()
            } else if passed {
                "World Check passed! Take it again any time for practice.".to_string()
            } else {
                format!("World Check: answer {} questions about {} to move on.", check::ASKED, ch.concept.to_lowercase())
            });
        }
    }
    let info = match hovered {
        _ if check_info.is_some() => check_info.unwrap_or_default(),
        Some(i) if LEVELS[i].is_random() => format!("Level {}: {}  (changes every time you play!)", i + 1, LEVELS[i].name),
        Some(i) => format!("Level {}: {}", i + 1, LEVELS[i].name),
        None => "Finish a level to unlock the next one.".to_string(),
    };
    text_centered(&info, Rect::new(0.0, 760.0, VW, 28.0), 24.0, WHITE, true);
    out
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
fn dev_env(name: &str) -> Option<String> {
    if cfg!(debug_assertions) { std::env::var(name).ok() } else { None }
}

async fn run() {
    keyboard::detect_touch();
    let art = Art::new();
    let mut sfx = Sounds::load().await;
    let mut users = Users::load();
    let mut user: Option<User> = None;
    let mut progress = Progress::new(None);
    let mut screen = Screen::Title;
    let mut speed = 1;
    let mut world_sel = 0;

    // Developer autopilot: BC_SHOT=<dir> saves screenshots of a level being solved.
    let shot_dir = dev_env("BC_SHOT");
    // BC_CHECK=<n> opens World Check n (0-based) instead of a level.
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
        let scale = (sw / VW).min(sh / VH);
        let (vw, vh) = (VW * scale, VH * scale);
        let (ox, oy) = ((sw - vw) / 2.0, (sh - vh) / 2.0);
        // screen_width() and mouse_position() are in logical pixels, but the
        // viewport and text rasterizing work in physical pixels (2-3x on phones
        // with high_dpi). dpi is 1.0 on desktop.
        let dpi = screen_dpi_scale();
        set_scale(scale * dpi);
        set_default_camera();
        clear_background(BLACK);
        set_camera(&Camera2D {
            target: vec2(VW / 2.0, VH / 2.0),
            zoom: vec2(2.0 / VW, 2.0 / VH),
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
                if title_screen(&mut ui, &art, &sfx) {
                    next = Some(Screen::Users(UsersScreen::new(&users)));
                }
            }
            Screen::Users(u) => {
                menu_background(&art);
                match u.frame(&mut ui, &art, &sfx, &mut users) {
                    UsersOut::Stay => {}
                    UsersOut::Back => next = Some(Screen::Title),
                    UsersOut::Picked(id) => {
                        user = users.list.iter().find(|u| u.id == id).cloned();
                        progress = Progress::load(id);
                        world_sel = current_world(&progress);
                        next = Some(Screen::Select);
                    }
                }
            }
            Screen::Select => {
                let me = user.clone().unwrap_or(User { id: 0, name: "Guest".into(), shirt: 0 });
                match select_screen(&mut ui, &art, &sfx, &mut progress, &me, &mut world_sel) {
                    Some(SelectOut::Back) => next = Some(Screen::Users(UsersScreen::new(&users))),
                    Some(SelectOut::Level(i)) => {
                        next = Some(Screen::Play(Box::new(Play::new(i, progress.programs[i].clone(), speed, me.shirt))))
                    }
                    Some(SelectOut::Check(c)) => next = Some(Screen::Check(Box::new(CheckScreen::new(c, &mut progress, me.shirt)))),
                    None => {}
                }
            }
            Screen::Play(p) => match p.frame(&mut ui, &art, &mut sfx, &mut progress) {
                Nav::Stay => {}
                Nav::Back => {
                    speed = p.speed();
                    world_sel = LEVELS[p.idx].chapter;
                    next = Some(Screen::Select);
                }
                Nav::Goto(i) => {
                    speed = p.speed();
                    let shirt = user.as_ref().map_or(0, |u| u.shirt);
                    next = Some(Screen::Play(Box::new(Play::new(i, progress.programs[i].clone(), speed, shirt))));
                }
                Nav::Check(c) => {
                    speed = p.speed();
                    let shirt = user.as_ref().map_or(0, |u| u.shirt);
                    next = Some(Screen::Check(Box::new(CheckScreen::new(c, &mut progress, shirt))));
                }
            },
            Screen::Check(c) => match c.frame(&mut ui, &art, &sfx, &mut progress) {
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
                    screen = Screen::Users(UsersScreen::new(&users));
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
                    screen = match shot_check {
                        Some(c) => Screen::Check(Box::new(CheckScreen::new(c, &mut progress, 4))),
                        None => Screen::Play(Box::new(Play::new(shot_level, None, 2, 4))),
                    };
                }
                45 => {
                    shot("3-intro");
                    if let Screen::Play(p) = &mut screen {
                        p.debug_solve();
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
