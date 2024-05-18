use std::{io::{BufReader, BufWriter, Read, Seek, Write}, sync::Arc};
use crossbeam_channel::Sender;
use zip::{write::{FileOptions, SimpleFileOptions}, CompressionMethod, ZipArchive, ZipWriter};

use crate::{fop::FileOp, optimizer::EntryType};
use super::{EntryReader, EntrySaverSpec, EntrySaver};

/// An entry reader implementation for ZIP archive. It reads its contents from a provided reader (with seeking).
pub struct ZipEntryReader<R: Read + Seek> {
    r: R
}
impl <R: Read + Seek> ZipEntryReader<R> {
    /// Creates an entry reader with a specified reader.
    pub const fn new(r: R) -> Self {
        Self { r }
    }
}
impl <R: Read + Seek> ZipEntryReader<BufReader<R>> {
    /// Creates an entry reader wrapping a specified reader with a [`BufReader`].
    pub fn new_buf(r: R) -> Self {
        Self { r: BufReader::new(r) }
    }
}
impl <R: Read + Seek> EntryReader for ZipEntryReader<R> {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> crate::Result_<()> {
        let mut za = ZipArchive::new(self.r)?;
        let jfc = za.len();
        super::wrap_send(&tx, EntryType::Count(jfc))?;
        for i in 0..jfc {
            let Some(name) = za.name_for_index(i) else { continue; };
            let fname: Arc<str> = name.into();
            super::wrap_send(&tx, if fname.ends_with('/') {
                EntryType::Directory(fname)
            } else {
                let fop = FileOp::by_name(&fname, use_blacklist);
                let mut obuf = Vec::new();
                if let FileOp::Ignore(_) = fop {} else {
                    let mut jf = za.by_index(i)?;
                    if jf.compression() != CompressionMethod::Deflated { eprintln!("{}: CM {}\n", fname, jf.compression()); }
                    obuf.reserve_exact(jf.size() as usize);
                    jf.read_to_end(&mut obuf)?;
                }
                EntryType::File(fname, obuf.into(), fop)
            })?;
        }
        Ok(())
    }
}

#[cfg(feature = "zip-zopfli")]
const MAX_LEVEL: i64 = 24;

#[cfg(not(feature = "zip-zopfli"))]
const MAX_LEVEL: i64 = 9;

/// An entry saver implementation for ZIP archive. It writes entries to it using a provided writer.
pub struct ZipEntrySaver<W: Write + Seek> {
    w: ZipWriter<BufWriter<W>>,
    opts_deflated: SimpleFileOptions,
    opts_stored: SimpleFileOptions
}
impl <W: Write + Seek> ZipEntrySaver<W> {
    /// Creates an entry saver with a seekable writer.
    pub fn new(w: W) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)),
            opts_deflated: FileOptions::default().compression_method(CompressionMethod::Deflated).compression_level(Some(MAX_LEVEL)),
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        })
    }
    /// Creates an entry saver with custom file options for ZIP archive and a seekable writer.
    pub fn custom(w: W, opts_stored: SimpleFileOptions, opts_deflated: SimpleFileOptions) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)), opts_deflated, opts_stored
        })
    }
    /// Creates an entry saver with custom file options for deflated entries of ZIP archive and a seekable writer.
    pub fn custom_deflated(w: W, opts_deflated: SimpleFileOptions) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)),
            opts_deflated,
            opts_stored: FileOptions::default().compression_method(CompressionMethod::Stored),
        })
    }
}
impl <W: Write + Seek> EntrySaverSpec for ZipEntrySaver<W> {
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()> {
        if dir != ".cache/" {
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
        let mut d = flate2::write::DeflateEncoder::new(std::io::sink(), flate2::Compression::best());
        if d.write_all(b).and_then(|_| d.try_finish()).is_ok() && d.total_out() as usize + 8 < lb { return true }
    }
    false
}