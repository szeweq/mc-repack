use std::error::Error;

use crate::{cfg, errors};

mod json;
#[cfg(feature = "png")] mod png;
#[cfg(feature = "toml")] mod toml;
#[cfg(feature = "nbt")] mod nbt;

#[inline]
const fn strip_bom(b: &[u8]) -> &[u8] {
    if let Some(([239, 187, 191], x)) = b.split_first_chunk() { x } else { b }
}

#[inline]
fn find_brackets(b: &[u8]) -> Option<(usize, usize)> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some((i, j))
}

/// Checks if a file can be recompressed (not minified) depending on its extension
#[must_use]
pub fn only_recompress(ftype: &str) -> bool {
    matches!(ftype, "glsl" | "html" | "kotlin_module" | "md" | "ogg" | "txt" | "vert" | "xml")
}


/// A type to determine a minifying method and minimum compress size for file data.
pub enum Minifier {
    /// A PNG minifier using `oxipng`.
    #[cfg(feature = "png")] PNG,
    /// A JSON minifier using `serde_json`.
    JSON,
    /// A TOML minifier using `toml`.
    #[cfg(feature = "toml")] TOML,
    /// A customized NBT minifier
    #[cfg(feature = "nbt")] NBT,
    /// A minifier that removes hash (`#`) comment lines (and empty lines)
    Hash,
    /// A minifier that removes double-slash (`//`) comment lines (and empty lines)
    Slash
}
impl Minifier {
    /// Return a Minifier based on file extension.
    #[must_use]
    pub fn by_extension(ftype: &str) -> Option<Self> {
        Some(match ftype {
            #[cfg(feature = "png")] "png" => Self::PNG,
            "json" | "mcmeta" => Self::JSON,
            #[cfg(feature = "toml")] "toml" => Self::TOML,
            #[cfg(feature = "nbt")] "nbt" => Self::NBT,
            "cfg" | "obj" | "mtl" => Self::Hash,
            "zs" | "js" | "fsh" | "vsh" => Self::Slash,
            _ => return None
        })
    }

    /// Minifies file data and writes the result in provided vec.
    /// # Errors
    /// Returns an error if minifying fails, depending on file type
    pub fn minify(&self, cfgmap: &cfg::ConfigMap, v: &[u8], vout: &mut Vec<u8>) -> Result_ {
        match self {
            #[cfg(feature = "png")] Self::PNG => cfgmap.fetch::<png::MinifierPNG>().minify(v, vout),
            Self::JSON => cfgmap.fetch::<json::MinifierJSON>().minify(strip_bom(v), vout),
            #[cfg(feature = "toml")] Self::TOML => cfgmap.fetch::<toml::MinifierTOML>().minify(strip_bom(v), vout),
            #[cfg(feature = "nbt")] Self::NBT => cfgmap.fetch::<nbt::MinifierNBT>().minify(v, vout),
            Self::Hash => remove_line_comments(b"#", v, vout),
            Self::Slash => remove_line_comments(b"//", v, vout)
        }
    }

    /// Define a minimal size for file compression. Files with lower sizes will be stored as-is.
    #[must_use]
    pub const fn compress_min(&self) -> u16 {
        match self {
            #[cfg(feature = "png")] Self::PNG => 512,
            Self::JSON => 64,
            #[cfg(feature = "toml")] Self::TOML => 64,
            #[cfg(feature = "nbt")] Self::NBT => 768,
            _ => 8
        }
    }
}

type Result_ = Result<(), errors::Error_>;

fn remove_line_comments(bs: &'static [u8], v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    std::str::from_utf8(v)?;
    for l in v.split(|&b| b == b'\n' || b == b'\r') {
        let Some(ix) = l.iter().position(|&b| !b.is_ascii_whitespace()) else {
            continue;
        };
        let l = &l[ix..];
        if !l.starts_with(bs) {
            vout.extend_from_slice(l);
            vout.push(b'\n');
        }
    }
    Ok(())
}

/// An error indicating that a file has mismatched pair of brackets
#[derive(Debug)]
pub struct BracketsError;
impl Error for BracketsError {}
impl std::fmt::Display for BracketsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("File has improper opening/closing brackets")
    }
}

fn strip_string(s: &mut String) {
    let Some(li) = s.bytes().position(|b| !b.is_ascii_whitespace()) else {
        return;
    };
    *s = s.split_off(li);
    let Some(ri) = s.bytes().rposition(|b| !b.is_ascii_whitespace()) else {
        return;
    };
    s.truncate(ri + 1);
}