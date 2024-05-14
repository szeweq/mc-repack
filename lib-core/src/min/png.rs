#![cfg(feature = "png")]

use crate::cfg::{acfg, ConfigHolder};

use super::Result_;

#[cfg(feature = "png-zopfli")]
const BEST_ZOPFLI: oxipng::Deflaters = oxipng::Deflaters::Zopfli { iterations: 15 };

const BEST_DEFLATE: oxipng::Deflaters = oxipng::Deflaters::Libdeflater { compression: 12 };

acfg!(MinifierPNG: PNGConfig);
impl ConfigHolder<MinifierPNG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let v = oxipng::optimize_from_memory(b, &self.oxipng_opts)?;
        let _ = std::mem::replace(vout, v);
        Ok(())
    }
}

/// Configuration for PNG optimizer
pub struct PNGConfig {
    oxipng_opts: oxipng::Options,
    #[cfg(feature = "png-zopfli")]
    use_zopfli: bool
}
impl Default for PNGConfig {
    fn default() -> Self {
        let mut popts = oxipng::Options {
            fix_errors: true,
            strip: oxipng::StripChunks::Safe,
            optimize_alpha: true,
            deflate: BEST_DEFLATE,
            ..Default::default()
        };
        popts.filter.insert(oxipng::RowFilter::Up);
        popts.filter.insert(oxipng::RowFilter::Paeth);
        
        Self { oxipng_opts: popts, #[cfg(feature = "png-zopfli")] use_zopfli: false }
    }
}
impl PNGConfig {
    #[cfg(feature = "png-zopfli")]
    /// Sets whether to use Zopfli for PNG compression
    pub fn use_zopfli(&mut self, v: bool) -> &mut Self {
        self.use_zopfli = v;
        self.oxipng_opts.deflate = if v { BEST_ZOPFLI } else { BEST_DEFLATE };
        self
    }
}