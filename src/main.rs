//! Block Coders - a blocky coding adventure that teaches kids to think like programmers.

mod art;
mod editor;
mod levels;
mod play;
mod program;
mod save;
mod sound;
mod ui;
mod users;
mod world;
#[cfg(test)]
mod tests;

use art::{draw_tex, Art};
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

/// Returns the level to open, or Some(usize::MAX) to go back to the player list.
fn select_screen(
    ui: &mut Ui,
    art: &Art,
    sfx: &Sounds,
    progress: &mut Progress,
    user: &User,
    world: &mut usize,
) -> Option<usize> {
    menu_background(art);
    let mut out = None;
    if ui.button(Rect::new(20.0, 18.0, 170.0, 40.0), "< Switch player", rgb8(110, 110, 120), 20.0) {
        sfx.play(Sfx::Click);
        out = Some(usize::MAX);
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
            shadow_text("Locked", r.x + 16.0, r.y + 68.0, 18.0, rgb8(150, 150, 150));
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
                out = Some(i);
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
    let info = match hovered {
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

async fn run() {
    let art = Art::new();
    let mut sfx = Sounds::load().await;
    let mut users = Users::load();
    let mut user: Option<User> = None;
    let mut progress = Progress::new(None);
    let mut screen = Screen::Title;
    let mut speed = 1;
    let mut world_sel = 0;

    // Developer autopilot: BC_SHOT=<dir> saves screenshots of a level being solved.
    let shot_dir = std::env::var("BC_SHOT").ok();
    let shot_level: usize = std::env::var("BC_LEVEL").ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    let mut frame: u32 = 0;
    let mut finished_at: Option<u32> = None;
    let shot_click: Option<(f32, f32)> = std::env::var("BC_CLICK").ok().and_then(|s| {
        let (x, y) = s.split_once(',')?;
        Some((x.parse().ok()?, y.parse().ok()?))
    });

    loop {
        let (sw, sh) = (screen_width(), screen_height());
        let scale = (sw / VW).min(sh / VH);
        let (vw, vh) = (VW * scale, VH * scale);
        let (ox, oy) = ((sw - vw) / 2.0, (sh - vh) / 2.0);
        set_scale(scale);
        set_default_camera();
        clear_background(BLACK);
        set_camera(&Camera2D {
            target: vec2(VW / 2.0, VH / 2.0),
            zoom: vec2(2.0 / VW, 2.0 / VH),
            viewport: Some((ox as i32, oy as i32, vw as i32, vh as i32)),
            ..Default::default()
        });
        let (mx, my) = mouse_position();
        let mut ui = Ui::new(vec2((mx - ox) / scale, (my - oy) / scale));
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
                    Some(usize::MAX) => next = Some(Screen::Users(UsersScreen::new(&users))),
                    Some(i) => {
                        next = Some(Screen::Play(Box::new(Play::new(i, progress.programs[i].clone(), speed, me.shirt))))
                    }
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
                    screen = Screen::Select;
                }
                30 => {
                    shot("2d-select");
                    screen = Screen::Play(Box::new(Play::new(shot_level, None, 2, 4)));
                }
                45 => {
                    shot("3-intro");
                    if let Screen::Play(p) = &mut screen {
                        p.debug_solve();
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
