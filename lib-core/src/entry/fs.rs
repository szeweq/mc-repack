use std::{path::Path, fs, io};
use crossbeam_channel::{Sender, SendError};
use crate::{optimizer::EntryType, fop::FileOp};
use super::{EntryReader, EntrySaver, EntrySaverSpec};

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
        const SEND_ERR: fn(SendError<EntryType>) -> io::Error = |e: SendError<EntryType>| {
            io::Error::new(io::ErrorKind::Other, e)
        };
        let mut vdir = Vec::new();
        vdir.push(self.src_dir.to_path_buf());
        while let Some(px) = vdir.pop() {
            let rd = fs::read_dir(px)?.collect::<Result<Vec<_>, _>>()?;
            tx.send(EntryType::Count(rd.len())).map_err(SEND_ERR)?;
            for de in rd {
                let meta = de.metadata()?;
                let et = if meta.is_dir() {
                    let dp = de.path();
                    let dname = if let Ok(d) = dp.strip_prefix(&self.src_dir) {
                        d.to_string_lossy().to_string()
                    } else {
                        continue
                    };
                    vdir.push(dp);
                    EntryType::Directory(dname.into())
                } else if meta.is_file() {
                    let fp = de.path();
                    let fname = if let Ok(d) = fp.strip_prefix(&self.src_dir) {
                        d.to_string_lossy().to_string()
                    } else {
                        continue
                    };
                    let fop = FileOp::by_name(&fname, use_blacklist);
                    let ff = fs::read(&fp)?;
                    EntryType::File(fname.into(), ff.into(), fop)
                } else {
                    continue
                };
                tx.send(et).map_err(SEND_ERR)?;
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