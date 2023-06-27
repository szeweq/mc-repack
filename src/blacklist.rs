use std::{error::Error, fmt::Display};

/// This method checks if a file extension can be ignored (likely not used).
pub fn can_ignore_type(s: &str) -> bool {
    matches!(s, "blend" | "blend1" | "psd")
}

/// An error struct informing that processed file is blacklisted.
#[derive(Debug)]
pub struct BlacklistedFile;

impl Error for BlacklistedFile {}
impl Display for BlacklistedFile {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Blacklisted")
    }
}