mod audio;
mod cache;
mod config;
mod dialog;
mod emulator;
mod game_db;
mod hash;
mod menu;
mod saves;

use std::{
    collections::{HashMap, VecDeque},
    path::PathBuf,
};

use chrono::Utc;
use macroquad::prelude::*;

use crate::{
    cache::Cache,
    config::*,
    dialog::{Dialog, DialogUpdate, DynamicDialog},
    emulator::*,
    game_db::*,
    menu::*,
    saves::Saves,
};

#[tokio::main]
async fn main() {
    pretty_env_logger::init();
    let mut cache = Cache::new("cache/hashes", "cache/image").unwrap();
    let game_db = GameDb::load(&mut cache).await.unwrap();

    macroquad::Window::new("RetroArcade", async {
        let result = macroquad_main(cache, game_db).await;
        result.unwrap();
    });
}

async fn macroquad_main(cache: Cache, game_db: GameDb) -> anyhow::Result<()> {
    let config = Config::load("retroarcade.toml")?;
    let current_user = config.default_user.clone();
    let max_horizontal_games = config.max_horizontal_games;

    let glowing_material = load_material(
        include_str!("shaders/glowing_vert.glsl"),
        include_str!("shaders/glowing_frag.glsl"),
        MaterialParams {
            uniforms: vec![
                ("time".to_string(), UniformType::Float1),
                ("glowFrequency".to_string(), UniformType::Float1),
                ("glowIntensity".to_string(), UniformType::Float1),
                ("zoomFactor".to_string(), UniformType::Float1),
            ],
            ..Default::default()
        },
    )?;
    glowing_material.set_uniform("glowFrequency", 1.0f32);
    glowing_material.set_uniform("glowIntensity", 1.0f32);
    glowing_material.set_uniform("zoomFactor", 0.2f32);

    let mut app = App {
        state: AppState::Menu,
        menu: MenuState {
            game_db,
            config,
            saves: Saves::load("saves/")?,
            cache,
            textures: HashMap::new(),

            selected_game: 0,
            max_horizontal_games,
            current_user,

            glowing_material,
            glowing_material_time: 0.0,
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
                        .saves
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
