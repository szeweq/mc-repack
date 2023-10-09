use std::{io::{self, Read, Seek, BufReader, Write, BufWriter}, sync::Arc};
use crossbeam_channel::{Sender, SendError};
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
    ) -> io::Result<()> {
        const SEND_ERR: fn(SendError<EntryType>) -> io::Error = |e: SendError<EntryType>| {
            io::Error::new(io::ErrorKind::Other, e)
        };
        let mut za = ZipArchive::new(BufReader::new(self.r))?;
        let jfc = za.len();
        tx.send(EntryType::Count(jfc)).map_err(SEND_ERR)?;
        for i in 0..jfc {
            let mut jf = za.by_index(i)?;
            let fname: Arc<str> = jf.name().into();
            tx.send(if fname.ends_with('/') {
                EntryType::Directory(fname)
            } else {
                let fop = FileOp::by_name(&fname, use_blacklist);
                let mut obuf = Vec::new();
                match fop {
                    FileOp::Ignore(_) => {}
                    _ => {
                        obuf.reserve_exact(jf.size() as usize);
                        jf.read_to_end(&mut obuf)?;
                    }
                }
                EntryType::File(fname, obuf, fop)
            }).map_err(SEND_ERR)?;
        }
        Ok(())
    }
}

/// An entry saver implementation for ZIP archive. It writes entries to it using a provided writer.
pub struct ZipEntrySaver<W: Write + Seek> {
    w: ZipWriter<BufWriter<W>>,
    file_opts: FileOptions
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
    pub fn custom(w: W, file_opts: FileOptions) -> EntrySaver<Self> {
        EntrySaver(Self {
            w: ZipWriter::new(BufWriter::new(w)), file_opts
        })
    }
}
impl <W: Write + Seek> EntrySaverSpec for ZipEntrySaver<W> {
    fn save_dir(&mut self, dir: &str) -> io::Result<()> {
        if dir != "./cache" {
            self.w.add_directory(dir, self.file_opts)?;
        }
        Ok(())
    }
    fn save_file(&mut self, name: &str, data: &[u8], compress_min: u32) -> io::Result<()> {
        let z = &mut self.w;
        z.start_file(name, self.file_opts
            .compression_method(compress_check(data, compress_min as usize))
        )?;
        z.write_all(data)
    }
}

fn compress_check(b: &[u8], compress_min: usize) -> CompressionMethod {
    let lb = b.len();
    let nc = if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false };
    if nc { CompressionMethod::DEFLATE } else { CompressionMethod::STORE }
}