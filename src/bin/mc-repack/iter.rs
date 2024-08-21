use std::{io, path::Path};


type FileResult = io::Result<(Option<bool>, Box<Path>)>;

pub enum Files {
    Single(Option<FileResult>),
    Dir(std::vec::IntoIter<FileResult>)
}
impl Files {
    pub fn from_path(p: &Path) -> io::Result<(Box<Path>, Self)> {
        let ft = p.metadata()?.file_type();
        let p: Box<Path> = Box::from(p);
        if ft.is_dir() {
            Ok((p.clone(), Self::Dir(walkdir::WalkDir::new(p).into_iter().map(|r| Ok(check_dir_entry(r?))).collect::<Vec<_>>().into_iter())))
        } else if ft.is_file() {
            let parent = p.parent().unwrap();
            Ok((parent.into(), Self::Single(Some(Ok((Some(false), p))))))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Not a file or directory"))
        }
    }
}
impl Iterator for Files {
    type Item = FileResult;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(it) => it.take(),
            Self::Dir(it) => it.next()
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Single(Some(_)) => (1, Some(1)),
            Self::Single(None) => (0, Some(0)),
            Self::Dir(it) => it.size_hint()
        }
    }
}
impl ExactSizeIterator for Files {
    fn len(&self) -> usize {
        match self {
            Self::Single(Some(_)) => 1,
            Self::Single(None) => 0,
            Self::Dir(it) => it.len()
        }
    }
}

fn check_dir_entry(de: walkdir::DirEntry) -> (Option<bool>, Box<Path>) {
    let ft = de.file_type();
    let p = de.into_path().into_boxed_path();
    (if ft.is_dir() {
        Some(true)
    } else if ft.is_file() {
        Some(false)
    } else {
        None
    }, p)
}