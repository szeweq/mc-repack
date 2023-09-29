use crate::{minify::{MinifyType, only_recompress}, errors::FileIgnoreError};

pub(crate) const REPACKED: &str = "_repack";

/// A file type (not extension) that MC-Repack will check before repacking.
#[derive(PartialEq, Eq)]
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
    Recompress(u32),
    /// Minify a file.
    Minify(MinifyType),
    /// Ignore a file and return an error.
    Ignore(FileIgnoreError),
}

impl FileOp {
    pub(crate) fn by_name(fname: &str, use_blacklist: bool) -> Self {
        use FileOp::*;
        if fname.starts_with(".cache/") { return Ignore(FileIgnoreError::Blacklisted) }
        if let Some(sub) =  fname.strip_prefix("META-INF/") {
            match sub {
                "MANIFEST.MF" => {return Recompress(64) }
                "SIGNFILE.SF" | "SIGNFILE.DSA" => { return Ignore(FileIgnoreError::Signfile) }
                x if x.starts_with("SIG-") || [".DSA", ".RSA", ".SF"].into_iter().any(|e| x.ends_with(e)) => {
                    return Ignore(FileIgnoreError::Signfile)
                }
                x if x.starts_with("services/") => { return Recompress(64) }
                _ => {}
            }
        }
        let ftype = fname.rsplit_once('.').unzip().1.unwrap_or("");
        if ftype == "class" {
            return Recompress(64)
        }
        if only_recompress(ftype) {
            return Recompress(4)
        }
        if let Some(x) = MinifyType::by_extension(ftype) {
            return Minify(x)
        }
        if use_blacklist && can_ignore_type(ftype) { Ignore(FileIgnoreError::Blacklisted) } else { Recompress(2) }
    }
}

fn can_ignore_type(s: &str) -> bool {
    matches!(s, "blend" | "blend1" | "psd")
}