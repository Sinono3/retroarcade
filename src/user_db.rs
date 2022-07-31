use std::fs::{self, File};
use std::io::Write;
use std::path::Path;
use std::{collections::HashMap, path::PathBuf};

use anyhow::{Context, Result};
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct UserDb {
    pub users: Vec<User>,
    pub saves: HashMap<String, Vec<SaveState>>,
    pub users_path: PathBuf,
    pub saves_path: PathBuf,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub password: String,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct SaveState {
    pub path: PathBuf,
    pub game: i64,
    pub date: chrono::DateTime<Utc>,
}

impl UserDb {
    pub fn load<P>(users_path: P, saves_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let users_file = File::open(&users_path).context("opening users database")?;
        let users: Vec<User> =
            serde_json::from_reader(users_file).context("parsing users database")?;

        let mut saves = HashMap::new();

        for save in fs::read_dir(&saves_path)? {
            let save = if let Ok(save) = save { save } else { continue };
            let save_path = save.path();
            let save_name = save_path.file_stem().unwrap().to_str().unwrap().to_owned();
            let parts: Vec<&str> = save_name.splitn(3, "_").collect();

            if let [username, game, date_time] = parts[0..3] {
                let username_exists = users
                    .iter()
                    .filter(|u| u.username == username)
                    .next()
                    .is_some();

                if !username_exists {
                    continue;
                }

                let save_vec = saves
                    .entry(username.to_string())
                    .or_insert_with(|| Vec::new());

                let parsed_date = NaiveDateTime::parse_from_str(date_time, "%Y%m%d_%H%M%S");

                let date = if let Ok(date) = parsed_date {
                    DateTime::from_utc(date, Utc)
                } else {
                    continue;
                };

                save_vec.push(SaveState {
                    path: save_path,
                    game: game.parse()?,
                    date,
                });
            } else {
                // Invalid save file
                continue;
            }
        }

        Ok(Self {
            users,
            saves,
            users_path: users_path.as_ref().to_path_buf(),
            saves_path: saves_path.as_ref().to_path_buf(),
        })
    }

    pub fn save(
        &mut self,
        data: &[u8],
        username: &str,
        game: i64,
        date: DateTime<Utc>,
    ) -> Result<()> {
        let mut path = self.saves_path.clone();

        path.push(format!(
            "{}_{}_{}_{}.sav",
            username,
            game,
            date.format("%Y%m%d"),
            date.format("%H%M%S")
        ));

        // Write to file
        let mut file = File::create(&path)?;
        file.write_all(data)?;

        // Once written, store save metadata in memory
        let save_state = SaveState { path, game, date };

        self.saves
            .entry(username.to_string())
            .or_insert_with(|| Vec::new())
            .push(save_state);

        Ok(())
    }
}
