use std::{io::Cursor, error::Error};

use json_comments::StripComments;
use serde_json::Value;

use crate::errors;

const BOM_BYTES: [u8; 3] = [239, 187, 191];
fn strip_bom(b: &[u8]) -> &[u8] {
    if b.len() >= 3 && b[..3] == BOM_BYTES { &b[3..] } else { b }
}

fn find_brackets(b: &[u8]) -> Result<(usize, usize), BracketsError> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return Err(BracketsError); }
    };
    let Some(j) = b.iter().rposition(|&b| b == endb) else {
        return Err(BracketsError);
    };
    Ok((i, j))
}

/// Checks if a file can be recompressed (not minified) depending on its extension
#[must_use]
pub fn only_recompress(ftype: &str) -> bool {
    matches!(ftype, "glsl" | "html" | "js" | "kotlin_module" | "md" | "nbt" | "ogg" | "txt" | "vert" | "xml")
}


/// A type to determine a minifying method and minimum compress size for file data.
pub enum Minifier {
    /// A PNG minifier using `oxipng`.
    #[cfg(feature = "png")] PNG,
    /// A JSON minifier using `serde_json`.
    JSON,
    /// A TOML minifier using `toml`.
    TOML,
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
            "toml" => Self::TOML,
            "cfg" | "obj" | "mtl" => Self::Hash,
            "zs" | "fsh" | "vsh" => Self::Slash,
            _ => return None
        })
    }

    /// Minifies file data and writes the result in provided vec.
    /// # Errors
    /// Returns an error if minifying fails, depending on file type
    pub fn minify(&self, v: &[u8], vout: &mut Vec<u8>) -> Result_ {
        match self {
            #[cfg(feature = "png")] Self::PNG => minify_png(v, vout),
            Self::JSON => minify_json(v, vout),
            Self::TOML => minify_toml(v, vout),
            Self::Hash => remove_line_comments(b"#", v, vout),
            Self::Slash => remove_line_comments(b"//", v, vout)
        }
    }

    /// Define a minimal size for file compression. Files with lower sizes will be stored as-is.
    #[must_use]
    pub const fn compress_min(&self) -> u32 {
        match self {
            #[cfg(feature = "png")] Self::PNG => 512,
            Self::JSON | Self::TOML => 48,
            _ => 4
        }
    }
}

type Result_ = Result<(), errors::Error_>;

#[cfg(feature = "png")] 
fn minify_png(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let mut popts = oxipng::Options {
        fix_errors: true,
        strip: oxipng::StripChunks::Safe,
        optimize_alpha: true,
        deflate: oxipng::Deflaters::Libdeflater { compression: 12 },
        ..Default::default()
    };
    popts.filter.insert(oxipng::RowFilter::Up);
    popts.filter.insert(oxipng::RowFilter::Paeth);
    //popts.filter.insert(oxipng::RowFilter::MinSum);

    let v = oxipng::optimize_from_memory(v, &popts)?;
    let _ = std::mem::replace(vout, v);
    Ok(())
}

fn minify_json(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let mut fv = strip_bom(v);
    let (i, j) = find_brackets(fv)?;
    fv = &fv[i..=j];
    let strip_comments = StripComments::new(Cursor::new(fv));
    let mut sv: Value = serde_json::from_reader(strip_comments)?;
    if let Value::Object(xm) = &mut sv {
        uncomment_json_recursive(xm);
    }
    serde_json::to_writer(vout, &sv)?;
    Ok(())
}

fn minify_toml(v: &[u8], vout: &mut Vec<u8>) -> Result_ {
    let fv = std::str::from_utf8(strip_bom(v))?;
    let table: toml::Table = toml::from_str(fv)?;
    toml::to_string(&table)?.lines().for_each(|l| {
        vout.extend_from_slice(l.replacen(" = ", "=", 1).as_bytes());
        vout.push(b'\n');
    });
    Ok(())
}

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
        write!(f, "File has improper opening/closing brackets")
    }
}

fn uncomment_json_recursive(m: &mut serde_json::Map<String, Value>) {
    m.retain(|k, _| !k.starts_with('_'));
    for v in m.values_mut() {
        if let Value::Object(xm) = v {
            uncomment_json_recursive(xm);
        }
    }
}
