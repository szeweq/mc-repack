use std::{fs::File, io::{self}, thread, path::Path, any::Any, sync::Arc};

use zip::write::FileOptions;
use crossbeam_channel::{bounded, Sender};

use crate::{fop::FileOp, errors::ErrorCollector, entry::{self, EntryReader, EntrySaver, EntrySaverSpec}};

const JOIN_ERR: fn(Box<dyn Any + Send>) -> io::Error = |_| {
    io::Error::new(io::ErrorKind::Other, "Thread join failed")
};

/// Optimizes entries using entry reader in saver in separate threads.
pub fn optimize_with<R: EntryReader + Send + 'static, S: EntrySaverSpec>(
    reader: R,
    saver: EntrySaver<S>,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        reader.read_entries(tx, use_blacklist)
    });
    saver.save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
}

/// Optimizes an archive and saves repacked one in a new destination.
pub fn optimize_archive(
    in_path: Box<Path>,
    out_path: Box<Path>,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        let fin = File::open(in_path)?;
        entry::zip::ZipEntryReader::new(fin).read_entries(tx, use_blacklist)
    });
    let fout = File::create(out_path)?;
    let file_opts = FileOptions::default().compression_level(Some(9));
    entry::zip::ZipEntrySaver::custom(fout, file_opts).save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
}

/// Optimizes files in directory and saves them in a new destination.
pub fn optimize_fs_copy(
    in_path: Box<Path>,
    out_path: Box<Path>,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    if in_path == out_path {
        return same_paths_err()
    }
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        entry::fs::FSEntryReader::new(in_path).read_entries(tx, use_blacklist)
    });
    entry::fs::FSEntrySaver::new(out_path).save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
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
    File(Arc<str>, Vec<u8>, FileOp)
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