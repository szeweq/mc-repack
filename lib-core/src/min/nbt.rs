use std::{error::Error, io::{copy, Cursor, Write}};

use crate::cfg::{AcceptsConfig, ConfigHolder};

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

pub(super) enum MinifierNBT {}
impl AcceptsConfig for MinifierNBT {
    type Cfg = NBTConfig;
}
impl ConfigHolder<MinifierNBT> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let Some(compression) = NBTCompression::detect(b) else {
            return Err(NBTError.into());
        };
    
        #[cfg(feature = "nbt-zopfli")]
        if self.use_zopfli {
            return minify_with_zopfli(b, vout, compression);
        }

        minify_with_gzip(b, vout, compression)
    }
}

fn minify_with_gzip(b: &[u8], vout: &mut Vec<u8>, nc: NBTCompression) -> Result_ {
    let mut enc = flate2::write::GzEncoder::new(vout, flate2::Compression::best());
    copy_to_encoder(&mut enc, b, nc)?;
    enc.finish()?;
    Ok(())
}

#[cfg(feature = "nbt-zopfli")]
fn minify_with_zopfli(b: &[u8], vout: &mut Vec<u8>, nc: NBTCompression) -> Result_ {
    use std::num::NonZeroU64;
    let zo = zopfli::Options {
        iteration_count: NonZeroU64::new(10).unwrap(),
        iterations_without_improvement: NonZeroU64::new(2).unwrap(),
        ..<zopfli::Options as Default>::default()
    };
    let mut enc = zopfli::GzipEncoder::new(zo, zopfli::BlockType::Dynamic, vout)?;
    copy_to_encoder(&mut enc, b, nc)?;
    enc.finish()?;
    Ok(())
}

fn copy_to_encoder<W: Write>(w: &mut W, b: &[u8], nc: NBTCompression) -> Result_ {
    match nc {
        NBTCompression::None => {
            let mut cur = Cursor::new(b);
            copy(&mut cur, w)
        }
        NBTCompression::GZip => {
            let mut gzr = flate2::bufread::GzDecoder::new(b);
            copy(&mut gzr, w)
        }
        NBTCompression::ZLib => {
            let mut zlr = flate2::bufread::ZlibDecoder::new(b);
            copy(&mut zlr, w)
        }
    }?;
    Ok(())
}

#[derive(Default)]
pub struct NBTConfig {
    #[cfg(feature = "nbt-zopfli")]
    pub use_zopfli: bool
}

#[derive(Debug)]
pub struct NBTError;
impl Error for NBTError {}
impl std::fmt::Display for NBTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NBT entry")
    }
}