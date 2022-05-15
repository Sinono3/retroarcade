use std::{
    fs::File,
    io::{self, Read, Write},
};
use thiserror::Error;

pub trait RomHasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError>;
}

#[derive(Error, Debug)]
pub enum RomHashError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
}

pub struct DefaultHasher;

impl RomHasher for DefaultHasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError> {
        let _ = io::copy(file, hasher)?;
        Ok(())
    }
}

pub struct SnesHasher;

impl RomHasher for SnesHasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError> {
        let metadata = file.metadata()?;

        if metadata.len() % 1024 == 512 {
            file.read_exact(&mut [0; 512])?;
        }

        let _ = io::copy(file, hasher)?;
        Ok(())
    }
}
