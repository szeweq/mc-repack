use std::{fs, path::Path};
use crate::fop::FileOp;
use super::{EntryReaderSpec, EntrySaver, EntrySaverSpec, EntryType};

/// An entry reader implementation for a file system. It reads a file tree from a provided directory.
pub struct FSEntryReader {
    src_dir: Box<Path>,
    rrd: std::iter::Peekable<RecursiveReadDir>
}
impl FSEntryReader {
    /// Creates an entry reader with a source directory path.
    pub fn new(src_dir: Box<Path>) -> Self {
        let rrd = RecursiveReadDir::new(src_dir.clone()).peekable();
        Self { src_dir, rrd }
    }
}
impl EntryReaderSpec for FSEntryReader {
    fn len(&self) -> usize {
        RecursiveReadDir::new(self.src_dir.clone()).filter(|x| x.is_ok()).count()
    }
    fn peek(&mut self) -> Option<(Option<bool>, Box<str>)> {
        self.rrd.peek().map(|x| {
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
        self.rrd.next();
    }
    fn read(&mut self) -> crate::Result_<EntryType> {
        let Some(r) = self.rrd.next() else {
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

struct RecursiveReadDir {
    dirs: Vec<Box<Path>>,
    cur: Option<fs::ReadDir>
}
impl RecursiveReadDir {
    fn new(src_dir: Box<Path>) -> Self {
        Self { dirs: vec![src_dir], cur: None }
    }
}
impl Iterator for RecursiveReadDir {
    type Item = std::io::Result<(Option<bool>, Box<Path>)>;
    fn next(&mut self) -> Option<Self::Item> {
        let rd = match self.cur {
            None => {
                let p = self.dirs.pop()?;
                match fs::read_dir(p) {
                    Ok(rd) => {
                        self.cur = Some(rd);
                        self.cur.as_mut().unwrap()
                    },
                    Err(e) => return Some(Err(e))
                }
            }
            Some(ref mut rd) => rd
        };
        let e = match rd.next() {
            None => {
                self.cur = None;
                return self.next()
            }
            Some(Ok(x)) => {
                match x.file_type() {
                    Ok(ft) => {
                        let p = x.path().into_boxed_path();
                        return Some(Ok((if ft.is_dir() {
                            self.dirs.push(p.clone());
                            Some(true)
                        } else if ft.is_file() {
                            Some(false)
                        } else {
                            None
                        }, p)))
                    }
                    Err(e) => e
                }
            },
            Some(Err(e)) => e
        };
        Some(Err(e))
    }
}