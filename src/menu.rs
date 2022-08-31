use std::collections::HashMap;

use macroquad::prelude::*;

use crate::{cache::Cache, config::Config, game_db::GameDb, AppEvent};

pub struct MenuState {
    pub game_db: GameDb,
    pub config: Config,
    pub cache: Cache,
    pub textures: HashMap<i64, Texture2D>,

    pub selected_game: usize,
    pub max_tile_size: usize,

    pub glowing_material: Material,
    pub glowing_material_time: f32,
}

impl MenuState {
    pub fn update(&mut self) -> AppEvent {
        let row_width = screen_width() as usize / self.max_tile_size;
        let previous_game = self.selected_game;

        selected_game_input(
            &mut self.selected_game,
            row_width,
            self.game_db.games_iter().count(),
        );

        if self.selected_game != previous_game {
            self.glowing_material_time = 0.0;
        }

        if is_key_pressed(KeyCode::Enter) {
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
                self.glowing_material_time += get_frame_time();
                self.glowing_material
                    .set_uniform("time", self.glowing_material_time);
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

fn selected_game_input(selected_game: &mut usize, row_width: usize, game_count: usize) {
    if is_key_pressed(KeyCode::Right) {
        *selected_game = selected_game.saturating_add(1);
    }
    if is_key_pressed(KeyCode::Left) {
        *selected_game = selected_game.saturating_sub(1);
    }
    if is_key_pressed(KeyCode::Down) {
        *selected_game = selected_game.saturating_add(row_width);
    }
    if is_key_pressed(KeyCode::Up) {
        *selected_game = selected_game.saturating_sub(row_width);
    }

    *selected_game = (*selected_game).max(0).min(game_count.saturating_sub(1));
}
