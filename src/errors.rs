use std::{error::Error, fmt::Display};

/// A struct for collecting errors.
pub struct ErrorCollector {
    silent: bool,
    vec: Vec<EntryRepackError>,
    name: Option<String>,
}
impl ErrorCollector {
    /// Creates a new `ErrorCollector` with a `silent` option.
    pub const fn new(silent: bool) -> Self { Self { silent, vec: Vec::new(), name: None } }

    /// Sets the new prefix name for collected entries. 
    pub fn rename(&mut self, name: &dyn Display)  {
        self.name = Some(name.to_string());
    }

    /// Collects errors for files based on their name (path).
    pub fn collect(&mut self, name: &dyn Display, e: Box<dyn Error>) {
        if !self.silent {
            self.vec.push(EntryRepackError {
                name: self.name.as_ref().map_or_else(|| name.to_string(), |n| format!("{n}/{name}")),
                inner: e
            })
        }
    }

    /// Returns all currently gathered results.
    pub fn results(&self) -> &[EntryRepackError] {
        &self.vec
    }
}

/// An error struct that wraps an inner error thrown while a file was processed. 
#[derive(Debug)]
pub struct EntryRepackError {
    name: String,
    inner: Box<dyn Error>
}
impl Error for EntryRepackError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&*self.inner)
    }
}
impl Display for EntryRepackError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.name)
    }
}
impl EntryRepackError {
    /// Borrows an associated file name from the error.
    pub fn name(&self) -> &str {
        &self.name
    }
}