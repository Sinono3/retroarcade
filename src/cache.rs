use macroquad::prelude::Image;
use std::path::Path;

use crate::hash::{bytes_to_hex, Sha1Hash, RomHashError};

pub struct Cache {
    hash_cache: sled::Db,
    image_cache: sled::Db,
}

impl Cache {
    pub fn new<P>(hash_cache_path: P, image_cache_path: P) -> Result<Self, sled::Error>
    where
        P: AsRef<Path>,
    {
        Ok(Self {
            hash_cache: sled::open(hash_cache_path)?,
            image_cache: sled::open(image_cache_path)?,
        })
    }

    pub fn get_or_insert_rom_hash<F>(&mut self, path: &str, mut f: F) -> anyhow::Result<String>
    where
        F: FnMut(&str) -> Result<Sha1Hash, RomHashError>,
    {
        if let Some(hash) = self.hash_cache.get(path)? {
            Ok(String::from_utf8(hash.to_vec())?)
        } else {
            let hash = bytes_to_hex(&f(path)?);
            self.hash_cache.insert(path, &hash[..])?;
            Ok(hash)
        }
    }

    pub fn get_or_insert_image<F>(&mut self, url: &str, mut f: F) -> anyhow::Result<Vec<u8>>
    where
        F: FnMut(&str) -> Result<Vec<u8>, anyhow::Error>,
    {
        let bytes = if let Some(bytes) = self.image_cache.get(url)? {
            bytes.to_vec()
        } else {
            let bytes = f(url)?;
            self.image_cache.insert(url, &bytes[..])?;
            bytes
        };

        Ok(bytes)
    }
}
