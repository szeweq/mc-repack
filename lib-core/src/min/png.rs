#![cfg(feature = "png")]

use crate::cfg::{acfg, ConfigHolder};

use super::Result_;

#[cfg(feature = "png-zopfli")]
const BEST_DEFLATE: oxipng::Deflaters = oxipng::Deflaters::Zopfli { iterations: 15 };
#[cfg(not(feature = "png-zopfli"))]
const BEST_DEFLATE: oxipng::Deflaters = oxipng::Deflaters::Libdeflater { compression: 12 };

acfg!(MinifierPNG: PNGConfig);
impl ConfigHolder<MinifierPNG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let v = oxipng::optimize_from_memory(b, &self.oxipng_opts)?;
        let _ = std::mem::replace(vout, v);
        Ok(())
    }
}

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
