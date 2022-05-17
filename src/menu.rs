use std::collections::HashMap;

use crate::{
    cache::Cache,
    dialog::{DynamicDialog, YesOrNoDialog},
    game_db::GameDb,
    user_db::{SaveState, UserDb},
    AppEvent,
};
use macroquad::prelude::*;

pub struct MenuState {
    pub game_db: GameDb,
    pub user_db: UserDb,
    pub textures: HashMap<i64, Texture2D>,
    pub cache: Cache,

    pub selected_game: usize,
    pub max_horizontal_games: usize,
    pub current_user: String,
}

impl MenuState {
    pub fn update(&mut self) -> AppEvent {
        selected_game_input(
            &mut self.selected_game,
            self.max_horizontal_games,
            self.game_db.games.len(),
        );

        if is_key_pressed(KeyCode::Enter) {
            let game = &self.game_db.games.values().nth(self.selected_game).unwrap();
            let console = &self.game_db.consoles[&game.console_id];

            let mut saves: Vec<&SaveState> = self.user_db.saves[&self.current_user]
                .iter()
                .filter(|s| s.game == game.id)
                .collect();

            saves.sort_by_key(|s| s.date);

            let rom = game.rom_path.clone();
            let core = console.core_path.clone();

            let latest_save = saves.last().and_then(|s| std::fs::read(&s.path).ok());

            if latest_save.is_some() {
                AppEvent::SpawnDialog(DynamicDialog::YesOrNo(YesOrNoDialog {
                    text: "Do you wish to load a saved state?".to_string(),
                    value: false,
                    event_handler: Box::new(|yes| {
                        if yes {
                            AppEvent::StartEmulator {
                                core,
                                rom,
                                save: latest_save,
                            }
                        } else {
                            AppEvent::StartEmulator {
                                core,
                                rom,
                                save: None,
                            }
                        }
                    }),
                }))
            } else {
                AppEvent::StartEmulator {
                    core,
                    rom,
                    save: None,
                }
            }
        } else {
            AppEvent::Continue
        }
    }

    pub fn render(&mut self) {
        clear_background(LIGHTGRAY);

        let games = &self.game_db.games.values();
        let game_size = (screen_width() / self.max_horizontal_games as f32) as f32;

        for (i, game) in games.clone().enumerate() {
            let x = (i % self.max_horizontal_games) as f32 * game_size;
            let y = (i / self.max_horizontal_games) as f32 * game_size + TITLE_TEXT_SIZE + MARGIN;
            let cover_url = &game.metadata.cover_url;

            let texture = self
                .textures
                .entry(game.id)
                .or_insert_with(|| {
                    if let Ok(img) = self.cache.get_image(cover_url) {
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

            if i == self.selected_game {
                draw_rectangle_lines(x, y, game_size, game_size, 8.0, BLACK);
            }
        }

        const MARGIN: f32 = 10.0;
        const TITLE_TEXT_SIZE: f32 = 30.0;

        if let Some(game) = games.clone().nth(self.selected_game) {
            let console = &self.game_db.consoles[&game.console_id];

            // Show console name
            draw_text(
                &console.name,
                20.0,
                screen_height() - MARGIN,
                TITLE_TEXT_SIZE,
                DARKGRAY,
            );

            // Show game title
            draw_text(
                &game.metadata.title,
                20.0,
                TITLE_TEXT_SIZE,
                TITLE_TEXT_SIZE,
                DARKGRAY,
            );
        }
    }
}

fn selected_game_input(selected_game: &mut usize, max_horizontal_games: usize, game_count: usize) {
    if is_key_pressed(KeyCode::Right) {
        *selected_game = selected_game.saturating_add(1);
    }
    if is_key_pressed(KeyCode::Left) {
        *selected_game = selected_game.saturating_sub(1);
    }
    if is_key_pressed(KeyCode::Down) {
        *selected_game = selected_game.saturating_add(max_horizontal_games);
    }
    if is_key_pressed(KeyCode::Up) {
        *selected_game = selected_game.saturating_sub(max_horizontal_games);
    }

    *selected_game = (*selected_game).max(0).min(game_count.saturating_sub(1));
}
