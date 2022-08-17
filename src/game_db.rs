use std::{collections::HashMap, ffi::OsStr, fs, path::PathBuf};

use anyhow::{Context, Result};
use log::error;
use macroquad::{prelude::Color, rand};
use sqlx::SqliteConnection;

use crate::{cache::Cache, hash::*};

const CORE_DIR: &str = "cores/";
const ROMS_DIR: &str = "roms/";

pub struct Game {
    pub id: i64,
    pub rom_path: PathBuf,
    pub sha1: String,
    pub console_id: i64,
    pub metadata: GameMetadata,
}

pub struct GameMetadata {
    pub title: String,
    pub cover_url: String,
    pub color: Color,
}

pub struct Console {
    pub id: i64,
    pub core_path: PathBuf,
    pub name: String,
}

pub struct GameDb {
    pub games: HashMap<i64, Game>,
    pub consoles: HashMap<i64, Console>,
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
    system_name: String,
    system_short_name: String,
}

impl GameDb {
    pub async fn load(cache: &mut Cache) -> Result<Self> {
        let mut games = HashMap::new();
        let mut consoles = HashMap::new();

        let cores_dir = fs::read_dir(CORE_DIR)
            .context("reading core dir")?
            .filter_map(|core| core.ok())
            .filter(|core| core.file_type().map_or(false, |t| t.is_file()))
            .filter_map(|core| {
                let path = core.path();
                path.file_stem()
                    .map(|stem: &OsStr| stem.to_owned())
                    .map(move |name| (path, name))
            });

        // TODO: download openvgdb
        let openvgdb = sqlx::SqlitePool::connect("openvgdb.sqlite").await?;
        let mut conn = openvgdb.acquire().await?;

        for (core_path, core_name) in cores_dir {
            let mut roms_path = PathBuf::from(ROMS_DIR);
            roms_path.push(&core_name);

            let roms_iter = fs::read_dir(roms_path)
                .context("reading roms path")?
                .filter_map(|rom| rom.ok())
                .filter(|rom| rom.file_type().map_or(false, |t| t.is_file()))
                .filter_map(|rom| {
                    let path = rom.path();
                    let name = path.file_name()?.to_owned();
                    Some((path, name))
                });

            for (rom_path, name) in roms_iter {
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
                        console_id: openvgdb_rom.system_id,
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

                // Insert console if not yet in DB
                if !consoles.contains_key(&openvgdb_rom.system_id) {
                    let openvgdb_system =
                        get_system_with_id(&mut conn, openvgdb_rom.system_id).await?;
                    consoles.insert(
                        openvgdb_rom.system_id,
                        Console {
                            id: openvgdb_rom.system_id,
                            core_path: core_path.clone(),
                            name: openvgdb_system.system_short_name,
                        },
                    );
                }
            }
        }

        Ok(GameDb { games, consoles })
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
