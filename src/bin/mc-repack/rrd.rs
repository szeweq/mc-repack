use std::{fs, path::Path};


/// A literal copy of [`mc_repack_core::entry::fs::RecursiveReadDir`]
pub struct RecursiveReadDir {
    dirs: Vec<Box<Path>>,
    cur: Option<Box<fs::ReadDir>>
}
impl RecursiveReadDir {
    pub fn new(src_dir: Box<Path>) -> Self {
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
                        self.cur = Some(Box::new(rd));
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