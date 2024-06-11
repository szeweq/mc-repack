use std::collections::HashSet;

use crate::{errors::FileIgnoreError, ext, min::Minifier};

/// A file operation needed before a file is saved in repacked archive
pub enum FileOp {
    /// Pass a file (no operation needed).
    Pass,
    /// Recompress data (check minimal size to determine if a file can be compressed or not).
    Recompress(u8),
    /// Minify a file.
    Minify(Minifier),
    /// Ignore a file and return an error.
    Ignore(FileIgnoreError),
}

impl FileOp {
    pub(crate) fn by_name(fname: &str, blacklist: &TypeBlacklist) -> Self {
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
            return Self::Pass
        };
        if blacklist.can_ignore(ftype) {
            return Self::Ignore(FileIgnoreError::Blacklisted)
        }
        ext::KnownFmt::by_extension(ftype)
            .map_or(Self::Pass, |x| Minifier::by_file_format(x).map_or(Self::Pass, Self::Minify))
    }
}

/// A blacklist of file types to ignore.
/// It has built-in file types (if [`TypeBlacklist::Extend`] is used): `bak`, `blend`, `blend1`, `disabled`, `gitignore`, `gitkeep`, `lnk`, `old`, `pdn`, `psd`, `xcf`.
pub enum TypeBlacklist {
    /// Extend the blacklist. It uses a predefined list of file types that are not supposed to be repacked.
    Extend(Option<HashSet<Box<str>>>),
    /// Override the blacklist. You can define your own list of file types regardless of the predefined list.
    Override(Option<HashSet<Box<str>>>)
}
impl TypeBlacklist {
    fn can_ignore(&self, s: &str) -> bool {
        let inner = match self {
            Self::Extend(x) => {
                if matches!(s, "bak" | "blend" | "blend1" | "disabled" | "gitignore" | "gitkeep" | "lnk" | "old" | "pdn" | "psd" | "xcf") {
                    return true
                }
                x
            }
            Self::Override(x) => x
        };
        inner.as_ref().map_or(false, |x| x.contains(s))
    }
}