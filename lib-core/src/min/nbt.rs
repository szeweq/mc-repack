use std::{error::Error, io::Write};

use super::Result_;

#[derive(Debug)]
enum NBTCompression {
    None,
    GZip,
    ZLib
}
impl NBTCompression {
    #[inline]
    const fn detect(b: &[u8]) -> Option<Self> {
        match b[0] {
            0..=12 => Some(Self::None),
            31 => Some(Self::GZip),
            120 => Some(Self::ZLib),
            _ => None
        }
    }
}

pub(super) fn minify_nbt(b: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let Some(compression) = NBTCompression::detect(b) else {
        return Err(NBTError.into());
    };
    let mut gzw = flate2::write::GzEncoder::new(vout, flate2::Compression::best());
    match compression {
        NBTCompression::None => {
            gzw.write_all(b)?;
        }
        NBTCompression::GZip => {
            let mut gzr = flate2::bufread::GzDecoder::new(b);
            std::io::copy(&mut gzr, &mut gzw)?;
        }
        NBTCompression::ZLib => {
            let mut zlr = flate2::bufread::ZlibDecoder::new(b);
            std::io::copy(&mut zlr, &mut gzw)?;
        }
    }
    Ok(())
}

#[derive(Debug)]
pub struct NBTError;
impl Error for NBTError {}
impl std::fmt::Display for NBTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NBT entry")
    }
}