
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use std::io;
use crate::{optimizer::{EntryType, ProgressState, StrError, ERR_SIGNFILE}, errors::ErrorCollector, fop};

use crossbeam_channel::{Sender, Receiver};

/// Trait for reading entries for further optimization. Typically used with `EntrySaver`.
/// Any function that matches these arguments (excluding self) can be used as custom entry reader.
pub trait EntryReader {
    /// Reads entries and sends them via `tx`.
    /// The `use_blacklist` parameter is used to ignore predefined file types written in `blacklist` module.
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> io::Result<()>;
}

/// A struct for saving entries that have been optimized. Typically used with `EntryReader`.
/// Any function that matches these arguments (excluding self) can be used as custom entry saver.
pub struct EntrySaver<S: EntrySaverSpec>(S);

/// Saves entries in a file-based system.
pub trait EntrySaverSpec {
    /// Saves a directory.
    fn save_dir(&mut self, dir: &str) -> io::Result<()>;
    /// Saves a file with a minimum file size constraint for compression.
    fn save_file(&mut self, fname: &str, buf: &[u8], min_compress: usize) -> io::Result<()>;
    
}
impl<T: EntrySaverSpec> EntrySaver<T> {
    /// Receives entries from `rx`, optimizes, sends progress (via `ps`), and saves them.
    /// Errors are collected with entry names.
    pub fn save_entries(
        mut self,
        rx: Receiver<EntryType>,
        ev: &mut dyn ErrorCollector,
        ps: &Sender<ProgressState>
    ) -> io::Result<i64> {
        let mut dsum = 0;
        let mut cv = Vec::new();
        let mut n = 0;
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    ps.send(ProgressState::Start(u)).unwrap();
                }
                EntryType::Directory(dir) => {
                    self.0.save_dir(&dir)?;
                }
                EntryType::File(fname, buf, fop) => {
                    ps.send(ProgressState::Push(n, fname.clone())).unwrap();
                    n += 1;
                    use fop::FileOp::*;
                    match fop {
                        Ignore => {
                            ev.collect(&fname, Box::new(crate::blacklist::BlacklistedFile));
                        }
                        Signfile => {
                            ev.collect(&fname, Box::new(StrError(ERR_SIGNFILE.to_string())));
                        }
                        Warn(x) => {
                            ev.collect(&fname, Box::new(StrError(x)));
                            self.0.save_file(&fname, &buf, 0)?;
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
                            self.0.save_file(&fname, buf, m.compress_min())?;
                            cv.clear();
                        }
                        Recompress(x) => {
                            self.0.save_file(&fname, &buf, x)?;
                        }
                    }
                }
            }
        }
        ps.send(ProgressState::Finish).unwrap();
        Ok(dsum)
    }
}

impl<T: FnOnce(Sender<EntryType>, bool) -> io::Result<()>> EntryReader for T {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> io::Result<()> {
        self(tx, use_blacklist)
    }
}