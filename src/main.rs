mod audio;
mod cache;
mod dialog;
mod emulator;
mod game_db;
mod hash;
mod menu;
mod user_db;

use cache::Cache;
use chrono::Utc;
use dialog::{Dialog, DialogUpdate};
use emulator::*;
use game_db::*;
use menu::*;
use user_db::*;

use macroquad::prelude::*;
use std::{
    collections::{HashMap, VecDeque},
    path::{Path, PathBuf},
};

use crate::dialog::DynamicDialog;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let game_db = GameDb::load().await.unwrap();

    macroquad::Window::new("RetroArcade", async {
        let result = macroquad_main(game_db).await;
        result.unwrap();
    });
}

async fn macroquad_main(game_db: GameDb) -> anyhow::Result<()> {
    let mut app = App {
        state: AppState::Menu,
        menu: MenuState {
            game_db,
            user_db: UserDb::load(Path::new("users.json"), Path::new("saves/"))?,
            textures: HashMap::new(),
            cache: Cache::new(Path::new("image_cache"))?,

            selected_game: 0,
            max_horizontal_games: 4,
            current_user: "sinono3".to_string(),
        },
        emulator: None,

        dialog_queue: VecDeque::new(),
        current_dialog: None,
    };

    loop {
        let event = app.update();

        match event {
            AppEvent::Continue => (),
            AppEvent::GoToMenu => {
                if let Some(emulator) = app.emulator {
                    let save_buffer = emulator.snapshot();
                    let username = &app.menu.current_user;
                    let game = app
                        .menu
                        .game_db
                        .games
                        .values()
                        .nth(app.menu.selected_game)
                        .unwrap()
                        .id;

                    app.menu
                        .user_db
                        .save(&save_buffer, username, game, Utc::now())?;
                }

                app.state = AppState::Menu;
                app.emulator = None;
            }
            AppEvent::StartEmulator { core, rom, save } => {
                app.state = AppState::Emulator;
                app.emulator = Some(EmulatorState::create(&core, &rom, save));
            }
            AppEvent::SpawnDialog(dialog) => {
                app.dialog_queue.push_back(dialog);
            }
        }

        app.render();

        next_frame().await;
    }
}

pub struct App {
    pub state: AppState,
    pub menu: MenuState,
    pub emulator: Option<EmulatorState>,

    pub dialog_queue: VecDeque<DynamicDialog>,
    pub current_dialog: Option<DynamicDialog>,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub enum AppState {
    Menu,
    Emulator,
}

pub enum AppEvent {
    Continue,
    GoToMenu,
    StartEmulator {
        core: PathBuf,
        rom: PathBuf,
        save: Option<Vec<u8>>,
    },
    SpawnDialog(DynamicDialog),
}

impl App {
    pub fn update(&mut self) -> AppEvent {
        // Update dialogs
        if self.current_dialog.is_none() {
            self.current_dialog = self.dialog_queue.pop_front();
        }

        if let Some(dialog) = &mut self.current_dialog {
            let update = match dialog {
                DynamicDialog::YesOrNo(dialog) => dialog.update(),
            };

            match update {
                DialogUpdate::Finish => {
                    let dialog = self.current_dialog.take().unwrap();
                    let event = match dialog {
                        DynamicDialog::YesOrNo(dialog) => dialog.produce_event(),
                    };

                    return event;
                }
                DialogUpdate::Continue => return AppEvent::Continue,
            };
        };

        match self.state {
            AppState::Menu => self.menu.update(),
            AppState::Emulator => {
                if let Some(emulator) = &mut self.emulator {
                    emulator.update()
                } else {
                    AppEvent::GoToMenu
                }
            }
        }
    }

    pub fn render(&mut self) {
        match self.state {
            AppState::Menu => self.menu.render(),
            AppState::Emulator => {
                if let Some(emulator) = self.emulator.as_ref() {
                    emulator.render();
                }
            }
        }

        // Show dialogs
        if let Some(dialog) = self.current_dialog.as_ref() {
            match dialog {
                DynamicDialog::YesOrNo(dialog) => dialog.render(),
            }
        }
    }
}
