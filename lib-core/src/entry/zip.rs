use bytes::Bytes;
use std::io::{BufReader, BufWriter, Cursor, Read, Seek, Write};
use zip::{
    CompressionMethod, ZipArchive, ZipWriter,
    write::{FileOptions, SimpleFileOptions},
};

use super::{EntryReader, EntrySaver, ReadEntry, SavingEntry};
use crate::Result_;

/// An entry reader implementation for ZIP archive. It reads its contents from a provided reader (with seeking).
pub struct ZipEntryReader<R: Read + Seek> {
    za: ZipArchive<R>,
    cur: usize,
}
impl<R: Read + Seek> ZipEntryReader<R> {
    /// Creates an entry reader with a specified reader.
    ///
    /// # Errors
    ///
    /// Returns an error if the reader content is not a valid ZIP archive.
    pub fn new(r: R) -> Result_<Self> {
        Ok(Self {
            za: ZipArchive::new(r)?,
            cur: 0,
        })
    }
}
impl<R: Read + Seek> ZipEntryReader<BufReader<R>> {
    /// Creates an entry reader wrapping a specified reader with a [`BufReader`].
    ///
    /// # Errors
    ///
    /// Returns an error if the reader content is not a valid ZIP archive.
    #[inline]
    pub fn new_buf(r: R) -> Result_<Self> {
        Self::new(BufReader::new(r))
    }
}
impl<T: AsRef<[u8]>> ZipEntryReader<Cursor<T>> {
    /// Creates an entry reader wrapping a specified reader with a [`Cursor`].
    ///
    /// # Errors
    ///
    /// Returns an error if the reader content is not a valid ZIP archive.
    #[inline]
    pub fn new_mem(t: T) -> Result_<Self> {
        Self::new(Cursor::new(t))
    }
}
impl<R: Read + Seek> EntryReader for ZipEntryReader<R> {
    type RE<'a>
        = ReadZipFileEntry<'a, R>
    where
        R: 'a;
    fn read_next(&mut self) -> Option<Self::RE<'_>> {
        let za = &mut self.za;
        let jfc = za.len();
        if self.cur >= jfc {
            None
        } else {
            let idx = self.cur;
            self.cur += 1;
            Some(ReadZipFileEntry { zip: za, idx })
        }
    }
    #[inline]
    fn read_len(&self) -> usize {
        self.za.len()
    }
}

/// A read entry of a ZIP archive.
pub struct ReadZipFileEntry<'a, RS: Read + Seek> {
    zip: &'a mut ZipArchive<RS>,
    idx: usize,
}
impl<RS: Read + Seek> ReadEntry for ReadZipFileEntry<'_, RS> {
    fn meta(&self) -> (Option<bool>, Box<str>) {
        self.zip
            .name_for_index(self.idx)
            .map_or_else(|| (None, "".into()), |n| (Some(n.ends_with('/')), n.into()))
    }
    fn data(self) -> crate::Result_<Bytes> {
        let mut obuf = Vec::new();
        let mut jf = self.zip.by_index(self.idx)?;
        obuf.reserve_exact(jf.size() as usize);
        jf.read_to_end(&mut obuf)?;
        Ok(obuf.into())
    }
}

#[cfg(feature = "zip-zopfli")]
const MAX_LEVEL: i64 = 24;

#[cfg(not(feature = "zip-zopfli"))]
const MAX_LEVEL: i64 = 9;

/// An entry saver implementation for ZIP archive. It writes entries to it using a provided writer.
pub struct ZipEntrySaver<W: Write + Seek> {
    w: ZipWriter<BufWriter<W>>,
    keep_dirs: bool,
    opts_deflated: SimpleFileOptions,
    opts_stored: SimpleFileOptions,
}
impl<W: Write + Seek> ZipEntrySaver<W> {
    /// Creates an entry saver with a seekable writer.
    pub fn new(w: W, keep_dirs: bool) -> Self {
        Self {
            w: ZipWriter::new(BufWriter::new(w)),
            keep_dirs,
            opts_deflated: FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .compression_level(Some(MAX_LEVEL)),
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        }
    }
    /// Creates an entry saver with custom file options for ZIP archive and a seekable writer.
    pub fn custom(
        w: W,
        keep_dirs: bool,
        opts_stored: SimpleFileOptions,
        opts_deflated: SimpleFileOptions,
    ) -> Self {
        Self {
            w: ZipWriter::new(BufWriter::new(w)),
            keep_dirs,
            opts_deflated,
            opts_stored,
        }
    }
    /// Creates an entry saver with custom compression level for deflated entries of ZIP archive and a seekable writer.
    pub fn custom_compress(w: W, keep_dirs: bool, compress: impl Into<i64>) -> Self {
        Self {
            w: ZipWriter::new(BufWriter::new(w)),
            keep_dirs,
            opts_deflated: FileOptions::default()
                .compression_method(CompressionMethod::Deflated)
                .compression_level(Some(compress.into())),
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        }
    }
}
impl<W: Write + Seek> EntrySaver for ZipEntrySaver<W> {
    fn save(&mut self, name: &str, entry: SavingEntry) -> crate::Result_<()> {
        let z = &mut self.w;
        match entry {
            SavingEntry::Directory => {
                if self.keep_dirs && name != ".cache/" {
                    z.add_directory(name, self.opts_stored)?;
                }
            }
            SavingEntry::File(data, compress_min) => {
                z.start_file(
                    name,
                    if compress_check(data, compress_min as usize) {
                        self.opts_deflated
                    } else {
                        self.opts_stored
                    },
                )?;
                z.write_all(data)?;
            }
        }
        Ok(())
    }
}

/// Check if data should be compressed. If the compressed size is smaller than original, then the compression should be chosen.
pub fn compress_check(b: &[u8], compress_min: usize) -> bool {
    let lb = b.len();
    if lb > compress_min {
        if calc_entropy(b) < 7.0 {
            return true;
        }
        let mut d =
            flate2::write::DeflateEncoder::new(std::io::sink(), flate2::Compression::best());
        if d.write_all(b).and_then(|_| d.try_finish()).is_ok() && d.total_out() as usize + 8 < lb {
            return true;
        }
    }
    false
}

fn calc_entropy(b: &[u8]) -> f64 {
    if b.is_empty() {
        return 0.0;
    }
    let mut freq = [0usize; 256];
    for &b in b {
        freq[b as usize] += 1;
    }
    let total = b.len() as f64;
    let logt = total.log2();
    let e = freq
        .into_iter()
        .filter_map(|f| match f {
            0 => None,
            n => {
                let nf = n as f64;
                Some(-nf * (nf.log2() - logt))
            }
        })
        .sum::<f64>()
        / total;
    assert!((0.0..=8.0).contains(&e), "Invalid entropy: {e}");
    e
}
