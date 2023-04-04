use std::{fs::File, io::{self}, error::Error, fmt, thread, path::{PathBuf}};

use zip::write::FileOptions;
use crossbeam_channel::{bounded, Sender};

use crate::{fop::FileOp, errors::ErrorCollector, entry::{self, EntryReader, EntrySaver}};

/// Optimizes entries using entry reader in saver in separate threads.
pub fn optimize_with<R: EntryReader + Send + 'static, S: EntrySaver>(
    reader: R,
    saver: S,
    ps: &Sender<ProgressState>,
    errors: &mut dyn ErrorCollector,
    use_blacklist: bool
) -> io::Result<i64> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        reader.read_entries(tx, use_blacklist)
    });
    let rsum = saver.save_entries(rx, errors, ps);
    t1.join().unwrap()?;
    rsum
}

/// Optimizes an archive and saves repacked one in a new destination.
pub fn optimize_archive(
    in_path: PathBuf,
    out_path: PathBuf,
    ps: &Sender<ProgressState>,
    errors: &mut dyn ErrorCollector,
    file_opts: &FileOptions,
    use_blacklist: bool
) -> io::Result<i64> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        let fin = File::open(in_path)?;
        entry::zip::ZipEntryReader::new(fin)
            .read_entries(tx, use_blacklist)
    });
    let fout = File::create(out_path)?;
    let rsum = entry::zip::ZipEntrySaver::custom(fout, file_opts.clone())
        .save_entries(rx, errors, ps);
    t1.join().unwrap()?;
    rsum
}

/// Optimizes files in directory and saves them in a new destination.
pub fn optimize_fs_copy(
    in_path: PathBuf,
    out_path: PathBuf,
    ps: &Sender<ProgressState>,
    errors: &mut dyn ErrorCollector,
    use_blacklist: bool
) -> io::Result<i64> {
    if in_path == out_path {
        return Err(io::Error::new(io::ErrorKind::InvalidInput, "The paths are the same"))
    }
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || {
        entry::fs::FSEntryReader::new(in_path)
            .read_entries(tx, use_blacklist)
    });
    let rsum = entry::fs::FSEntrySaver::new(out_path)
        .save_entries(rx, errors, ps);
    t1.join().unwrap()?;
    rsum
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
    Push(String),
    /// Marks a progress as finished
    Finish
}