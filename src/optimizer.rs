use std::{fs::File, io::{self, Read, BufReader, BufWriter}, error::Error, fmt, thread, path::PathBuf};

use indicatif::ProgressBar;
use zip::{ZipArchive, write::FileOptions, ZipWriter};
use crossbeam_channel::{bounded, Sender, Receiver};

use crate::{minify::{only_recompress, MinifyType}, blacklist, fop::{FileOp, pack_file}, errors::ErrorCollector};

/// Optimizes an archive and saves repacked one in a new destination.
pub fn optimize_archive(
    in_path: PathBuf,
    out_path: PathBuf,
    pb: ProgressBar,
    errors: &mut dyn ErrorCollector,
    file_opts: &FileOptions,
    use_blacklist: bool
) -> io::Result<i64> {
    let (tx, rx) = bounded(2);
    let t1 = thread::spawn(move || read_archive_entries(in_path, tx, use_blacklist));
    let rsum = save_archive_entries(out_path, rx, file_opts, errors, pb);
    t1.join().unwrap()?;
    rsum
}

fn save_archive_entries(out_path: PathBuf, rx: Receiver<EntryType>, file_opts: &FileOptions, ev: &mut dyn ErrorCollector, pb: ProgressBar) -> io::Result<i64> {
    let fout = File::create(out_path)?;
    let mut dsum = 0;
    let mut zw = ZipWriter::new(BufWriter::new(fout));
    let mut cnt = 0;
    let mut cv = Vec::new();
    for et in rx {
        match et {
            EntryType::Count(u) => {
                pb.set_length(u);
            }
            EntryType::Directory(d) => {
                if d != ".cache/" {
                    zw.add_directory(d, file_opts.clone())?;
                }
            }
            EntryType::File(fname, buf, fop) => {
                use FileOp::*;
                cnt += 1;
                pb.set_position(cnt);
                pb.set_message(fname.clone());
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
    pb.finish_with_message("Saving...");
    zw.finish()?;
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

fn read_archive_entries(in_path: PathBuf, tx: Sender<EntryType>, use_blacklist: bool) -> io::Result<()> {
    let fin = File::open(in_path)?;
    let mut za = ZipArchive::new(BufReader::new(fin))?;
    let jfc = za.len() as u64;
    tx.send(EntryType::Count(jfc)).unwrap();
    for i in 0..jfc {
        let mut jf = za.by_index(i as usize)?;
        let fname = jf.name().to_string();
        tx.send(if fname.ends_with('/') {
            EntryType::Directory(fname)
        } else {
            let mut obuf = Vec::new();
            obuf.reserve_exact(jf.size() as usize);
            jf.read_to_end(&mut obuf)?;
            let fop = check_file_by_name(&fname, use_blacklist);
            EntryType::File(fname, obuf, fop)
        }).unwrap()
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