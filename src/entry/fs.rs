use std::{path::PathBuf, fs};
use crossbeam_channel::Sender;
use crate::{optimizer::EntryType, fop::FileOp};
use super::{EntryReader, EntrySaver, EntrySaverSpec};

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
                    let fop = FileOp::by_name(&fname, use_blacklist);
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
    pub fn new(dest_dir: PathBuf) -> EntrySaver<Self> {
        EntrySaver(Self { dest_dir })
    }
}
impl EntrySaverSpec for FSEntrySaver {
    fn save_dir(&mut self, dir: &str) -> std::io::Result<()> {
        let mut dp = self.dest_dir.clone();
        dp.push(dir);
        fs::create_dir(dp)
    }
    fn save_file(&mut self, fname: &str, buf: &[u8], _: usize) -> std::io::Result<()> {
        let mut fp = self.dest_dir.clone();
        fp.push(fname);
        fs::write(fp, buf)
    }
}