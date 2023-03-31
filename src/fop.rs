use crate::minify::MinifyType;

pub(crate) const REPACKED: &str = "$repack";

/// A file type (not extension) that MC-Repack will check before repacking.
#[derive(PartialEq)]
pub enum FileType {
    /// Type for files which MC-Repack cannot repack.
    Other,
    /// A JAR or ZIP file which was not yet repacked.
    Original,
    /// A repacked file.
    Repacked
}

/// Checks file type based on its file name.
pub fn check_file_type(s: &str) -> FileType {
    use FileType::*;
    match s.rsplit_once('.') {
        Some((n, "jar" | "zip")) => if n.ends_with(REPACKED) { Repacked } else { Original }
        _ => Other
    }
}

/// A file operation needed before a file is saved in repacked archive
pub enum FileOp {
    /// Recompress data (check minimal size to determine if a file can be compressed or not).
    Recompress(usize),
    /// Minify a file.
    Minify(MinifyType),
    /// Ignore a file.
    Ignore,
    /// A "Signfile" was found.
    Signfile,
    /// Give a warning about a file.
    Warn(String)
}
