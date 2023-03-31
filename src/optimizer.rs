use std::{fs::File, io::{self, Read, Seek, Write}, error::Error, fmt, thread, path::{PathBuf, Path}};

use zip::write::FileOptions;
use crossbeam_channel::{bounded, Sender, Receiver};

use crate::{minify::{only_recompress, MinifyType}, blacklist, fop::FileOp, errors::ErrorCollector, entry::{self, EntryReader, EntrySaver}};

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

/// Writes optimized ZIP entries into a specified writer.
#[deprecated = "Use `entry::zip::ZipEntrySaver`"]
pub fn save_archive_entries<W: Write + Seek>(
    w: W,
    rx: Receiver<EntryType>,
    file_opts: &FileOptions,
    ev: &mut dyn ErrorCollector,
    ps: &Sender<ProgressState>
) -> io::Result<i64> {
    entry::zip::ZipEntrySaver::custom(w, file_opts.clone()).save_entries(rx, ev, ps)
}

pub(crate) fn check_file_by_name(fname: &str, use_blacklist: bool) -> FileOp {
    use FileOp::*;
    if fname.starts_with(".cache/") { return Ignore }
    if fname.starts_with("META-INF/") {
        let sub = &fname[9..];
        match sub {
            "MANIFEST.MF" => {return Recompress(64) }
            "SIGNFILE.SF" | "SIGNFILE.DSA" => { return Signfile }
            x if x.starts_with("SIG-") || [".DSA", ".RSA", ".SF"].into_iter().any(|e| x.ends_with(e)) => {
                return Signfile
            }
            x if x.starts_with("services/") => { return Recompress(64) }
            _ => {}
        }
    }
    let ftype = fname.rsplit_once('.').unzip().1.unwrap_or("");
    if ftype == "class" {
        return Recompress(64)
    }
    if only_recompress(ftype) {
        return Recompress(4)
    }
    if let Some(x) = MinifyType::by_extension(ftype) {
        return Minify(x)
    }
    if use_blacklist && blacklist::can_ignore_type(ftype) { Ignore } else { Recompress(2) }
}

/// Reads ZIP entries and sends data using a channel.
#[deprecated = "Use `entry::zip::ZipEntryReader`"]
pub fn read_archive_entries<R: Read + Seek>(
    r: R,
    tx: Sender<EntryType>,
    use_blacklist: bool
) -> io::Result<()> {
    entry::zip::ZipEntryReader::new(r).read_entries(tx, use_blacklist)
}

/// Writes optimized file system entries into a specified destination directory.
#[deprecated = "Use `entry::fs::FSEntrySaver`"]
pub fn save_fs_entries(
    dest_dir: &Path,
    rx: Receiver<EntryType>,
    ev: &mut dyn ErrorCollector,
    ps: &Sender<ProgressState>
) -> io::Result<i64> {
    entry::fs::FSEntrySaver::new(dest_dir.to_owned()).save_entries(rx, ev, ps)
}

/// Reads file system entries from a source directory and sends data using a channel.
#[deprecated = "Use `entry::fs::FSEntryReader`"]
pub fn read_fs_entries(
    src_dir: &Path,
    tx: Sender<EntryType>,
    use_blacklist: bool
) -> io::Result<()> {
    entry::fs::FSEntryReader::new(src_dir.to_owned()).read_entries(tx, use_blacklist)
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