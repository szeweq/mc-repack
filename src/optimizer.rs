use std::{fs::{File, self}, io::{self, Read, BufReader, BufWriter, Seek, Write}, error::Error, fmt, thread, path::{PathBuf, Path}};

use zip::{ZipArchive, write::FileOptions, ZipWriter};
use crossbeam_channel::{bounded, Sender, Receiver};

use crate::{minify::{only_recompress, MinifyType}, blacklist, fop::{FileOp, pack_file}, errors::ErrorCollector};

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
        read_archive_entries(fin, tx, use_blacklist)
    });
    let fout = File::create(out_path)?;
    let rsum = save_archive_entries(fout, rx, file_opts, errors, ps);
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
        read_fs_entries(&in_path, tx, use_blacklist)
    });
    let rsum = save_fs_entries(&out_path, rx, errors, ps);
    t1.join().unwrap()?;
    rsum
}

/// Writes optimized ZIP entries into a specified writer.
pub fn save_archive_entries<W: Write + Seek>(
    w: W,
    rx: Receiver<EntryType>,
    file_opts: &FileOptions,
    ev: &mut dyn ErrorCollector,
    ps: &Sender<ProgressState>
) -> io::Result<i64> {
    let mut dsum = 0;
    let mut zw = ZipWriter::new(BufWriter::new(w));
    let mut cv = Vec::new();
    for et in rx {
        match et {
            EntryType::Count(u) => {
                ps.send(ProgressState::Start(u)).unwrap();
            }
            EntryType::Directory(d) => {
                if d != ".cache/" {
                    zw.add_directory(d, file_opts.clone())?;
                }
            }
            EntryType::File(fname, buf, fop) => {
                use FileOp::*;
                ps.send(ProgressState::Push(fname.clone())).unwrap();
                match fop {
                    Recompress(cmin) => {
                        pack_file(
                            &mut zw,
                            &fname,
                            file_opts,
                            &buf,
                            cmin
                        )?;
                    }
                    Minify(m) => {
                        let fsz = buf.len() as i64;
                        let buf = match m.minify(&buf, &mut cv) {
                            Ok(()) => &cv,
                            Err(e) => {
                                ev.collect(fname.to_string(), e);
                                &buf
                            }
                        };
                        dsum -= (buf.len() as i64) - fsz;
                        pack_file(&mut zw, &fname, file_opts, &buf, m.compress_min())?;
                        cv.clear();
                    }
                    Ignore => {
                        ev.collect(fname.to_string(), Box::new(blacklist::BlacklistedFile));
                    }
                    Warn(x) => {
                        ev.collect(fname.to_string(), Box::new(StrError(x)));
                        pack_file(&mut zw, &fname, file_opts, &buf, 0)?;
                    }
                    Signfile => {
                        ev.collect(fname.to_string(), Box::new(StrError(ERR_SIGNFILE.to_string())));
                    }
                }
            }
        }
    }
    zw.finish()?;
    ps.send(ProgressState::Finish).unwrap();
    Ok(dsum)
}

fn check_file_by_name(fname: &str, use_blacklist: bool) -> FileOp {
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
pub fn read_archive_entries<R: Read + Seek>(
    r: R,
    tx: Sender<EntryType>,
    use_blacklist: bool
) -> io::Result<()> {
    let mut za = ZipArchive::new(BufReader::new(r))?;
    let jfc = za.len() as u64;
    tx.send(EntryType::Count(jfc)).unwrap();
    for i in 0..jfc {
        let mut jf = za.by_index(i as usize)?;
        let fname = jf.name().to_string();
        tx.send(if fname.ends_with('/') {
            EntryType::Directory(fname)
        } else {
            let fop = check_file_by_name(&fname, use_blacklist);
            let mut obuf = Vec::new();
            match fop {
                FileOp::Ignore => {}
                _ => {
                    obuf.reserve_exact(jf.size() as usize);
                    jf.read_to_end(&mut obuf)?;
                }
            }
            EntryType::File(fname, obuf, fop)
        }).unwrap();
    }
    Ok(())
}

/// Writes optimized file system entries into a specified destination directory.
pub fn save_fs_entries(
    dest_dir: &Path,
    rx: Receiver<EntryType>,
    ev: &mut dyn ErrorCollector,
    ps: &Sender<ProgressState>
) -> io::Result<i64> {
    let mut dsum = 0;
    let mut cv = Vec::new();
    for et in rx {
        match et {
            EntryType::Count(u) => {
                ps.send(ProgressState::Start(u)).unwrap();
            }
            EntryType::Directory(dir) => {
                let mut dp = PathBuf::from(dest_dir);
                dp.push(dir);
                fs::create_dir(dp)?;
            }
            EntryType::File(fname, buf, fop) => {
                use FileOp::*;
                ps.send(ProgressState::Push(fname.clone())).unwrap();
                let mut fp = PathBuf::from(dest_dir);
                fp.push(fname.clone());
                match fop {
                    Recompress(_) => {
                        // Write in file system as-is
                        fs::write(fp, buf)?;
                    }
                    Minify(m) => {
                        let fsz = buf.len() as i64;
                        let buf = match m.minify(&buf, &mut cv) {
                            Ok(()) => &cv,
                            Err(e) => {
                                ev.collect(fname.to_string(), e);
                                &buf
                            }
                        };
                        dsum -= (buf.len() as i64) - fsz;
                        fs::write(fp, buf)?;
                        cv.clear();
                    }
                    Ignore => {
                        ev.collect(fname, Box::new(blacklist::BlacklistedFile));
                    }
                    Warn(x) => {
                        ev.collect(fname, Box::new(StrError(x)));
                        fs::write(fp, buf)?;
                    }
                    Signfile => {
                        ev.collect(fname, Box::new(StrError(ERR_SIGNFILE.to_string())));
                    }
                }
            }
        }
    }
    ps.send(ProgressState::Finish).unwrap();
    Ok(dsum)
}

/// Read file system entries from a source directory and sends data using a channel.
pub fn read_fs_entries(
    src_dir: &Path,
    tx: Sender<EntryType>,
    use_blacklist: bool
) -> io::Result<()> {
    let mut vdir = Vec::new();
    vdir.push(src_dir.to_owned());
    while let Some(px) = vdir.pop() {
        let rd = fs::read_dir(px)?.collect::<Result<Vec<_>, _>>()?;
        tx.send(EntryType::Count(rd.len() as u64)).unwrap();
        for de in rd {
            let meta = de.metadata()?;
            if meta.is_dir() {
                let dp = de.path();
                let dname = dp.strip_prefix(src_dir).expect("Subdir not in source dir")
                    .to_string_lossy().to_string();
                vdir.push(dp);
                tx.send(EntryType::Directory(dname)).unwrap();
            } else if meta.is_file() {
                let fp = de.path();
                let fname = fp.strip_prefix(src_dir).expect("File not in source dir")
                    .to_string_lossy().to_string();
                let fop = check_file_by_name(&fname, use_blacklist);
                let ff = fs::read(fp)?;
                tx.send(EntryType::File(fname, ff, fop)).unwrap();
            }
        }
    }
    Ok(())
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

const ERR_SIGNFILE: &str = "This file cannot be repacked since it contains SHA-256 digests for zipped entries";

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