use crate::{min::{Minifier, only_recompress}, errors::FileIgnoreError};

pub(crate) const REPACKED: &str = "_repack";

/// A file type (not extension) that MC-Repack will check before repacking.
pub enum FileType {
    /// Type for files which MC-Repack cannot repack.
    Other,
    /// A JAR or ZIP file which was not yet repacked.
    Original,
    /// A repacked file.
    Repacked
}
impl FileType {
    /// Returns file type based on its file name.
    #[must_use]
    pub fn by_name(s: &str) -> Self {
        match s.rsplit_once('.') {
            Some((n, "jar" | "zip")) => if n.ends_with(REPACKED) { Self::Repacked } else { Self::Original }
            _ => Self::Other
        }
    }
}

/// A file operation needed before a file is saved in repacked archive
pub enum FileOp {
    /// Recompress data (check minimal size to determine if a file can be compressed or not).
    Recompress(u32),
    /// Minify a file.
    Minify(Minifier),
    /// Ignore a file and return an error.
    Ignore(FileIgnoreError),
}

impl FileOp {
    pub(crate) fn by_name(fname: &str, use_blacklist: bool) -> Self {
        if fname.starts_with(".cache/") { return Self::Ignore(FileIgnoreError::Blacklisted) }
        if let Some(sub) =  fname.strip_prefix("META-INF/") {
            match sub {
                "MANIFEST.MF" => {return Self::Recompress(64) }
                "SIGNFILE.SF" | "SIGNFILE.DSA" => { return Self::Ignore(FileIgnoreError::Signfile) }
                x if x.starts_with("SIG-") || [".DSA", ".RSA", ".SF"].into_iter().any(|e| x.ends_with(e)) => {
                    return Self::Ignore(FileIgnoreError::Signfile)
                }
                x if x.starts_with("services/") => { return Self::Recompress(64) }
                _ => {}
            }
        }
        let Some((_, ftype)) = fname.rsplit_once('.') else {
            return Self::Recompress(2)
        };
        if ftype == "class" {
            return Self::Recompress(64)
        }
        if only_recompress(ftype) {
            return Self::Recompress(4)
        }
        if let Some(x) = Minifier::by_extension(ftype) {
            return Self::Minify(x)
        }
        if use_blacklist && can_ignore_type(ftype) { Self::Ignore(FileIgnoreError::Blacklisted) } else { Self::Recompress(2) }
    }
}

fn can_ignore_type(s: &str) -> bool {
    matches!(s, "blend" | "blend1" | "psd")
}