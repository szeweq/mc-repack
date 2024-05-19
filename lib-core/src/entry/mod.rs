
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use std::sync::Arc;
use crate::{cfg, errors::ErrorCollector, fop::FileOp, ProgressState};

pub use fs::{FSEntryReader, FSEntrySaver};
pub use zip::{ZipEntryReader, ZipEntrySaver};

/// Trait for reading entries for further optimization. Typically used with `EntrySaver`.
/// Any function that matches these arguments (excluding self) can be used as custom entry reader.
pub trait EntryReader {
    /// Reads entries and sends them via `tx`.
    /// The `use_blacklist` parameter is used to ignore predefined file types written in `blacklist` module.
    fn read_entries(
        self,
        tx: impl FnMut(EntryType) -> crate::Result_<()>,
        use_blacklist: bool
    ) -> crate::Result_<()>;
}

/// A struct for saving entries that have been optimized. Typically used with `EntryReader`.
/// Any function that matches these arguments (excluding self) can be used as custom entry saver.
pub struct EntrySaver<S: EntrySaverSpec>(S);

/// Saves entries in a file-based system.
pub trait EntrySaverSpec {
    /// Saves a directory.
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()>;
    /// Saves a file with a minimum file size constraint for compression.
    fn save_file(&mut self, fname: &str, buf: &[u8], min_compress: u16) -> crate::Result_<()>;
    
}
impl<T: EntrySaverSpec> EntrySaver<T> {
    /// Receives entries from `rx`, optimizes, sends progress (via `ps`), and saves them.
    /// Errors are collected with entry names.
    pub fn save_entries(
        mut self,
        rx: impl IntoIterator<Item = EntryType>,
        ev: &mut ErrorCollector,
        cfgmap: &cfg::ConfigMap,
        mut ps: impl FnMut(ProgressState) -> crate::Result_<()>,
    ) -> crate::Result_<()> {
        let mut cv = Vec::new();
        let mut n = 0;
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    ps(ProgressState::Start(u))?;
                }
                EntryType::Directory(dir) => {
                    self.0.save_dir(&dir)?;
                }
                EntryType::File(fname, buf, fop) => {
                    ps(ProgressState::Push(n, fname.clone()))?;
                    n += 1;
                    match fop {
                        FileOp::Ignore(e) => {
                            ev.collect(fname.clone(), e.into());
                        }
                        FileOp::Minify(m) => {
                            let buf: &[u8] = match m.minify(cfgmap, &buf, &mut cv) {
                                Ok(()) => &cv,
                                Err(e) => {
                                    ev.collect(fname.clone(), e);
                                    &buf
                                }
                            };
                            self.0.save_file(&fname, buf, m.compress_min())?;
                            cv.clear();
                        }
                        FileOp::Recompress(x) => {
                            self.0.save_file(&fname, &buf, x as u16)?;
                        }
                        FileOp::Pass => {
                            self.0.save_file(&fname, &buf, 0)?;
                        }
                    }
                }
            }
        }
        ps(ProgressState::Finish)
    }
}

/// An entry type based on extracted data from an archive
pub enum EntryType {
    /// Number of files stored in an archive
    Count(usize),
    /// A directory with its path
    Directory(Arc<str>),
    /// A file with its path, data and file operation
    File(Arc<str>, Box<[u8]>, FileOp)
}
impl EntryType {
    /// A shorthand function for creating a directory entry
    #[inline]
    pub fn dir(name: impl Into<Arc<str>>) -> Self {
        Self::Directory(name.into())
    }

    /// A shorthand function for creating a file entry
    #[inline]
    pub fn file(name: impl Into<Arc<str>>, data: impl Into<Box<[u8]>>, fop: FileOp) -> Self {
        Self::File(name.into(), data.into(), fop)
    }
}
