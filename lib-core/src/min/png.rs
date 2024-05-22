#![cfg(feature = "png")]

use state::InitCell;

use crate::cfg::{self, acfg, ConfigHolder};
use super::Result_;

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
#[derive(Default)]
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct PNGConfig {
    #[cfg_attr(feature = "serde-cfg", serde(skip))]
    oxipng_opts: InitCell<oxipng::Options>,
    #[cfg(feature = "png-zopfli")]
    use_zopfli: cfg::CfgZopfli
}
impl PNGConfig {
    fn png_opts(&self) -> &oxipng::Options {
        self.oxipng_opts.get_or_init(|| {
            let mut popts = oxipng::Options {
                fix_errors: true,
                strip: oxipng::StripChunks::Safe,
                optimize_alpha: true,
                #[cfg(feature = "png-zopfli")]
                deflate: self.use_zopfli.iter_count().map_or(
                    oxipng::Deflaters::Libdeflater { compression: 12 },
                    |ic| oxipng::Deflaters::Zopfli { iterations: ic }
                ),
                #[cfg(not(feature = "png-zopfli"))]
                deflate: oxipng::Deflaters::Libdeflater { compression: 12 },
                ..Default::default()
            };
            popts.filter.insert(oxipng::RowFilter::Up);
            popts.filter.insert(oxipng::RowFilter::Paeth);

            popts
        })
    }
}