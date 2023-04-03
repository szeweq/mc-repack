use std::{path::PathBuf, fs};
use crossbeam_channel::{Receiver, Sender};
use crate::{optimizer::{EntryType, ProgressState, StrError, ERR_SIGNFILE}, fop::{FileOp, check_file_by_name}, blacklist};
use super::{EntryReader, EntrySaver};

/// An entry reader implementation for a file system. It reads a file tree from a provided directory.
pub struct FSEntryReader {
    src_dir: PathBuf
}
impl FSEntryReader {
    /// Creates an entry reader with a source directory path.
    pub fn new(src_dir: PathBuf) -> Self {
        Self { src_dir }
    }
}
impl EntryReader for FSEntryReader {
    fn read_entries(
        self,
        tx: Sender<crate::optimizer::EntryType>,
        use_blacklist: bool
    ) -> std::io::Result<()> {
        let mut vdir = Vec::new();
        vdir.push(self.src_dir.clone());
        while let Some(px) = vdir.pop() {
            let rd = fs::read_dir(px)?.collect::<Result<Vec<_>, _>>()?;
            tx.send(EntryType::Count(rd.len() as u64)).unwrap();
            for de in rd {
                let meta = de.metadata()?;
                if meta.is_dir() {
                    let dp = de.path();
                    let dname = dp.strip_prefix(&self.src_dir).expect("Subdir not in source dir")
                        .to_string_lossy().to_string();
                    vdir.push(dp);
                    tx.send(EntryType::Directory(dname)).unwrap();
                } else if meta.is_file() {
                    let fp = de.path();
                    let fname = fp.strip_prefix(&self.src_dir).expect("File not in source dir")
                        .to_string_lossy().to_string();
                    let fop = check_file_by_name(&fname, use_blacklist);
                    let ff = fs::read(fp)?;
                    tx.send(EntryType::File(fname, ff, fop)).unwrap();
                }
            }
        }
        Ok(())
    }
}

/// An entry saver implementation for a file system. It writes files into a provided directory.
pub struct FSEntrySaver {
    dest_dir: PathBuf
}
impl FSEntrySaver {
    /// Creates an entry saver with a destination directory path.
    pub fn new(dest_dir: PathBuf) -> Self {
        Self { dest_dir }
    }
}
impl EntrySaver for FSEntrySaver {
    fn save_entries(
        self,
        rx: Receiver<EntryType>,
        ev: &mut dyn crate::errors::ErrorCollector,
        ps: &Sender<crate::optimizer::ProgressState>
    ) -> std::io::Result<i64> {
        let mut dsum = 0;
        let mut cv = Vec::new();
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    ps.send(ProgressState::Start(u)).unwrap();
                }
                EntryType::Directory(dir) => {
                    let mut dp = self.dest_dir.clone();
                    dp.push(dir);
                    fs::create_dir(dp)?;
                }
                EntryType::File(fname, buf, fop) => {
                    use FileOp::*;
                    ps.send(ProgressState::Push(fname.clone())).unwrap();
                    let mut fp = self.dest_dir.clone();
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
                                    ev.collect(&fname, e);
                                    &buf
                                }
                            };
                            dsum -= (buf.len() as i64) - fsz;
                            fs::write(fp, buf)?;
                            cv.clear();
                        }
                        Ignore => {
                            ev.collect(&fname, Box::new(blacklist::BlacklistedFile));
                        }
                        Warn(x) => {
                            ev.collect(&fname, Box::new(StrError(x)));
                            fs::write(fp, buf)?;
                        }
                        Signfile => {
                            ev.collect(&fname, Box::new(StrError(ERR_SIGNFILE.to_string())));
                        }
                    }
                }
            }
        }
        ps.send(ProgressState::Finish).unwrap();
        Ok(dsum)
    }
}