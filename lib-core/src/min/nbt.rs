#![cfg(feature = "nbt")]
use std::{error::Error, io::{self, copy, Write}};

use crate::cfg::{self, acfg, ConfigHolder};

use super::Result_;

enum NBTReader<'a> {
    Raw(&'a [u8]),
    GZip(flate2::bufread::GzDecoder<&'a [u8]>),
    ZLib(flate2::bufread::ZlibDecoder<&'a [u8]>)
}
impl <'a> NBTReader<'a> {
    #[inline]
    fn from_bytes(b: &'a [u8]) -> Option<Self> {
        Some(match b[0] {
            0..=12 => Self::Raw(b),
            31 => Self::GZip(flate2::bufread::GzDecoder::new(b)),
            120 => Self::ZLib(flate2::bufread::ZlibDecoder::new(b)),
            _ => { return None; }
        })
    }
    #[inline]
    fn reader(&mut self) -> &mut dyn io::Read {
        match self {
            Self::Raw(b) => b,
            Self::GZip(g) => g,
            Self::ZLib(z) => z
        }
    }
    #[inline]
    fn write_to<W: Write>(&mut self, w: &mut W) -> io::Result<u64> {
        copy(self.reader(), w)
    }
}

acfg!(
    /// A NBT minifier that accepts [`NBTConfig`].
    MinifierNBT: NBTConfig
);

impl ConfigHolder<MinifierNBT> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let Some(mut nbtr) = NBTReader::from_bytes(b) else {
            return Err(NBTError.into());
        };
    
        #[cfg(feature = "nbt-zopfli")]
        if let Some(ic) = self.use_zopfli.iter_count() {
            return minify_with_zopfli(vout, &mut nbtr, ic.into())
        }

        let mut enc = flate2::write::GzEncoder::new(vout, flate2::Compression::best());
        nbtr.write_to(&mut enc)?;
        enc.finish()?;
        Ok(())
    }
}

#[cfg(feature = "nbt-zopfli")]
fn minify_with_zopfli(vout: &mut Vec<u8>, nbtr: &mut NBTReader, ic: std::num::NonZeroU64) -> Result_ {
    let zo = zopfli::Options {
        iteration_count: ic,
        iterations_without_improvement: std::num::NonZeroU64::new(6).unwrap(),
        ..<zopfli::Options as Default>::default()
    };
    let mut enc = zopfli::GzipEncoder::new(zo, zopfli::BlockType::Dynamic, vout)?;
    nbtr.write_to(&mut enc)?;
    enc.finish()?;
    Ok(())
}

/// Configuration for the NBT minifier
#[derive(Default)]
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct NBTConfig {
    #[cfg(feature = "nbt-zopfli")]
    /// Enables Zopfli compression (better, but slower)
    pub use_zopfli: cfg::CfgZopfli
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