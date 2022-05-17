use macroquad::prelude::Image;
use std::path::Path;

pub struct Cache {
    image_cache: sled::Db,
}

impl Cache {
    pub fn new(image_cache_path: &Path) -> Result<Self, sled::Error> {
        Ok(Self {
            image_cache: sled::open(image_cache_path)?,
        })
    }

    pub fn get_image(&mut self, url: &str) -> anyhow::Result<Image> {
        let bytes = if let Some(image) = self.image_cache.get(url)? {
            image.to_vec()
        } else { 
            let image_data = reqwest::blocking::get(url)?.bytes()?;
            self.image_cache.insert(url, image_data.to_vec())?;
            image_data.to_vec()
        };

        let image = image::load_from_memory(&bytes[..])?;
        let rgba8 = image.to_rgba8();
        let bytes: Vec<_> = rgba8.as_raw().as_slice().to_vec();

        Ok(Image {
            bytes,
            width: rgba8.width() as u16,
            height: rgba8.height() as u16,
        })
    }
}
