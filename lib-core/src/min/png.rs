#![cfg(feature = "png")]

use state::InitCell;

use crate::cfg::{acfg, ConfigHolder};
use super::Result_;

acfg!(
    /// A PNG minifier that accepts [`PNGConfig`].
    MinifierPNG: PNGConfig
);
impl ConfigHolder<MinifierPNG> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let v = oxipng::optimize_from_memory(b, self.png_opts())?;
        vout.extend_from_slice(&v);
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
    use_zopfli: crate::cfg::CfgZopfli
}
impl PNGConfig {
    fn png_opts(&self) -> &oxipng::Options {
        self.oxipng_opts.get_or_init(|| {
            let mut popts = oxipng::Options {
                fix_errors: true,
                strip: oxipng::StripChunks::Safe,
                optimize_alpha: true,
                #[cfg(feature = "png-zopfli")]
                deflater: self.use_zopfli.iter_count().map_or(
                    oxipng::Deflater::Libdeflater { compression: 12 },
                    |ic| oxipng::Deflater::Zopfli(zopfli::Options { iteration_count: ic.into(), ..Default::default() })
                ),
                #[cfg(not(feature = "png-zopfli"))]
                deflater: oxipng::Deflater::Libdeflater { compression: 12 },
                ..Default::default()
            };
            popts.filters.insert(oxipng::FilterStrategy::UP);
            popts.filters.insert(oxipng::FilterStrategy::PAETH);

            popts
        })
    }
}