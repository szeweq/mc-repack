use std::io::{self, Read, Seek, BufReader, Write, BufWriter};
use crossbeam_channel::{Sender, Receiver};
use flate2::bufread::DeflateEncoder;
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};

use crate::{entry::{EntryReader, EntrySaver}, optimizer::{EntryType, ProgressState, StrError, ERR_SIGNFILE}, fop::{FileOp, check_file_by_name}, errors::ErrorCollector, blacklist};

/// An entry reader implementation for ZIP archive. It reads its contents from a provided reader (with seeking).
pub struct ZipEntryReader<R: Read + Seek> {
    r: R
}
impl <R: Read + Seek> ZipEntryReader<R> {
    /// Creates an entry reader with a specified reader.
    pub fn new(r: R) -> Self {
        Self { r }
    }
}
impl <R: Read + Seek> EntryReader for ZipEntryReader<R> {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> io::Result<()> {
        let mut za = ZipArchive::new(BufReader::new(self.r))?;
        let jfc = za.len() as u64;
        tx.send(EntryType::Count(jfc)).unwrap();
        for i in 0..jfc {
            let mut jf = za.by_index(i as usize)?;
            let fname = jf.name().to_string();
            tx.send(if fname.ends_with('/') {
                EntryType::Directory(fname)
            } else {
                let fop = check_file_by_name(&fname, use_blacklist);
                let mut obuf = Vec::new();
                match fop {
                    FileOp::Ignore => {}
                    _ => {
                        obuf.reserve_exact(jf.size() as usize);
                        jf.read_to_end(&mut obuf)?;
                    }
                }
                EntryType::File(fname, obuf, fop)
            }).unwrap();
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
    pub fn new(w: W) -> Self {
        Self {
            w: ZipWriter::new(BufWriter::new(w)),
            file_opts: FileOptions::default().compression_level(Some(9))
        }
    }
    /// Creates an entry saver with custom file options for ZIP archive and seekable writer.
    pub fn custom(w: W, file_opts: FileOptions) -> Self {
        Self {
            w: ZipWriter::new(BufWriter::new(w)), file_opts
        }
    }

    fn pack_file(&mut self, name: &str, data: &[u8], compress_min: usize) -> io::Result<()> {
        let z = &mut self.w;
        z.start_file(name, self.file_opts.clone()
            .compression_method(compress_check(data, compress_min))
        )?;
        z.write_all(data)
    }
}
impl <W: Write + Seek> EntrySaver for ZipEntrySaver<W> {
    fn save_entries(
        mut self,
        rx: Receiver<EntryType>,
        ev: &mut dyn ErrorCollector,
        ps: &Sender<ProgressState>
    ) -> io::Result<i64> {
        let mut dsum = 0;
        let mut cv = Vec::new();
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    ps.send(ProgressState::Start(u)).unwrap();
                }
                EntryType::Directory(d) => {
                    if d != ".cache/" {
                        self.w.add_directory(d, self.file_opts.clone())?;
                    }
                }
                EntryType::File(fname, buf, fop) => {
                    use FileOp::*;
                    ps.send(ProgressState::Push(fname.clone())).unwrap();
                    match fop {
                        Recompress(cmin) => {
                            self.pack_file(
                                &fname,
                                &buf,
                                cmin
                            )?;
                        }
                        Minify(m) => {
                            let fsz = buf.len() as i64;
                            let buf = match m.minify(&buf, &mut cv) {
                                Ok(()) => &cv,
                                Err(e) => {
                                    ev.collect(&fname, e);
                                    &buf
                                }
                            };
                            dsum -= (buf.len() as i64) - fsz;
                            self.pack_file(&fname, &buf, m.compress_min())?;
                            cv.clear();
                        }
                        Ignore => {
                            ev.collect(&fname, Box::new(blacklist::BlacklistedFile));
                        }
                        Warn(x) => {
                            ev.collect(&fname, Box::new(StrError(x)));
                            self.pack_file(&fname, &buf, 0)?;
                        }
                        Signfile => {
                            ev.collect(&fname, Box::new(StrError(ERR_SIGNFILE.to_string())));
                        }
                    }
                }
            }
        }
        self.w.finish()?;
        ps.send(ProgressState::Finish).unwrap();
        Ok(dsum)
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