use std::{
    fs::File,
    io::{self, BufReader, Read, Write},
    mem,
};
use thiserror::Error;

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

pub struct N64Hasher;

impl RomHasher for N64Hasher {
    fn hash(file: &mut File, hasher: &mut dyn Write) -> Result<(), RomHashError> {
        // TODO!
        return Err(RomHashError::Invalid);

        let size = file.metadata()?.len();
        let mut header = [0u8; 4];

        file.read_exact(&mut header)?;

        if size < 4 {
            return Err(RomHashError::Invalid);
        }

        enum ByteSwap {
            /// [1, 2, 3, 4] to [2, 1, 4, 3]
            Z,
            /// [1, 2, 3, 4] to [1, 2, 3, 4]
            N,
        }

        let swap_type = match header {
            // Native
            [0x80, 0x37, 0x12, 0x40] => ByteSwap::Z, // r.s = zSwap
            // Byte swapped
            [0x37, 0x80, 0x40, 0x12] => ByteSwap::N, // r.s = nSwap
            // Little-endian
            [0x40, 0x12, 0x37, 0x80] => return Err(RomHashError::Unsupported),
            _ => return Err(RomHashError::Invalid),
        };

        match swap_type {
            ByteSwap::Z => {
                // Pre
                /*let mut r = 4;
                let mut p = [0u8; 1024 * 1024 * 4];
                let mut rb = [0u8; 4];

                let ll = p.len();
                let rl = ll - r;
                let l = rl + 4 - 1 - (rl - 1) % 4;

                io::copy(&mut &rb[..r], &mut &mut p[..])?;

                if rl <= 0 {
                    //r = r - ll;
                    //io::copy(&mut &b[ll..], &mut &mut b[..])?;
                    panic!(); // TODO!
                }

                let n = r;
                let mut b = vec![0u8; l];
                let x = file.read(&mut b)?;

                if x == 0 {
                    panic!("End"); // TODO!
                }

                io::copy(&mut &b[..x], &mut &mut p[n..ll])?;

                n += x;

                if ll <= n {
                    
                }*/

                /*let mut buffer = [0u8; 4];

                while let Ok(()) = file.read_exact(&mut buffer) {
                    hasher.write_all(&[buffer[1], buffer[0]])?;
                    hasher.write_all(&[buffer[3], buffer[2]])?;
                }*/
                /*let mut buffer = Vec::new();
                file.read_to_end(&mut buffer)?;

                println!("{}", bytes_to_hex(&buffer[..4]));

                for chunk in buffer.chunks_exact_mut(4) {
                    println!("R {:?}", chunk);
                    let mut aux;
                    aux = chunk[0];
                    chunk[0] = chunk[1];
                    chunk[1] = aux;
                    aux = chunk[2];
                    chunk[2] = chunk[3];
                    chunk[3] = aux;
                    println!("N {:?}", chunk);
                }

                let mut reader = BufReader::new(&buffer[..]);
                io::copy(&mut reader, hasher)?;*/
            }
            ByteSwap::N => {
                let _ = io::copy(file, hasher)?;
                let mut buffer = [0u8; 4];

                while let Ok(()) = file.read_exact(&mut buffer) {
                    hasher.write_all(&[buffer[1], buffer[0]])?;
                    hasher.write_all(&[buffer[3], buffer[2]])?;
                }
            }
        }

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
