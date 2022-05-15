use crate::{
    dialog::{DynamicDialog, YesOrNoDialog},
    game_db::GameDb,
    user_db::{SaveState, UserDb},
    AppEvent,
};
use macroquad::prelude::*;
use std::path::{Path, PathBuf};

pub struct MenuState {
    pub game_db: GameDb,
    pub selected_console: usize,
    pub selected_game: usize,
    pub max_horizontal_games: usize,

    pub user_db: UserDb,
    pub current_user: String,
}

impl MenuState {
    pub fn update(&mut self) -> AppEvent {
        let consoles = self.game_db.consoles();

        if is_key_pressed(KeyCode::N) {
            self.selected_console = self.selected_console.saturating_add(1) % consoles.len();
        }
        if is_key_pressed(KeyCode::P) {
            self.selected_console = self.selected_console.saturating_sub(1) % consoles.len();
        }

        let selected_console_name = &consoles[self.selected_console];

        selected_game_input(
            &mut self.selected_game,
            self.max_horizontal_games,
            self.game_db.games[selected_console_name].len(),
        );

        if is_key_pressed(KeyCode::Enter) {
            let selected_game_filename =
                &self.game_db.games[selected_console_name][self.selected_game].filename;

            let selected_game_name = Path::new(selected_game_filename)
                .file_stem()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string();

            let mut core_path = PathBuf::from("cores/");
            core_path.push(selected_console_name);

            let mut rom_path = PathBuf::from("roms/");
            rom_path.push(selected_console_name);
            rom_path.push(selected_game_filename);

            let mut saves: Vec<&SaveState> = self.user_db.saves[&self.current_user]
                .iter()
                .filter(|s| {
                    s.console.as_str() == selected_console_name.as_str()
                        && s.game.as_str() == selected_game_name.as_str()
                })
                .collect();

            saves.sort_by_key(|s| s.date);

            let latest_save = saves.last().and_then(|s| std::fs::read(&s.path).ok());

            if latest_save.is_some() {
                AppEvent::SpawnDialog(DynamicDialog::YesOrNo(YesOrNoDialog {
                    text: "Do you wish to load a saved state?".to_string(),
                    value: false,
                    event_handler: Box::new(|yes| {
                        if yes {
                            AppEvent::StartEmulator {
                                core: core_path,
                                rom: rom_path,
                                save: latest_save,
                            }
                        } else {
                            AppEvent::StartEmulator {
                                core: core_path,
                                rom: rom_path,
                                save: None,
                            }
                        }
                    }),
                }))
            } else {
                AppEvent::StartEmulator {
                    core: core_path,
                    rom: rom_path,
                    save: None,
                }
            }
        } else {
            AppEvent::Continue
        }
    }

    pub fn render(&self) {
        clear_background(LIGHTGRAY);

        let games = &self.game_db.games[&self.game_db.consoles()[self.selected_console]];
        let game_size = (screen_width() / self.max_horizontal_games as f32) as f32;

        for (i, game) in games.iter().enumerate() {
            let x = (i % self.max_horizontal_games) as f32 * game_size;
            let y = (i / self.max_horizontal_games) as f32 * game_size + TITLE_TEXT_SIZE + MARGIN;

            draw_rectangle(x, y, game_size, game_size, game.color);

            if i == self.selected_game {
                draw_rectangle_lines(x, y, game_size, game_size, 8.0, BLACK);
            }
        }

        const MARGIN: f32 = 10.0;
        const TITLE_TEXT_SIZE: f32 = 30.0;

        // Show console name
        draw_text(
            &self.game_db.consoles()[self.selected_console],
            20.0,
            screen_height() - MARGIN,
            TITLE_TEXT_SIZE,
            DARKGRAY,
        );

        // Show game title
        if let Some(game) = games.get(self.selected_game) {
            draw_text(
                &game.title,
                20.0,
                TITLE_TEXT_SIZE,
                TITLE_TEXT_SIZE,
                DARKGRAY,
            );
        }
    }

    pub fn console_name(&self, console: usize) -> &str {
        &self.game_db.consoles()[console]
    }

    pub fn game_name(&self, console: usize, game: usize) -> &str {
        Path::new(&self.game_db.games[self.console_name(console)][game].filename)
            .file_stem()
            .unwrap()
            .to_str()
            .unwrap()
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
