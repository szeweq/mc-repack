use std::{fs, io, path::Path};
use super::{EntryReader, EntrySaver, EntrySaverSpec, ReadEntry};

type FSEntry = io::Result<(Option<bool>, Box<Path>)>;

/// An entry reader implementation for a file system. It reads a file tree from a provided directory.
pub struct FSEntryReader<I: ExactSizeIterator<Item = FSEntry>> {
    src_dir: Box<Path>,
    iter: I
}
impl FSEntryReader<std::vec::IntoIter<FSEntry>> {
    /// Creates an entry reader with a source directory path.
    pub fn new(src_dir: Box<Path>) -> Self {
        let files = walkdir::WalkDir::new(src_dir.clone()).into_iter().map(check_dir_entry).collect::<Vec<_>>();
        Self::from_vec(src_dir, files)
    }
    /// Creates an entry reader with a source directory path with a list of files.
    pub fn from_vec(src_dir: Box<Path>, files: Vec<FSEntry>) -> Self {
        Self { src_dir, iter: files.into_iter() }
    }
}
impl <I: ExactSizeIterator<Item = FSEntry>> FSEntryReader<I> {
    /// Creates an entry reader with a source directory path and a custom iterator.
    pub fn custom(src_dir: Box<Path>, iter: I) -> Self {
        Self { src_dir, iter }
    }
}
impl <I: ExactSizeIterator<Item = FSEntry>> EntryReader for FSEntryReader<I> {
    type RE<'a> = ReadFileEntry<'a> where Self: 'a;
    #[inline]
    fn read_len(&self) -> usize {
        self.iter.len()
    }
    #[inline]
    fn read_next(&mut self) -> Option<Self::RE<'_>> {
        self.iter.next().map(|cur| ReadFileEntry { src_dir: &self.src_dir, cur })
    }
}

/// A read entry of a file system.
pub struct ReadFileEntry<'a> {
    src_dir: &'a Path,
    cur: FSEntry
}
impl ReadEntry for ReadFileEntry<'_> {
    fn meta(&self) -> (Option<bool>, Box<str>) {
        let Ok((is_dir, p)) = &self.cur else { return (None, "".into()) };
        let lname = if let Ok(p) = p.strip_prefix(self.src_dir) {
            p.to_string_lossy().to_string()
        } else {
            return (None, "".into())
        };
        (*is_dir, lname.into_boxed_str())
    }
    fn data(self) -> crate::Result_<bytes::Bytes> {
        match self.cur {
            Ok((_, p)) => Ok(fs::read(&p)?.into()),
            Err(e) => Err(e.into()),
        }
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

fn check_dir_entry(de: walkdir::Result<walkdir::DirEntry>) -> FSEntry {
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