use std::{error::Error, io::{copy, Cursor, Write}};

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

    let mut enc = get_encoder(vout);
    match compression {
        NBTCompression::None => {
            let mut cur = Cursor::new(b);
            copy(&mut cur, &mut enc)
        }
        NBTCompression::GZip => {
            let mut gzr = flate2::bufread::GzDecoder::new(b);
            copy(&mut gzr, &mut enc)
        }
        NBTCompression::ZLib => {
            let mut zlr = flate2::bufread::ZlibDecoder::new(b);
            copy(&mut zlr, &mut enc)
        }
    }?;
    enc.finish()?;
    Ok(())
}

#[cfg(feature="nbt-zopfli")]
fn get_encoder<W: Write>(w: W) -> zopfli::GzipEncoder<W> {
    use std::num::NonZeroU64;
    let zo = zopfli::Options {
        iteration_count: NonZeroU64::new(10).unwrap(),
        iterations_without_improvement: NonZeroU64::new(2).unwrap(),
        ..<zopfli::Options as Default>::default()
    };
    let mut enc = zopfli::GzipEncoder::new(zo, zopfli::BlockType::Dynamic, vout)?;
}

#[cfg(not(feature="nbt-zopfli"))]
fn get_encoder<W: Write>(w: W) -> flate2::write::GzEncoder<W> {
    flate2::write::GzEncoder::new(w, flate2::Compression::best())
}

#[derive(Debug)]
pub struct NBTError;
impl Error for NBTError {}
impl std::fmt::Display for NBTError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Invalid NBT entry")
    }
}