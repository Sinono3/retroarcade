use anyhow::{Context, Result};
use macroquad::{
    prelude::{Color, Image},
    rand,
};
use sha1::{Digest, Sha1};
use std::{
    collections::HashMap,
    ffi::{OsStr, OsString},
    fs::{self, File},
    path::PathBuf,
};
use crate::hash::{SnesHasher, DefaultHasher, RomHasher};

const CORE_DIR: &str = "cores/";
const ROMS_DIR: &str = "roms/";

pub struct Game {
    pub title: String,
    pub filename: OsString,
    pub color: Color,
    pub sha1: [u8; 20],
    pub image: Option<Image>,
}

pub struct GameDb {
    /// console -> list of games
    pub games: HashMap<String, Vec<Game>>,
    consoles: Vec<String>,
}

impl GameDb {
    pub async fn load() -> Result<Self> {
        let mut games = HashMap::new();
        let cores_dir = fs::read_dir(CORE_DIR)
            .context("reading core dir")?
            .filter_map(|core| core.ok())
            .filter(|core| core.file_type().map_or(false, |t| t.is_file()))
            .filter_map(|core| {
                let path = core.path();
                path.file_stem().map(|stem: &OsStr| stem.to_owned())
            });

        let openvgdb = sqlx::SqlitePool::connect("openvgdb.sqlite").await?;
        openvgdb.acquire();

        for core in cores_dir {
            let mut roms_path = PathBuf::from(ROMS_DIR);
            roms_path.push(&core);

            let roms =
                fs::read_dir(roms_path)
                    .context("reading roms path")
                    .map_or(vec![], |roms| {
                        roms.filter_map(|rom| rom.ok())
                            .filter(|rom| rom.file_type().map_or(false, |t| t.is_file()))
                            .filter_map(|rom| {
                                let path = rom.path();
                                let name = path.file_name()?.to_owned();
                                Some((path, name))
                            })
                            .filter_map(|(path, name)| {
                                let mut file = File::open(&path).ok()?;
                                let mut hasher = Sha1::new();

                                match path.extension().and_then(|e| e.to_str()) {
                                    Some("sfc") => SnesHasher::hash(&mut file, &mut hasher).ok()?,
                                    _ => DefaultHasher::hash(&mut file, &mut hasher).ok()?,
                                }

                                let sha1: [u8; 20] = hasher.finalize().into();

                                print!("{}: ", name.to_str().unwrap());

                                for byte in sha1.iter() {
                                    print!("{:02X}", byte);
                                }

                                println!();

                                Some(Game {
                                    title: name.to_string_lossy().to_string(),
                                    filename: name.to_os_string(),
                                    color: Color::from_rgba(
                                        rand::gen_range(0u8, 255u8),
                                        rand::gen_range(0u8, 255u8),
                                        rand::gen_range(0u8, 255u8),
                                        255,
                                    ),
                                    sha1,
                                    image: None,
                                })
                            })
                            .collect()
                    });

            games.insert(core.to_string_lossy().to_string(), roms);
        }

        let consoles = games.keys().cloned().collect();

        Ok(GameDb { games, consoles })
    }

    pub fn consoles(&self) -> &[String] {
        &self.consoles
    }
}
