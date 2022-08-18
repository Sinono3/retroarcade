use std::{
    collections::HashMap,
    fs::{self, File},
    io::Write,
    path::{Path, PathBuf},
};

use anyhow::Result;
use chrono::{DateTime, NaiveDateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Saves {
    pub saves: HashMap<String, Vec<SaveState>>,
    pub path: PathBuf,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct SaveState {
    pub path: PathBuf,
    pub game: i64,
    pub date: chrono::DateTime<Utc>,
}

impl Saves {
    pub fn load<P>(path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let mut saves = HashMap::new();

        for save in fs::read_dir(&path)? {
            let save = if let Ok(save) = save { save } else { continue };
            let save_path = save.path();
            let save_name = save_path.file_stem().unwrap().to_str().unwrap().to_owned();
            let parts: Vec<&str> = save_name.splitn(3, "_").collect();

            if let [username, game, date_time] = parts[0..3] {
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

        Ok(Saves {
            saves,
            path: path.as_ref().to_path_buf(),
        })
    }

    pub fn save(
        &mut self,
        data: &[u8],
        username: &str,
        game: i64,
        date: DateTime<Utc>,
    ) -> Result<()> {
        let mut path = self.path.clone();

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
