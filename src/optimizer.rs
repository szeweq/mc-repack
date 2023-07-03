use std::{fs::File, io::{self}, error::Error, fmt, thread, path::{PathBuf}, any::Any};

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
) -> io::Result<()> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        reader.read_entries(tx, use_blacklist)
    });
    saver.save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
}

/// Optimizes an archive and saves repacked one in a new destination.
pub fn optimize_archive(
    in_path: PathBuf,
    out_path: PathBuf,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    file_opts: &FileOptions,
    use_blacklist: bool
) -> io::Result<()> {
    use entry::zip::*;
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        let fin = File::open(in_path)?;
        ZipEntryReader::new(fin).read_entries(tx, use_blacklist)
    });
    let fout = File::create(out_path)?;
    ZipEntrySaver::custom(fout, *file_opts).save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
}

/// Optimizes files in directory and saves them in a new destination.
pub fn optimize_fs_copy(
    in_path: PathBuf,
    out_path: PathBuf,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> io::Result<()> {
    use entry::fs::*;
    if in_path == out_path {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "The paths are the same"))
    }
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        FSEntryReader::new(in_path).read_entries(tx, use_blacklist)
    });
    FSEntrySaver::new(out_path).save_entries(rx, errors, ps)?;
    t1.join().map_err(JOIN_ERR)?
}

/// An entry type based on extracted data from an archive
pub enum EntryType {
    /// Number of files stored in an archive
    Count(u64),
    /// A directory with its path
    Directory(String),
    /// A file with its path, data and file operation
    File(String, Vec<u8>, FileOp)
}

#[derive(Debug)]
pub(crate) struct StrError(pub String);
impl Error for StrError {}
impl fmt::Display for StrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

pub(crate) const ERR_SIGNFILE: &str = "This file cannot be repacked since it contains SHA-256 digests for zipped entries";

/// A progress state to update information about currently optimized entry
#[derive(Debug, Clone)]
pub enum ProgressState {
    /// Starts a progress with a step count
    Start(u64),
    /// Pushes a new step with text
    Push(u64, String),
    /// Marks a progress as finished
    Finish
}