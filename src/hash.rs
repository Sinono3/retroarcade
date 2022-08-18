use std::{
    fs::File,
    io::{self, Read, Write},
    path::Path,
};

use log::error;
use sha1::{Digest, Sha1};
use thiserror::Error;

pub type Sha1Hash = [u8; 20];

pub fn hash_rom<P>(rom_path: P) -> Result<Sha1Hash, RomHashError>
where
    P: AsRef<Path>,
{
    let mut file = File::open(&rom_path)?;
    let mut hasher = Sha1::new();

    match rom_path.as_ref().extension().and_then(|e| e.to_str()) {
        Some("sfc") => SnesHasher::hash(&mut file, &mut hasher),
        Some("nes") => NesHasher::hash(&mut file, &mut hasher),
        _ => DefaultHasher::hash(&mut file, &mut hasher),
    }?;

    Ok(hasher.finalize().into())
}

pub trait RomHasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError>;
}

#[derive(Error, Debug)]
pub enum RomHashError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),
    #[error("Invalid ROM")]
    Invalid,
    #[error("Unsupported ROM format")]
    Unsupported,
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
        let size = file.metadata()?.len();

        if size % 1024 == 512 {
            file.read_exact(&mut [0; 512])?;
        }

        let _ = io::copy(file, hasher)?;
        Ok(())
    }
}

pub struct NesHasher;

impl RomHasher for NesHasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError> {
        let mut header = [0u8; 16];
        file.read_exact(&mut header)?;

        if &header[..3] != b"NES" {
            return Err(RomHashError::Invalid);
        }

        let has_trainer = header[6] & 4 == 4;

        if has_trainer {
            let mut tmp = [0u8; 512];
            file.read_exact(&mut tmp)?;
        }

        let _ = io::copy(file, hasher)?;
        Ok(())
    }
}

pub fn bytes_to_hex(bytes: &[u8]) -> String {
    let mut hex = String::new();

    for byte in bytes.iter() {
        use std::fmt::Write;
        write!(hex, "{:02X}", byte).unwrap();
    }

    hex
}
