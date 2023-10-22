use std::{error::Error, fmt::Display, sync::Arc};

#[cfg(not(feature = "anyhow"))]
pub(crate) type Error_ = Box<dyn Error>;
#[cfg(feature = "anyhow")]
pub(crate) type Error_ = anyhow::Error;

/// A struct for collecting errors.
pub struct ErrorCollector {
    silent: bool,
    vec: Vec<EntryRepackError>,
    name: Arc<str>,
}
impl ErrorCollector {
    /// Creates a new `ErrorCollector` with a `silent` option.
    #[must_use]
    pub fn new(silent: bool) -> Self { Self { silent, vec: Vec::new(), name: "".into() } }

    /// Sets the new prefix name for collected entries. 
    pub fn rename(&mut self, name: &str)  {
        self.name = name.into();
    }

    /// Collects errors for files based on their name (path).
    pub fn collect(&mut self, name: impl Into<Arc<str>>, e: Error_) {
        if !self.silent {
            self.vec.push(EntryRepackError {
                parent: self.name.clone(),
                name: name.into(),
                inner: e
            });
        }
    }

    /// Returns all currently gathered results.
    #[must_use]
    pub fn results(&self) -> &[EntryRepackError] {
        &self.vec
    }
}

/// An error struct that wraps an inner error thrown while a file was processed. 
#[derive(Debug)]
pub struct EntryRepackError {
    /// A parent path (directory or an archive).
    pub parent: Arc<str>,
    /// An associated file name from the error.
    pub name: Arc<str>,
    inner: Error_
}
impl EntryRepackError {
    /// Returns the inner error.
    #[must_use]
    pub fn inner_error(&self) -> &dyn Error {
        &*self.inner
    }
}
impl Error for EntryRepackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.inner)
    }
}
impl Display for EntryRepackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}: {}", self.parent, self.name, self.inner)
    }
}

/// An error indicating a reason why a file cannot be repacked
#[derive(Debug)]
pub enum FileIgnoreError {
    /// A processed file is blacklisted.
    Blacklisted,
    /// A processed file contains SHA-256 hashes of zipped entries
    Signfile
}

impl Error for FileIgnoreError {}
impl Display for FileIgnoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Blacklisted => "blacklisted",
            Self::Signfile => "signfile contains SHA-256 hashes of zipped entries",
        })
    }
}