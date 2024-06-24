use crate::{cfg, ext::KnownFmt};

/// Minifier for JSON files
pub mod json;

/// Optimizer for PNG files
pub mod png;

/// Minifier for TOML files
pub mod toml;

/// Optimizer for NBT files
pub mod nbt;

/// Minifier for OGG files
pub mod ogg;

/// Optimizer for JAR archives
pub mod jar;

#[inline]
const fn strip_bom(b: &[u8]) -> &[u8] {
    if let [239, 187, 191, x @ ..] = b { x } else { b }
}

#[inline]
fn brackets(b: &[u8]) -> Option<&[u8]> {
    let i = b.iter().position(|&b| b == b'{' || b == b'[')?;
    let endb = match b[i] {
        b'{' => b'}',
        b'[' => b']',
        _ => { return None; }
    };
    let j = b.iter().rposition(|&b| b == endb)?;
    Some(&b[i..=j])
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
    /// An OGG minifier using `optivorbis`.
    #[cfg(feature = "ogg")] OGG,
    /// A simple repacker for embedded JAR archives
    #[cfg(feature = "jar")] JAR,
    /// A minifier that removes hash (`#`) comment lines (and empty lines)
    Hash,
    /// A minifier that removes double-slash (`//`) comment lines (and empty lines)
    Slash,
    /// A simple Unix line checker
    UnixLine
}
impl Minifier {
    /// Return a Minifier based on file extension.
    #[must_use]
    pub fn by_extension(ftype: &str) -> Option<Self> {
        Some(match ftype {
            #[cfg(feature = "png")] "png" => Self::PNG,
            "json" | "mcmeta" => Self::JSON,
            #[cfg(feature = "toml")] "toml" => Self::TOML,
            #[cfg(feature = "nbt")] "nbt" | "blueprint" => Self::NBT,
            #[cfg(feature = "ogg")] "ogg" => Self::OGG,
            #[cfg(feature = "jar")] "jar" => Self::JAR,
            "cfg" | "obj" | "mtl" => Self::Hash,
            "zs" | "js" | "fsh" | "vsh" => Self::Slash,
            "mf" => Self::UnixLine,
            _ => return None
        })
    }

    /// Return a Minifier based on known (by this library) file format.
    pub const fn by_file_format(f: KnownFmt) -> Option<Self> {
        Some(match f {
            #[cfg(feature = "png")] KnownFmt::Png => Self::PNG,
            KnownFmt::Json => Self::JSON,
            #[cfg(feature = "toml")] KnownFmt::Toml => Self::TOML,
            #[cfg(feature = "nbt")] KnownFmt::Nbt => Self::NBT,
            #[cfg(feature = "ogg")] KnownFmt::Ogg => Self::OGG,
            #[cfg(feature = "jar")] KnownFmt::Jar => Self::JAR,
            KnownFmt::Cfg | KnownFmt::Obj | KnownFmt::Mtl => Self::Hash,
            KnownFmt::Fsh | KnownFmt::Vsh | KnownFmt::Js | KnownFmt::Zs => Self::Slash,
            KnownFmt::Mf => Self::UnixLine,
            _ => return None
        })
    }

    /// Minifies file data and writes the result in provided vec.
    /// # Errors
    /// Returns an error if minifying fails, depending on file type
    pub fn minify(&self, cfgmap: &cfg::ConfigMap, v: &[u8], vout: &mut Vec<u8>) -> Result_ {
        match self {
            #[cfg(feature = "png")] Self::PNG => cfgmap.fetch::<png::MinifierPNG>().minify(v, vout),
            Self::JSON => cfgmap.fetch::<json::MinifierJSON>().minify(v, vout),
            #[cfg(feature = "toml")] Self::TOML => cfgmap.fetch::<toml::MinifierTOML>().minify(strip_bom(v), vout),
            #[cfg(feature = "nbt")] Self::NBT => cfgmap.fetch::<nbt::MinifierNBT>().minify(v, vout),
            #[cfg(feature = "ogg")] Self::OGG => cfgmap.fetch::<ogg::MinifierOGG>().minify(v, vout),
            #[cfg(feature = "jar")] Self::JAR => cfgmap.fetch::<jar::MinifierJAR>().minify(v, vout),
            Self::Hash => remove_line_comments("#", v, vout),
            Self::Slash => remove_line_comments("//", v, vout),
            Self::UnixLine => unixify_lines(v, vout)
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
            _ => 24
        }
    }
}

type Result_ = anyhow::Result<()>;

fn remove_line_comments(bs: &'static str, v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let v = std::str::from_utf8(v)?;
    for l in v.lines() {
        let Some(ix) = l.as_bytes().iter().position(|&b| !b.is_ascii_whitespace()) else {
            continue;
        };
        let l = &l[ix..];
        let nix = l.find(bs);
        let l = match nix {
            Some(nix) if nix == ix => "",
            Some(nix) => &l[..nix],
            None => l
        };
        vout.extend_from_slice(l.trim_end().as_bytes());
        vout.push(b'\n');
    }
    Ok(())
}

fn unixify_lines(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let v = std::str::from_utf8(v)?;
    for l in v.lines() {
        vout.extend_from_slice(l.trim_end().as_bytes());
        vout.push(b'\n');
    }
    Ok(())
}

/// An error indicating that a file has mismatched pair of brackets
#[derive(Debug)]
pub struct BracketsError;
impl std::error::Error for BracketsError {}
impl std::fmt::Display for BracketsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("improper opening/closing brackets")
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