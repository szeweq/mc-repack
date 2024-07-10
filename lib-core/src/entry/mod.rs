
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
        mut tx: impl FnMut(NamedEntry) -> crate::Result_<()>,
        blacklist: &TypeBlacklist
    ) -> crate::Result_<()> where Self: Sized {
        for re in self.read_iter() {
            let Some(ne) = read_entry::<Self>(re, blacklist)? else { continue };
            tx(ne)?;
        }
        Ok(())
    }
}

/// Reads an entry from an [`EntryReader`].
pub fn read_entry<R: EntryReader>(re: R::RE<'_>, blacklist: &TypeBlacklist) -> crate::Result_<Option<NamedEntry>> {
    let (is_dir, name) = re.meta();
    let Some(is_dir) = is_dir else { return Ok(None) };
    let et = if is_dir {
        NamedEntry::dir(name)
    } else {
        let fop = FileOp::by_name(&name, blacklist);
        if let FileOp::Ignore(_) = fop {
            return Ok(None);
        }
        NamedEntry::file(name, re.data()?, fop)
    };
    Ok(Some(et))
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
        rx: impl IntoIterator<Item = NamedEntry>,
        ev: &mut ErrorCollector,
        cfgmap: &cfg::ConfigMap,
        mut ps: impl FnMut(ProgressState) -> crate::Result_<()>,
    ) -> crate::Result_<()> where Self: Sized {
        let mut cv = Vec::new();
        let mut n = 0;
        for NamedEntry(name, et) in rx {
            match et {
                EntryType::Directory => {
                    self.save(&name, SavingEntry::Directory)?;
                }
                EntryType::File(buf, fop) => {
                    ps(ProgressState::Push(n, name.clone()))?;
                    n += 1;
                    match fop {
                        FileOp::Ignore(e) => {
                            ev.collect(name.clone(), e.into());
                        }
                        FileOp::Minify(m) => {
                            let buf: &[u8] = match m.minify(cfgmap, &buf, &mut cv) {
                                Ok(()) => &cv,
                                Err(e) => {
                                    ev.collect(name.clone(), e);
                                    &buf
                                }
                            };
                            self.save(&name, SavingEntry::File(buf, m.compress_min()))?;
                            cv.clear();
                        }
                        FileOp::Recompress(x) => {
                            self.save(&name, SavingEntry::File(&buf, x as u16))?;
                        }
                        FileOp::Pass => {
                            self.save(&name, SavingEntry::File(&buf, 24))?;
                        }
                    }
                }
            }
        }
        ps(ProgressState::Finish)
    }
}

/// An entry with its name and type.
pub struct NamedEntry(pub Arc<str>, pub EntryType);
impl NamedEntry {
    /// A shorthand function for creating a directory entry
    #[inline]
    pub fn dir(name: impl Into<Arc<str>>) -> Self {
        Self(name.into(), EntryType::Directory)
    }

    /// A shorthand function for creating a file entry
    #[inline]
    pub fn file(name: impl Into<Arc<str>>, data: impl Into<Bytes>, fop: FileOp) -> Self {
        Self(name.into(), EntryType::File(data.into(), fop))
    }
}

/// An entry type based on extracted data from an archive
#[derive(Clone)]
pub enum EntryType {
    /// A directory with its path
    Directory,
    /// A file with its path, data and file operation
    File(Bytes, FileOp)
}

/// A type for saving entries.
pub enum SavingEntry<'a> {
    /// A directory
    Directory,
    /// A file with data and a minimum compression constraint
    File(&'a [u8], u16)
}