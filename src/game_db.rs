use std::{
    collections::HashMap,
    ffi::{CStr, OsStr},
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use log::error;
use macroquad::{prelude::Color, rand};
use retro_rs::Emulator;
use sqlx::SqliteConnection;

use crate::{cache::Cache, hash::*};

const CORE_DIR: &str = "cores/";
const ROMS_DIR: &str = "roms/";

pub struct Game {
    pub id: i64,
    pub system_id: i64,
    pub rom_path: PathBuf,
    pub sha1: String,
    pub metadata: GameMetadata,
}

pub struct GameMetadata {
    pub title: String,
    pub cover_url: String,
    pub color: Color,
}

pub struct System {
    pub id: i64,
    pub core_path: PathBuf,
    pub name: String,
    pub extensions: Vec<String>,
}

pub struct GameDb {
    pub games: HashMap<i64, Game>,
    pub systems: HashMap<i64, System>,
}

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct OpenVgdbRom {
    rom_id: i64,
    rom_file_name: String,
    rom_extensionless_file_name: String,
    system_id: i64,
}

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct OpenVgdbRelease {
    release_title_name: String,
    release_cover_front: String,
    release_date: String,
    release_reference_url: String,
    release_reference_image_url: String,
}

#[derive(Clone, PartialEq, Eq, sqlx::FromRow)]
#[sqlx(rename_all = "camelCase")]
struct OpenVgdbSystem {
    system_id: i64,
    system_name: String,
    system_short_name: String,
}

impl GameDb {
    pub async fn load(cache: &mut Cache) -> Result<Self> {
        let mut games = HashMap::new();
        let mut systems = HashMap::new();

        // TODO: download openvgdb
        let openvgdb = sqlx::SqlitePool::connect("openvgdb.sqlite").await?;
        let mut conn = openvgdb.acquire().await?;

        let cores_dir = fs::read_dir(CORE_DIR)
            .context("reading core dir")?
            .filter_map(|core| core.ok())
            .filter(|core| core.file_type().map_or(false, |t| t.is_file()))
            .map(|core| core.path());

        'cores: for core_path in cores_dir {
            let extensions: Vec<String> = {
                let extensions = Emulator::create_for_extensions(&core_path);
                let string = extensions.to_str().unwrap().to_string();
                string.split('|').map(String::from).collect()
            };

            let mut extensions_iter = extensions.iter();

            let short_name = loop {
                if let Some(ext) = extensions_iter.next() {
                    match ext.as_str() {
                        "sfc" => break "SNES",
                        "nes" => break "NES",
                        "z64" => break "N64",
                        "cue" => break "PSX",
                        "nds" => break "NDS",
                        _ => continue,
                    }
                } else {
                    log::info!(
                        "Couldn't find system for extensions: {:?}",
                        extensions
                    );

                    // Insert default system
                    systems.insert(
                        -1,
                        System {
                            id: -1,
                            core_path: core_path.clone(),
                            extensions,
                            name: "Generic".to_string(),
                        },
                    );
                    continue 'cores;
                }
            };

            // Insert system if not yet in DB
            if !systems.iter().any(|(_, s)| s.name == short_name) {
                let openvgdb_system = get_system_with_short_name(&mut conn, short_name).await?;
                log::info!(
                    "Inserted system '{}' for extensions: {:?}",
                    openvgdb_system.system_short_name,
                    extensions
                );

                systems.insert(
                    openvgdb_system.system_id,
                    System {
                        id: openvgdb_system.system_id,
                        core_path: core_path.clone(),
                        extensions,
                        name: openvgdb_system.system_short_name,
                    },
                );
            }
        }

        for (rom_path, name) in walkdir::WalkDir::new(ROMS_DIR)
            .into_iter()
            .filter_map(|rom| rom.ok())
            .filter(|rom| rom.file_type().is_file())
            .filter_map(|rom| {
                let path = rom.path().to_path_buf();
                let name = path.file_name()?.to_owned();
                Some((path, name))
            })
        {
            let sha1 = match cache
                .get_or_insert_rom_hash(rom_path.to_str().unwrap(), |_| hash_rom(&rom_path))
            {
                Ok(sha1) => sha1,
                Err(e) => {
                    error!("ROM Hash error: {}", e);
                    continue;
                }
            };

            let openvgdb_rom = if let Ok(rom) = get_rom_with_sha1(&mut conn, &sha1).await {
                log::info!("ROM Found '{}': {}", name.to_str().unwrap(), sha1);
                rom
            } else {
                log::error!("ROM Failed '{}': {}", name.to_str().unwrap(), sha1);
                continue;
            };

            let openvgdb_release = if let Ok(release) =
                get_release_with_rom_id(&mut conn, openvgdb_rom.rom_id).await
            {
                release
            } else {
                continue;
            };

            games.insert(
                openvgdb_rom.rom_id,
                Game {
                    id: openvgdb_rom.rom_id,
                    system_id: openvgdb_rom.system_id,
                    rom_path,
                    sha1,

                    metadata: GameMetadata {
                        title: openvgdb_release.release_title_name,
                        cover_url: openvgdb_release.release_cover_front,
                        color: Color::from_rgba(
                            rand::gen_range(0u8, 255u8),
                            rand::gen_range(0u8, 255u8),
                            rand::gen_range(0u8, 255u8),
                            255,
                        ),
                    },
                },
            );
        }

        Ok(GameDb { games, systems })
    }
}

async fn get_rom_with_sha1(
    conn: &mut SqliteConnection,
    sha1_hex: &str,
) -> Result<OpenVgdbRom, sqlx::Error> {
    sqlx::query_as!(
        OpenVgdbRom,
        r#"
                    SELECT 
                        romID as "rom_id!: _", 
                        romFileName as "rom_file_name!: _", 
                        romExtensionlessFileName as "rom_extensionless_file_name!: _" ,
                        systemID as "system_id!: _"
                    FROM ROMs 
                    WHERE romHashSHA1 = $1
                    "#,
        sha1_hex,
    )
    .fetch_one(conn)
    .await
}

async fn get_release_with_rom_id(
    conn: &mut SqliteConnection,
    rom_id: i64,
) -> Result<OpenVgdbRelease, sqlx::Error> {
    sqlx::query_as!(
        OpenVgdbRelease,
        r#"
        SELECT 
            releaseTitleName as "release_title_name!: _",
            releaseCoverFront as "release_cover_front!: _",
            releaseDate as "release_date!: _",
            releaseReferenceURL as "release_reference_url!: _",
            releaseReferenceImageURL as "release_reference_image_url!: _"
        FROM RELEASES 
        WHERE romID = $1
        ORDER BY releaseDate
        "#,
        rom_id,
    )
    .fetch_one(conn)
    .await
}

async fn get_system_with_id(
    conn: &mut SqliteConnection,
    id: i64,
) -> Result<OpenVgdbSystem, sqlx::Error> {
    sqlx::query_as!(
        OpenVgdbSystem,
        r#"
        SELECT 
            systemID as "system_id!: _",
            systemName as "system_name!: _", 
            systemShortName as "system_short_name!: _"
        FROM SYSTEMS 
        WHERE systemID = $1
        "#,
        id,
    )
    .fetch_one(conn)
    .await
}

async fn get_system_with_short_name(
    conn: &mut SqliteConnection,
    short_name: &str,
) -> Result<OpenVgdbSystem, sqlx::Error> {
    sqlx::query_as!(
        OpenVgdbSystem,
        r#"
        SELECT 
            systemID as "system_id!: _",
            systemName as "system_name!: _", 
            systemShortName as "system_short_name!: _"
        FROM SYSTEMS 
        WHERE systemShortName = $1
        "#,
        short_name,
    )
    .fetch_one(conn)
    .await
}
