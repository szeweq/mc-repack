use std::{fs::File, io, path::Path, sync::Arc, thread};
use crossbeam_channel::{bounded, Sender};

use crate::{cfg, entry::{self, EntryReader, EntrySaver, EntrySaverSpec}, errors::ErrorCollector, fop::FileOp};

/// Optimizes entries using entry reader in saver in separate threads.
/// # Errors
/// Returns an error if any thread fails. [`EntrySaver`] will return an error first.
pub fn optimize_with<R: EntryReader + Send + 'static, S: EntrySaverSpec>(
    reader: R,
    saver: EntrySaver<S>,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || reader.read_entries(tx, use_blacklist));
    saver.save_entries(rx, errors, cfgmap, ps)?;
    crate::wrap_err(t1.join(), "thread join failed")?
}

/// Optimizes an archive and saves repacked one in a new destination.
/// # Errors
/// Returns an error if any thread fails. [`EntrySaver`] will return an error first.
#[inline]
#[deprecated(since = "0.20.0", note = "use [optimize_with] instead")]
pub fn optimize_archive(
    in_path: Box<Path>,
    out_path: Box<Path>,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    optimize_with(
        entry::ZipEntryReader::new(File::open(in_path)?),
        entry::ZipEntrySaver::new(File::create(out_path)?),
        cfgmap, ps, errors, use_blacklist
    )
}

/// Optimizes files in directory and saves them in a new destination.
/// # Errors
/// Returns an error if any thread fails. [`EntrySaver`] will return an error first.
#[inline]
#[deprecated(since = "0.20.0", note = "use [optimize_with] instead")]
pub fn optimize_fs_copy(
    in_path: Box<Path>,
    out_path: Box<Path>,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    if in_path == out_path {
        return same_paths_err()
    }
    optimize_with(
        entry::FSEntryReader::new(in_path),
        entry::FSEntrySaver::new(out_path),
        cfgmap, ps, errors, use_blacklist
    )
}

#[cfg(not(feature = "anyhow"))]
fn same_paths_err() -> crate::Result_<()> {
    Err(io::Error::new(io::ErrorKind::InvalidInput, "paths are the same"))
}
#[cfg(feature = "anyhow")]
fn same_paths_err() -> crate::Result_<()> {
    Err(anyhow::anyhow!("paths are the same"))
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

/// A progress state to update information about currently optimized entry
#[derive(Debug, Clone)]
pub enum ProgressState {
    /// Starts a progress with a step count
    Start(usize),
    /// Pushes a new step with text
    Push(usize, Arc<str>),
    /// Marks a progress as finished
    Finish
}