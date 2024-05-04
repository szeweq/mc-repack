use std::{io::{Read, Seek, BufReader, Write, BufWriter}, sync::Arc};
use crossbeam_channel::Sender;
use flate2::bufread::DeflateEncoder;
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};

use crate::{optimizer::EntryType, fop::FileOp};
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
impl <R: Read + Seek> EntryReader for ZipEntryReader<R> {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> crate::Result_<()> {
        let mut za = ZipArchive::new(BufReader::new(self.r))?;
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
                    obuf.reserve_exact(jf.size() as usize);
                    jf.read_to_end(&mut obuf)?;
                }
                EntryType::File(fname, obuf.into(), fop)
            })?;
        }
        Ok(())
    }
}

/// An entry saver implementation for ZIP archive. It writes entries to it using a provided writer.
pub struct ZipEntrySaver<W: Write + Seek> {
    w: ZipWriter<BufWriter<W>>,
    file_opts: FileOptions<()>
}
impl <W: Write + Seek> ZipEntrySaver<W> {
    /// Creates an entry saver with a seekable writer.
    pub fn new(w: W) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)),
            file_opts: FileOptions::default().compression_level(Some(9))
        })
    }
    /// Creates an entry saver with custom file options for ZIP archive and seekable writer.
    pub fn custom(w: W, file_opts: FileOptions<()>) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)), file_opts
        })
    }
}
impl <W: Write + Seek> EntrySaverSpec for ZipEntrySaver<W> {
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()> {
        if dir != ".cache/" {
            self.w.add_directory(dir, self.file_opts)?;
        }
        Ok(())
    }
    fn save_file(&mut self, name: &str, data: &[u8], compress_min: u16) -> crate::Result_<()> {
        let z = &mut self.w;
        z.start_file(name, self.file_opts
            .compression_method(compress_check(data, compress_min as usize))
        )?;
        z.write_all(data)?;
        Ok(())
    }
}

fn compress_check(b: &[u8], compress_min: usize) -> CompressionMethod {
    let lb = b.len();
    if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        if sum < lb { return CompressionMethod::DEFLATE }
    }
    CompressionMethod::STORE
}