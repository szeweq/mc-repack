#![cfg(feature = "png")]

use state::InitCell;

use crate::cfg::{acfg, ConfigHolder};
use super::Result_;

#[cfg(feature = "png-zopfli")]
const BEST_ZOPFLI: oxipng::Deflaters = oxipng::Deflaters::Zopfli { iterations: 15 };

const BEST_DEFLATE: oxipng::Deflaters = oxipng::Deflaters::Libdeflater { compression: 12 };

acfg!(
    /// A PNG minifier that accepts [`PNGConfig`].
    MinifierPNG: PNGConfig
);
impl ConfigHolder<MinifierPNG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let v = oxipng::optimize_from_memory(b, self.png_opts())?;
        let _ = std::mem::replace(vout, v);
        Ok(())
    }
}

/// Configuration for PNG optimizer
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct PNGConfig {
    #[cfg_attr(feature = "serde-cfg", serde(skip))]
    oxipng_opts: InitCell<oxipng::Options>,
    #[cfg(feature = "png-zopfli")]
    use_zopfli: bool
}
impl Default for PNGConfig {
    fn default() -> Self {
        Self { oxipng_opts: InitCell::new(), #[cfg(feature = "png-zopfli")] use_zopfli: false }
    }
}
impl PNGConfig {
    fn png_opts(&self) -> &oxipng::Options {
        self.oxipng_opts.get_or_init(|| {
            let mut popts = oxipng::Options {
                fix_errors: true,
                strip: oxipng::StripChunks::Safe,
                optimize_alpha: true,
                #[cfg(feature = "png-zopfli")]
                deflate: if self.use_zopfli { BEST_ZOPFLI } else { BEST_DEFLATE },
                #[cfg(not(feature = "png-zopfli"))]
                deflate: BEST_DEFLATE,
                ..Default::default()
            };
            popts.filter.insert(oxipng::RowFilter::Up);
            popts.filter.insert(oxipng::RowFilter::Paeth);

            popts
        })
    }
}