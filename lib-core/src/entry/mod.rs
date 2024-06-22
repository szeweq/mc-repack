
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use std::sync::Arc;
use crate::{cfg, errors::ErrorCollector, fop::{FileOp, TypeBlacklist}, ProgressState};

pub use fs::{FSEntryReader, FSEntrySaver};
pub use zip::{ZipEntryReader, ZipEntrySaver};

/// Reads entries from a file-based system.
pub trait EntryReaderSpec {
    /// Returns the number of entries.
    fn len(&self) -> usize;
    /// Peeks the next entry.
    fn peek(&mut self) -> Option<(Option<bool>, Box<str>)>;
    /// Skips the next entry.
    fn skip(&mut self);
    /// Reads the next entry.
    fn read(&mut self) -> crate::Result_<EntryType>;
    /// Returns `true` if there are no entries.
    fn is_empty(&self) -> bool { self.len() == 0 }
}

/// A struct for reading entries for further optimization. Typically used with `EntrySaver`.
pub struct EntryReader<R: EntryReaderSpec>(R);
impl <R: EntryReaderSpec> EntryReader<R> {
    /// Reads entries, checks if they are not blacklisted and sends them via `tx`.
    pub fn read_entries(
        mut self,
        mut tx: impl FnMut(EntryType) -> crate::Result_<()>,
        blacklist: &TypeBlacklist
    ) -> crate::Result_<()> {
        tx(EntryType::Count(self.0.len()))?;
        while let Some((is_dir, name)) = self.0.peek() {
            let fop = FileOp::by_name(&name, blacklist);
            match is_dir {
                Some(true) => {}
                Some(false) => {
                    if let FileOp::Ignore(_) = fop {
                        self.0.skip();
                        continue;
                    }
                }
                None => {
                    self.0.skip();
                    continue;
                }
            }
            let mut et = self.0.read()?;
            if let EntryType::File(n, d, _) = et {
                et = EntryType::File(n, d, fop);
            }
            tx(et)?;
        }
        Ok(())
    }
}

/// A struct for saving entries that have been optimized. Typically used with `EntryReader`.
pub struct EntrySaver<S: EntrySaverSpec>(S);

/// Saves entries in a file-based system.
pub trait EntrySaverSpec {
    /// Saves a directory.
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()>;
    /// Saves a file with a minimum file size constraint for compression.
    fn save_file(&mut self, fname: &str, buf: &[u8], min_compress: u16) -> crate::Result_<()>;
    
}
impl<S: EntrySaverSpec> EntrySaver<S> {
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
                            self.0.save_file(&fname, &buf, 24)?;
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
