#![cfg(feature = "jar")]

use std::io::{Cursor, Read, Write};

use zip::{write::{FileOptions, SimpleFileOptions}, CompressionMethod, ZipArchive, ZipWriter};

use crate::{cfg::{acfg, ConfigHolder}, entry::zip::compress_check};
use super::Result_;


acfg!(
    /// A JAR archive repacker that accepts [`JARConfig`].
    MinifierJAR: JARConfig
);

impl ConfigHolder<MinifierJAR> {
    pub(super) fn minify(&self, b: &[u8], vout: &mut Vec<u8>) -> Result_ {
        let mut zread = ZipArchive::new(Cursor::new(b))?;
        let stored: SimpleFileOptions = FileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated: SimpleFileOptions = FileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(self.compress_level()));
        let mut zwrite = ZipWriter::new(Cursor::new(vout));
        let mut v = Vec::new();
        for i in 0..zread.len() {
            let Some(name) = zread.name_for_index(i).map(|s| s.to_string()) else { continue; };
            if !self.keep_dirs && name.ends_with('/') { continue; }

            let mut zfile = zread.by_index(i)?;
            v.clear();
            v.reserve(zfile.size() as usize);
            zfile.read_to_end(&mut v)?;
            zwrite.start_file(name, if compress_check(&v, 24) {
                deflated
            } else {
                stored
            })?;
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
    pub use_zopfli: crate::cfg::CfgZopfli
}
impl JARConfig {
    #[inline]
    #[cfg(feature = "zip-zopfli")]
    fn compress_level(&self) -> i64 {
        self.use_zopfli.iter_count().map_or(9, |ic| 9 + ((u8::from(ic)) as i64))
    }

    #[inline]
    #[cfg(not(feature = "zip-zopfli"))]
    fn compress_level(&self) -> i64 {
        9
    }
}