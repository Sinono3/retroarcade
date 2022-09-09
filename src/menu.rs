use std::{collections::HashMap, io::Write, process::Command};

use gilrs::{Button, Event, Gilrs};
use macroquad::prelude::*;

use crate::{cache::Cache, config::Config, game_db::GameDb, AppEvent};

pub struct MenuState {
    pub game_db: GameDb,
    pub config: Config,
    pub cache: Cache,
    pub textures: HashMap<i64, Texture2D>,
    pub input: MenuInput,

    pub selected_game: usize,
    pub max_tile_size: usize,

    pub glowing_material: Material,
    pub time: f32,
}

impl MenuState {
    pub fn update(&mut self, gilrs: &mut Gilrs) -> AppEvent {
        let previous_game = self.selected_game;
        let game_count = self.game_db.games_iter().count();
        let row_width = screen_width() as usize / self.max_tile_size;

        self.input = get_input(gilrs, &self.input);
        self.selected_game = match self.input.direction {
            InputDirection::Right => self.selected_game.saturating_add(1),
            InputDirection::Left => self.selected_game.saturating_sub(1),
            InputDirection::Down => self.selected_game.saturating_add(row_width),
            InputDirection::Up => self.selected_game.saturating_sub(row_width),
            InputDirection::None => self.selected_game,
        };
        self.selected_game = self.selected_game.max(0).min(game_count.saturating_sub(1));

        // Glow effect reset
        if self.selected_game != previous_game {
            self.time = 0.0;
        }

        // Check for poweroff/reboot commands
        #[cfg(target_os = "linux")]
        poweroff_reboot_check(gilrs, &self.config);

        if self.input.enter {
            let (_id, game) = &self.game_db.games_iter().nth(self.selected_game).unwrap();
            let system = &self.game_db.get_system(game.system_id);

            let rom = game.rom_path.clone();
            let core = system.core_path.clone();

            AppEvent::StartEmulator {
                core,
                rom,
                save: None,
            }
        } else {
            AppEvent::Continue
        }
    }

    pub fn render(&mut self) {
        clear_background(DARKGRAY);

        let row_width = screen_width() as usize / self.max_tile_size;
        let game_size = (screen_width() / row_width as f32) as f32;
        let current_row = self.selected_game / row_width;
        let max_rows = (screen_height() - MARGIN) / game_size;
        // Max rows / 2 because the scrolling needs to happen before
        let scroll = (current_row as usize).saturating_sub(max_rows as usize / 2);

        for (gfx_counter, (counter, (_id, game))) in self
            .game_db
            .games_iter()
            .enumerate()
            .skip(scroll * row_width)
            .enumerate()
        {
            let x = (gfx_counter % row_width) as f32 * game_size;
            let y = (gfx_counter / row_width) as f32 * game_size + TITLE_TEXT_SIZE + MARGIN;

            if counter == self.selected_game {
                self.time += get_frame_time();
                self.glowing_material.set_uniform("time", self.time);
                gl_use_material(self.glowing_material);
            }

            if let Some(metadata) = &game.metadata {
                let cover_url = &metadata.cover_url;

                let texture = self.textures.entry(metadata.release_id).or_insert_with(|| {
                    if let Ok(bytes) = self.cache.get_or_insert_image(cover_url, |url| {
                        Ok(reqwest::blocking::get(url)?.bytes()?.to_vec())
                    }) {
                        let image = image::load_from_memory(&bytes[..]).unwrap();
                        let rgba8 = image.to_rgba8();
                        let bytes: Vec<_> = rgba8.as_raw().as_slice().to_vec();

                        let img = Image {
                            bytes,
                            width: rgba8.width() as u16,
                            height: rgba8.height() as u16,
                        };

                        Texture2D::from_image(&img)
                    } else {
                        Texture2D::from_rgba8(8, 8, &[255u8; 8 * 8])
                    }
                });

                draw_texture_ex(
                    *texture,
                    x,
                    y,
                    Color::new(1.0, 1.0, 1.0, 1.0),
                    DrawTextureParams {
                        dest_size: Some(Vec2::new(game_size, game_size)),
                        source: None,
                        rotation: 0.0,
                        flip_x: false,
                        flip_y: false,
                        pivot: Some(Vec2::new(0.0, 0.0)),
                    },
                );
            } else {
                // If no texture was found, then just draw a colored square
                // with the name of the game.
                draw_rectangle(x, y, game_size, game_size, game.color);
            }

            if counter == self.selected_game {
                gl_use_default_material();
                draw_rectangle_lines(x, y, game_size, game_size, 8.0, BLACK);
            }
        }

        const MARGIN: f32 = 10.0;
        const TITLE_TEXT_SIZE: f32 = 30.0;

        if let Some((_id, game)) = self.game_db.games_iter().nth(self.selected_game) {
            let system = &self.game_db.get_system(game.system_id);

            // Show console name
            draw_rectangle(
                0.0,
                screen_height() - MARGIN - 24.0,
                screen_width(),
                MARGIN + 24.0,
                DARKGRAY,
            );
            draw_text(
                &system.name,
                20.0,
                screen_height() - MARGIN,
                TITLE_TEXT_SIZE,
                LIGHTGRAY,
            );

            let text = if let Some(metadata) = &game.metadata {
                metadata.title.as_str()
            } else {
                game.filename.as_str()
            };
            // Show game title
            draw_text(text, 20.0, TITLE_TEXT_SIZE, TITLE_TEXT_SIZE, LIGHTGRAY);
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
pub struct MenuInput {
    direction: InputDirection,
    enter: bool,
    up: bool,
    down: bool,
    left: bool,
    right: bool,
}

#[derive(Copy, Clone, PartialEq, Eq, Default, Debug)]
enum InputDirection {
    Left,
    Right,
    Up,
    Down,
    #[default]
    None,
}

fn get_input(gilrs: &mut Gilrs, input: &MenuInput) -> MenuInput {
    // Keyboard input
    let mut right = is_key_pressed(KeyCode::Right);
    let mut left = is_key_pressed(KeyCode::Left);
    let mut down = is_key_pressed(KeyCode::Down);
    let mut up = is_key_pressed(KeyCode::Up);
    let mut enter = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space);

    // Gamepad input
    while let Some(Event { .. }) = gilrs.next_event() {}

    for (_g_id, gamepad) in gilrs.gamepads() {
        right = right || gamepad.is_pressed(Button::DPadRight);
        left = left || gamepad.is_pressed(Button::DPadLeft);
        down = down || gamepad.is_pressed(Button::DPadDown);
        up = up || gamepad.is_pressed(Button::DPadUp);
        enter = enter || gamepad.is_pressed(Button::South) || gamepad.is_pressed(Button::East);
    }

    let direction = if !input.right && right {
        InputDirection::Right
    } else if !input.left && left {
        InputDirection::Left
    } else if !input.down && down {
        InputDirection::Down
    } else if !input.up && up {
        InputDirection::Up
    } else {
        InputDirection::None
    };

    MenuInput {
        direction,
        enter,
        up,
        down,
        left,
        right,
    }
}

fn poweroff_reboot_check(gilrs: &Gilrs, config: &Config) {
    // Check for poweroff/reboot gamepad combinations
    let (mut poweroff, mut reboot) =
        gilrs
            .gamepads()
            .fold((false, false), |(poweroff, reboot), (_, g)| {
                let base = g.is_pressed(Button::Select) && g.is_pressed(Button::Start);
                // Start+Select+L1 = Power off
                let poweroff = poweroff || (base && g.is_pressed(Button::LeftTrigger));
                // Start+Select+R1 = Reboot
                let reboot = reboot || (base && g.is_pressed(Button::RightTrigger));
                (poweroff, reboot)
            });

    // Also check for poweroff/reboot key combinations
    // Ctrl+Alt+End = Power off
    poweroff = poweroff
        || (is_key_down(KeyCode::LeftControl)
            && is_key_down(KeyCode::LeftAlt)
            && is_key_down(KeyCode::End));

    // Ctrl+Alt+Del = Reboot
    reboot = reboot
        || (is_key_down(KeyCode::LeftControl)
            && is_key_down(KeyCode::LeftAlt)
            && is_key_down(KeyCode::Delete));

    let exec = |cmd| {
        let output = Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .output()
            .expect("failed to execute reboot process");
        std::io::stdout().write_all(&output.stdout).unwrap();
        std::io::stderr().write_all(&output.stderr).unwrap();
    };

    if poweroff {
        println!("Poweroff requested");
        exec(&config.menu.poweroff_cmd);
    }
    if reboot {
        println!("Reboot requested");
        exec(&config.menu.reboot_cmd);
    }
}
