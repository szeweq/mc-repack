#![cfg(feature = "jpeg")]

use std::panic::catch_unwind;

use crate::cfg::{acfg, ConfigHolder};
use super::Result_;
use mozjpeg::{Compress, Decompress};

acfg!(
    /// A JPEG minifier that accepts [`JPEGConfig`].
    MinifierJPEG: JPEGConfig
);
impl ConfigHolder<MinifierJPEG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let r = catch_unwind(|| {
            let d = Decompress::with_markers(mozjpeg::ALL_MARKERS).from_mem(b)?;
            let cs = d.color_space();
            let mut comp = Compress::new(cs);
            comp.set_size(d.width(), d.height());
            comp.set_quality(self.quality.clamp(0, 100).into());
            let mut comp = comp.start_compress(Vec::new())?;
            let mut img = d.to_colorspace(cs)?;
            let sl = img.read_scanlines()?;
            img.finish()?;
            comp.write_scanlines(&sl)?;
            let writer = comp.finish()?;
            Ok(writer)
        });
        match r {
            Ok(Ok(v)) => {
                vout.extend_from_slice(&v);
                Ok(())
            }
            Ok(Err(e)) => Err(e),
            Err(e) => anyhow::bail!("JPEG minification panicked: {:?}", e)
        }
    }
}

/// Configuration for JPEG
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct JPEGConfig {
    quality: u8
}

impl Default for JPEGConfig {
    fn default() -> Self {
        Self { quality: 75 }
    }
}
