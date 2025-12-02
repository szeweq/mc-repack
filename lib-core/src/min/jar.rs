#![cfg(feature = "jar")]

use std::io::{Cursor, Read, Write};

use zip::{
    CompressionMethod, HasZipMetadata, ZipArchive, ZipWriter,
    write::{FileOptions, SimpleFileOptions},
};

use super::Result_;
use crate::{
    cfg::{ConfigHolder, acfg},
    entry::zip::compress_check,
};

acfg!(
    /// A JAR archive repacker that accepts [`JARConfig`].
    MinifierJAR: JARConfig
);

impl ConfigHolder<MinifierJAR> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let mut zread = ZipArchive::new(Cursor::new(b))?;
        let stored: SimpleFileOptions =
            FileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated: SimpleFileOptions = FileOptions::default()
            .compression_method(CompressionMethod::Deflated)
            .compression_level(Some(self.compress_level()));
        let mut zwrite = ZipWriter::new(Cursor::new(vout));
        let mut v = Vec::new();
        for i in 0..zread.len() {
            let Some(is_dir) = zread.name_for_index(i).map(|s| s.ends_with('/')) else {
                continue;
            };
            if !self.keep_dirs && is_dir {
                continue;
            }

            let mut zfile = zread.by_index(i)?;
            v.clear();
            v.reserve(zfile.get_metadata().uncompressed_size as usize);
            zfile.read_to_end(&mut v)?;
            zwrite.start_file(
                &zfile.get_metadata().file_name,
                if compress_check(&v, 24) {
                    deflated
                } else {
                    stored
                },
            )?;
            zwrite.write_all(&v)?;
        }

        Ok(())
    }
}

/// Configuration for JAR repacker
#[derive(Default)]
#[cfg_attr(feature = "serde-cfg", derive(serde::Serialize, serde::Deserialize))]
pub struct JARConfig {
    /// Keep directories in the archive
    pub keep_dirs: bool,

    #[cfg(feature = "zip-zopfli")]
    /// Enables Zopfli compression (better, but slower)
    pub use_zopfli: crate::cfg::CfgZopfli,
}
impl JARConfig {
    #[inline]
    #[cfg(feature = "zip-zopfli")]
    fn compress_level(&self) -> i64 {
        self.use_zopfli
            .iter_count()
            .map_or(9, |ic| 9 + i64::from(ic.get()))
    }

    #[inline]
    #[cfg(not(feature = "zip-zopfli"))]
    fn compress_level(&self) -> i64 {
        9
    }
}
