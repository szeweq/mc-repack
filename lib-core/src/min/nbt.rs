#![cfg(feature = "nbt")]
use std::{error::Error, io::{self, copy, Write}, num::NonZeroU64};

use crate::cfg::{acfg, CfgZopfli, ConfigHolder};

use super::Result_;

#[repr(u8)]
#[derive(Debug, Clone, Copy)]
enum NBTFormat {
    Raw,
    GZip,
    ZLib
}
impl NBTFormat {
    #[inline]
    const fn detect(b: &[u8]) -> Option<Self> {
        match b[0] {
            0..=12 => Some(Self::Raw),
            31 => Some(Self::GZip),
            120 => Some(Self::ZLib),
            _ => None
        }
    }
    #[inline]
    fn reader(self, b: &[u8]) -> Box<dyn std::io::Read + '_> {
        match self {
            Self::Raw => Box::new(b),
            Self::GZip => Box::new(flate2::bufread::GzDecoder::new(b)),
            Self::ZLib => Box::new(flate2::bufread::ZlibDecoder::new(b)),
        }
    }
    #[inline]
    fn write_to<W: Write>(self, w: &mut W, b: &[u8]) -> io::Result<()> {
        copy(&mut self.reader(b), w).map(|_| ())
    }
}

acfg!(
    /// A NBT minifier that accepts [`NBTConfig`].
    MinifierNBT: NBTConfig
);

impl ConfigHolder<MinifierNBT> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let Some(nbtf) = NBTFormat::detect(b) else {
            return Err(NBTError.into());
        };
    
        #[cfg(feature = "nbt-zopfli")]
        match self.use_zopfli.iter_count() {
            None => {}
            Some(ic) => return minify_with_zopfli(b, vout, nbtf, ic.into())
        }

        let mut enc = flate2::write::GzEncoder::new(vout, flate2::Compression::best());
        nbtf.write_to(&mut enc, b)?;
        enc.finish()?;
        Ok(())
    }
}

#[cfg(feature = "nbt-zopfli")]
fn minify_with_zopfli(b: &[u8], vout: &mut Vec<u8>, nbtf: NBTFormat, ic: NonZeroU64) -> Result_ {
    let zo = zopfli::Options {
        iteration_count: ic,
        iterations_without_improvement: NonZeroU64::new(6).unwrap(),
        ..<zopfli::Options as Default>::default()
    };
    let mut enc = zopfli::GzipEncoder::new(zo, zopfli::BlockType::Dynamic, vout)?;
    nbtf.write_to(&mut enc, b)?;
    enc.finish()?;
    Ok(())
}

/// Configuration for the NBT minifier
#[derive(Default)]
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct NBTConfig {
    #[cfg(feature = "nbt-zopfli")]
    /// Enables Zopfli compression (better, but slower)
    pub use_zopfli: CfgZopfli
}

/// An error that occurs when a minifier cannot detect the compression type of a NBT entry
#[derive(Debug)]
pub struct NBTError;
impl Error for NBTError {}
impl std::fmt::Display for NBTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NBT entry")
    }
}