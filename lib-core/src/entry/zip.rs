use std::{io::{BufReader, BufWriter, Cursor, Read, Seek, Write}, sync::Arc};
use zip::{write::{FileOptions, SimpleFileOptions}, CompressionMethod, ZipArchive, ZipWriter};

use crate::{fop::FileOp, Result_};
use super::{EntryReader, EntryReaderSpec, EntrySaver, EntrySaverSpec, EntryType};

/// An entry reader implementation for ZIP archive. It reads its contents from a provided reader (with seeking).
pub struct ZipEntryReader<R: Read + Seek> {
    za: ZipArchive<R>,
    cur: usize
}
impl <R: Read + Seek> ZipEntryReader<R> {
    /// Creates an entry reader with a specified reader.
    pub fn new(r: R) -> Result_<EntryReader<Self>> {
        Ok(EntryReader(Self { za: ZipArchive::new(r)?, cur: 0 }))
    }
}
impl <R: Read + Seek> ZipEntryReader<BufReader<R>> {
    /// Creates an entry reader wrapping a specified reader with a [`BufReader`].
    #[inline]
    pub fn new_buf(r: R) -> Result_<EntryReader<Self>> {
        Self::new(BufReader::new(r))
    }
}
impl <T: AsRef<[u8]>> ZipEntryReader<Cursor<T>> {
    /// Creates an entry reader wrapping a specified reader with a [`Cursor`].
    #[inline]
    pub fn new_mem(t: T) -> Result_<EntryReader<Self>> {
        Self::new(Cursor::new(t))
    }
}
impl <R: Read + Seek> EntryReaderSpec for ZipEntryReader<R> {
    fn len(&self) -> usize {
        self.za.len()
    }
    fn peek(&mut self) -> Option<(Option<bool>, Box<str>)> {
        let za = &self.za;
        let jfc = za.len();
        if self.cur >= jfc {
            None
        } else {
            Some(za.name_for_index(self.cur).map_or_else(
                || (None, "".into()),
                |n| (Some(n.ends_with('/')), n.into())
            ))
        }
    }
    fn skip(&mut self) {
        self.cur += 1;
    }
    fn read(&mut self) -> crate::Result_<EntryType> {
        let za = &mut self.za;
        let jfc = za.len();
        if self.cur >= jfc {
            anyhow::bail!("No more entries");
        } else {
            let i = self.cur;
            self.cur += 1;
            let name: Arc<str> = za.name_for_index(i).unwrap_or_default().into();
            Ok(if name.ends_with('/') {
                EntryType::dir(name)
            } else {
                let mut obuf = Vec::new();
                let mut jf = za.by_index(i)?;
                obuf.reserve_exact(jf.size() as usize);
                jf.read_to_end(&mut obuf)?;
                EntryType::file(name, obuf, FileOp::Pass)
            })
        }
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
    opts_stored: SimpleFileOptions
}
impl <W: Write + Seek> ZipEntrySaver<W> {
    /// Creates an entry saver with a seekable writer.
    pub fn new(w: W, keep_dirs: bool) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)),
            keep_dirs,
            opts_deflated: FileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(MAX_LEVEL)),
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        })
    }
    /// Creates an entry saver with custom file options for ZIP archive and a seekable writer.
    pub fn custom(w: W, keep_dirs: bool, opts_stored: SimpleFileOptions, opts_deflated: SimpleFileOptions) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)), keep_dirs, opts_deflated, opts_stored
        })
    }
    /// Creates an entry saver with custom compression level for deflated entries of ZIP archive and a seekable writer.
    pub fn custom_compress(w: W, keep_dirs: bool, compress: impl Into<i64>) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)),
            keep_dirs,
            opts_deflated: FileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(compress.into())),
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        })
    }
}
impl <W: Write + Seek> EntrySaverSpec for ZipEntrySaver<W> {
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()> {
        if self.keep_dirs && dir != ".cache/" {
            self.w.add_directory(dir, self.opts_stored)?;
        }
        Ok(())
    }
    fn save_file(&mut self, name: &str, data: &[u8], compress_min: u16) -> crate::Result_<()> {
        let z = &mut self.w;
        z.start_file(name, if compress_check(data, compress_min as usize) {
            self.opts_deflated
        } else {
            self.opts_stored
        })?;
        z.write_all(data)?;
        Ok(())
    }
}

fn compress_check(b: &[u8], compress_min: usize) -> bool {
    let lb = b.len();
    if lb > compress_min {
        if calc_entropy(b) < 7.0 { return true }
        let mut d = flate2::write::DeflateEncoder::new(std::io::sink(), flate2::Compression::best());
        if d.write_all(b).and_then(|_| d.try_finish()).is_ok() && d.total_out() as usize + 8 < lb { return true }
    }
    false
}

fn calc_entropy(b: &[u8]) -> f32 {
    if b.is_empty() { return 0.0; }
    let mut freq = [0usize; 256];
    for &b in b { freq[b as usize] += 1; }
    let total = b.len() as f32;
    let logt = total.log2();
    let e = freq.into_iter().filter(|&f| f != 0)
        .map(|f| -(f as f32) * ((f as f32).log2() - logt))
        .sum::<f32>() / total;
    assert!((0.0..=8.0).contains(&e), "Invalid entropy: {}", e);
    e
}