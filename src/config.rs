use std::{fs, path::Path};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Config {
    pub max_tile_size: usize,
    pub system: Vec<PreconfSystem>,
}

/// Preconfigured/hardcoded systems
/// This works for cores that are not detected by OpenVGDB.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PreconfSystem {
    pub id: i64,
    pub name: String,
    pub extensions: Vec<String>,
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
