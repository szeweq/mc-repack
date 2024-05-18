use std::{fs, path::Path};
use crossbeam_channel::Sender;
use crate::fop::FileOp;
use super::{EntryReader, EntrySaver, EntrySaverSpec, EntryType};

/// An entry reader implementation for a file system. It reads a file tree from a provided directory.
pub struct FSEntryReader {
    src_dir: Box<Path>
}
impl FSEntryReader {
    /// Creates an entry reader with a source directory path.
    pub const fn new(src_dir: Box<Path>) -> Self {
        Self { src_dir }
    }
}
impl EntryReader for FSEntryReader {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> crate::Result_<()> {
        let mut vdir = vec![self.src_dir.clone()];
        while let Some(px) = vdir.pop() {
            let rd = fs::read_dir(px)?.collect::<Result<Vec<_>, _>>()?;
            super::wrap_send(&tx, EntryType::Count(rd.len()))?;
            for de in rd {
                let meta = de.metadata()?;
                let et = if meta.is_dir() {
                    let dp = de.path();
                    let dname = if let Ok(d) = dp.strip_prefix(&self.src_dir) {
                        d.to_string_lossy().to_string()
                    } else {
                        continue
                    };
                    vdir.push(dp.into_boxed_path());
                    EntryType::dir(dname)
                } else if meta.is_file() {
                    let fp = de.path();
                    let fname = if let Ok(d) = fp.strip_prefix(&self.src_dir) {
                        d.to_string_lossy().to_string()
                    } else {
                        continue
                    };
                    let fop = FileOp::by_name(&fname, use_blacklist);
                    let ff = fs::read(&fp)?;
                    EntryType::file(fname, ff, fop)
                } else {
                    continue
                };
                super::wrap_send(&tx, et)?;
            }
        }
        Ok(())
    }
}

/// An entry saver implementation for a file system. It writes files into a provided directory.
pub struct FSEntrySaver {
    dest_dir: Box<Path>
}
impl FSEntrySaver {
    /// Creates an entry saver with a destination directory path.
    pub const fn new(dest_dir: Box<Path>) -> EntrySaver<Self> {
        EntrySaver(Self { dest_dir })
    }
}
impl EntrySaverSpec for FSEntrySaver {
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()> {
        let mut dp = self.dest_dir.to_path_buf();
        dp.push(dir);
        fs::create_dir(dp)?;
        Ok(())
    }
    fn save_file(&mut self, fname: &str, buf: &[u8], _: u16) -> crate::Result_<()> {
        let mut fp = self.dest_dir.to_path_buf();
        fp.push(fname);
        fs::write(fp, buf)?;
        Ok(())
    }
}

// #[inline]
// fn send_err(e: SendError<EntryType>) -> io::Error {
//     io::Error::new(io::ErrorKind::Other, e)
// }