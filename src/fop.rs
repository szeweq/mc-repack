use crate::minify::{MinifyType, only_recompress};

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
    Signfile
}

impl FileOp {
    pub(crate) fn by_name(fname: &str, use_blacklist: bool) -> Self {
        use FileOp::*;
        if fname.starts_with(".cache/") { return Ignore }
        if fname.starts_with("META-INF/") {
            let sub = &fname[9..];
            match sub {
                "MANIFEST.MF" => {return Recompress(64) }
                "SIGNFILE.SF" | "SIGNFILE.DSA" => { return Signfile }
                x if x.starts_with("SIG-") || [".DSA", ".RSA", ".SF"].into_iter().any(|e| x.ends_with(e)) => {
                    return Signfile
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
        if use_blacklist && crate::blacklist::can_ignore_type(ftype) { Ignore } else { Recompress(2) }
    }
}
