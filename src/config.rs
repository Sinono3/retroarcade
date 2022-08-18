use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Config {
    pub users: Vec<User>,
    pub default_user: String,
    pub max_horizontal_games: usize,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct User {
    pub username: String,
    pub password: String,
}

impl Config {
    pub fn load<P>(config_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config_str = fs::read_to_string(&config_path).context("opening config file")?;
        let config: Self = toml::from_str(&config_str).context("parsing config file")?;
        Ok(config)
    }
}
