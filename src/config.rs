use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct Config {
    pub rom_path: PathBuf,
    pub core_path: PathBuf,
    pub cache_path: PathBuf,
    pub system: Vec<PreconfSystem>,
    pub menu: MenuConfig,
}

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct MenuConfig {
    pub max_tile_size: usize,
}

/// Preconfigured/hardcoded systems
/// This works for cores that are not detected by OpenVGDB.
#[derive(Clone, PartialEq, Eq, Serialize, Deserialize, Debug)]
pub struct PreconfSystem {
    #[serde(skip)]
    pub id: i64,
    pub name: String,
    pub lib: String,
    pub ext: Vec<String>,
}

impl Config {
    pub fn load<P>(config_path: P) -> Result<Self>
    where
        P: AsRef<Path>,
    {
        let config_str = fs::read_to_string(&config_path).context("opening config file")?;
        let mut config: Self = toml::from_str(&config_str).context("parsing config file")?;

        for (i, sys) in config.system.iter_mut().enumerate() {
            sys.id = -(i as i64);
            println!("{}", sys.id);
        }

        Ok(config)
    }
}
