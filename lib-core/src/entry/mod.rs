
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use std::sync::Arc;
use crate::{cfg, errors::ErrorCollector, fop::{FileOp, TypeBlacklist}, ProgressState};

use bytes::Bytes;
pub use fs::{FSEntryReader, FSEntrySaver};
pub use zip::{ZipEntryReader, ZipEntrySaver};

/// Reads entries from a file-based system. Typically used with `EntrySaver`.
pub trait EntryReader {
    /// A type for reading entries.
    type RE<'a>: ReadEntry where Self: 'a;

    /// Reads the next entry.
    fn read_next(&mut self) -> Option<Self::RE<'_>>;

    /// Returns the number of entries.
    fn read_len(&self) -> usize;

    /// Creates an iterator for reading entries.
    fn read_iter(&mut self) -> ReadEntryIter<Self> where Self: Sized { ReadEntryIter(self) }

    /// Reads entries, checks if they are not blacklisted and sends them via `tx`.
    fn read_entries(
        mut self,
        mut tx: impl FnMut(EntryType) -> crate::Result_<()>,
        blacklist: &TypeBlacklist
    ) -> crate::Result_<()> where Self: Sized {
        tx(EntryType::Count(self.read_len()))?;
        for re in self.read_iter() {
            let (is_dir, name) = re.meta();
            let fop = FileOp::by_name(&name, blacklist);
            let et = match is_dir {
                Some(true) => {
                    EntryType::dir(name)
                }
                Some(false) => {
                    if let FileOp::Ignore(_) = fop {
                        continue;
                    }
                    EntryType::file(name, re.data()?, fop)
                }
                None => {
                    continue;
                }
            };
            tx(et)?;
        }
        Ok(())
    }
}

/// An iterator for reading entries from an entry reader.
#[repr(transparent)]
pub struct ReadEntryIter<'a, R: EntryReader>(&'a mut R);
impl <'a, R: EntryReader + 'a> Iterator for ReadEntryIter<'a, R> {
    type Item = R::RE<'a>;

    #[inline]
    fn next(&mut self) -> Option<Self::Item> {
        // SAFETY: As long as R has a lifetime, it is safe to call next() on it.
        unsafe { std::mem::transmute(self.0.read_next()) }
    }
}

/// A trait for reading entries data. While the [`ReadEntry::data()`] is called, this entry is consumed.
pub trait ReadEntry {
    /// Returns the entry metadata.
    fn meta(&self) -> (Option<bool>, Box<str>);

    /// Reads the entry data.
    fn data(self) -> crate::Result_<Bytes>;
}

/// Saves entries in a file-based system. Typically used with `EntryReader`.
pub trait EntrySaver {
    /// Saves an entry.
    fn save(&mut self, name: &str, entry: SavingEntry) -> crate::Result_<()>;

    /// Receives entries from `rx`, optimizes, sends progress (via `ps`), and saves them.
    /// Errors are collected with entry names.
    fn save_entries(
        mut self,
        rx: impl IntoIterator<Item = EntryType>,
        ev: &mut ErrorCollector,
        cfgmap: &cfg::ConfigMap,
        mut ps: impl FnMut(ProgressState) -> crate::Result_<()>,
    ) -> crate::Result_<()> where Self: Sized {
        let mut cv = Vec::new();
        let mut n = 0;
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    ps(ProgressState::Start(u))?;
                }
                EntryType::Directory(dir) => {
                    self.save(&dir, SavingEntry::Directory)?;
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
                            self.save(&fname, SavingEntry::File(buf, m.compress_min()))?;
                            cv.clear();
                        }
                        FileOp::Recompress(x) => {
                            self.save(&fname, SavingEntry::File(&buf, x as u16))?;
                        }
                        FileOp::Pass => {
                            self.save(&fname, SavingEntry::File(&buf, 24))?;
                        }
                    }
                }
            }
        }
        ps(ProgressState::Finish)
    }
}

/// An entry type based on extracted data from an archive
#[derive(Clone)]
pub enum EntryType {
    /// Number of files stored in an archive
    Count(usize),
    /// A directory with its path
    Directory(Arc<str>),
    /// A file with its path, data and file operation
    File(Arc<str>, Bytes, FileOp)
}
impl EntryType {
    /// A shorthand function for creating a directory entry
    #[inline]
    pub fn dir(name: impl Into<Arc<str>>) -> Self {
        Self::Directory(name.into())
    }

    /// A shorthand function for creating a file entry
    #[inline]
    pub fn file(name: impl Into<Arc<str>>, data: impl Into<Bytes>, fop: FileOp) -> Self {
        Self::File(name.into(), data.into(), fop)
    }
}

/// A type for saving entries.
pub enum SavingEntry<'a> {
    /// A directory
    Directory,
    /// A file with data and a minimum compression constraint
    File(&'a [u8], u16)
}