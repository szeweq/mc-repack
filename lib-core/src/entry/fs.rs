use std::{fs, io, path::Path};
use crate::fop::FileOp;
use super::{EntryReader, EntryReaderSpec, EntrySaver, EntrySaverSpec, EntryType};

/// An entry reader implementation for a file system. It reads a file tree from a provided directory.
pub struct FSEntryReader<I: ExactSizeIterator<Item = io::Result<(Option<bool>, Box<Path>)>>> {
    src_dir: Box<Path>,
    iter: std::iter::Peekable<I>
}
impl FSEntryReader<std::vec::IntoIter<io::Result<(Option<bool>, Box<Path>)>>> {
    /// Creates an entry reader with a source directory path.
    pub fn new(src_dir: Box<Path>) -> EntryReader<Self> {
        let files = walkdir::WalkDir::new(src_dir.clone()).into_iter().map(check_dir_entry).collect::<Vec<_>>();
        Self::from_vec(src_dir, files)
    }
    /// Creates an entry reader with a source directory path with a list of files.
    pub fn from_vec(src_dir: Box<Path>, files: Vec<io::Result<(Option<bool>, Box<Path>)>>) -> EntryReader<Self> {
        EntryReader(Self { src_dir, iter: files.into_iter().peekable() })
    }
}
impl <I: ExactSizeIterator<Item = io::Result<(Option<bool>, Box<Path>)>>> FSEntryReader<I> {
    /// Creates an entry reader with a source directory path and a custom iterator.
    pub fn custom(src_dir: Box<Path>, iter: I) -> EntryReader<Self> {
        EntryReader(Self { src_dir, iter: iter.peekable() })
    }
}
impl <I: ExactSizeIterator<Item = io::Result<(Option<bool>, Box<Path>)>>> EntryReaderSpec for FSEntryReader<I> {
    fn len(&self) -> usize {
        self.iter.len()
    }
    fn peek(&mut self) -> Option<(Option<bool>, Box<str>)> {
        self.iter.peek().map(|x| {
            let Ok((is_dir, p)) = x else { return (None, "".into()) };
            let lname = if let Ok(p) = p.strip_prefix(&self.src_dir) {
                p.to_string_lossy().to_string()
            } else {
                return (None, "".into())
            };
            (*is_dir, lname.into_boxed_str())
        })
    }
    fn skip(&mut self) {
        self.iter.next();
    }
    fn read(&mut self) -> crate::Result_<EntryType> {
        let Some(r) = self.iter.next() else {
            anyhow::bail!("No more entries");
        };
        let (is_dir, p) = r?;
        let lname = if let Ok(p) = p.strip_prefix(&self.src_dir) {
            p.to_string_lossy().to_string()
        } else {
            anyhow::bail!("Invalid entry path: {}", p.display());
        };
        let et = match is_dir {
            Some(true) => EntryType::dir(lname),
            Some(false) => {
                let ff = fs::read(&p)?;
                EntryType::file(lname, ff, FileOp::Pass)
            },
            None => anyhow::bail!("Invalid entry type: {}", p.display()),
        };
        Ok(et)
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

fn check_dir_entry(de: walkdir::Result<walkdir::DirEntry>) -> io::Result<(Option<bool>, Box<Path>)> {
    match de {
        Err(e) => Err(e.into()),
        Ok(de) => {
            let ft = de.file_type();
        let p = de.into_path().into_boxed_path();
        Ok((if ft.is_dir() {
            Some(true)
        } else if ft.is_file() {
            Some(false)
        } else {
            None
        }, p))
        }
    }
}