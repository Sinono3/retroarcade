use std::{fs, path::{Path, PathBuf}, collections::HashMap};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Config {
    pub max_horizontal_games: usize,
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
