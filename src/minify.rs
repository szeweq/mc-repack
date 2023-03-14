use std::{io::{Cursor, BufRead}, collections::HashMap, error::Error};

use json_comments::StripComments;
use serde_json::Value;

const BOM_BYTES: [u8; 3] = [239, 187, 191];
fn strip_bom(b: &[u8]) -> &[u8] {
    if b.len() >= 3 && b[..3] == BOM_BYTES { &b[3..] } else { b }
}

fn find_brackets(b: &[u8]) -> Result<(usize, usize), Box<dyn Error>> {
    let (i, endb) = match b.iter().enumerate().find(|(_, &b)| b == b'{' || b == b'[') {
        Some((i, b'{')) => (i, b'}'),
        Some((i, b'[')) => (i, b']'),
        _ => { return Err(JSONMinifierError)?; }
    };
    let Some(j) = b.iter().rposition(|&b| b == endb) else {
        return Err(JSONMinifierError)?;
    };
    Ok((i, j))
}

const DUMMIES: &[&str] = &["fsh", "glsl", "html", "js", "kotlin_module", "md", "nbt", "ogg", "txt", "vert", "vsh", "xml"];

pub fn only_recompress(ftype: &str) -> bool {
    DUMMIES.binary_search(&ftype).is_ok()
}

pub fn all_minifiers() -> HashMap<&'static str, Box<dyn Minifier>> {
    let mut popts = oxipng::Options::default();
    popts.fix_errors = true;
    popts.strip = oxipng::Headers::Safe;
    popts.optimize_alpha = true;
    popts.deflate = oxipng::Deflaters::Libdeflater { compression: 12 };

    let mut minif: HashMap<&str, Box<dyn Minifier>> = HashMap::new();
    minif.insert("png", Box::new(PNGMinifier { opts: popts }));
    minif.insert("json", Box::new(JSONMinifier));
    minif.insert("mcmeta", Box::new(JSONMinifier));
    minif.insert("toml", Box::new(TOMLMinifier));
    minif.insert("cfg", Box::new(HashCommentRemover));
    minif.insert("obj", Box::new(HashCommentRemover));
    minif.insert("mtl", Box::new(HashCommentRemover));
    minif
}

pub type ResultBytes = Result<Vec<u8>, Box<dyn Error>>;

pub trait Minifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes;
    fn compress_min(&self) -> usize;
}

#[derive(Debug)]
pub struct JSONMinifierError;
impl Error for JSONMinifierError {}
impl std::fmt::Display for JSONMinifierError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "The file has improper opening and/or closing brackets")
    }
}

pub struct JSONMinifier;
impl Minifier for JSONMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let mut fv = strip_bom(v);
        let (i, j) = find_brackets(fv)?;
        fv = &fv[i..j+1];
        let strip_comments = StripComments::new(Cursor::new(fv));
        let mut sv: Value = serde_json::from_reader(strip_comments)?;
        if let Value::Object(xm) = &mut sv {
            uncomment_json_recursive(xm)
        }
        Ok(serde_json::to_vec(&sv)?)
    }
    fn compress_min(&self) -> usize { 48 }
}
fn uncomment_json_recursive(m: &mut serde_json::Map<String, Value>) {
    m.retain(|k, _| !k.starts_with('_'));
    m.values_mut().for_each(|v| {
        if let Value::Object(xm) = v {
            uncomment_json_recursive(xm);
        }
    });
}

pub struct PNGMinifier {
    pub opts: oxipng::Options
}
impl Minifier for PNGMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        Ok(oxipng::optimize_from_memory(&v, &self.opts)?)
    }
    fn compress_min(&self) -> usize { 512 }
}

pub struct TOMLMinifier;
impl Minifier for TOMLMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let fv = std::str::from_utf8(strip_bom(v))?;
        let table: toml::Table = toml::from_str(fv)?;
        Ok(toml::to_string(&table)?
            .lines()
            .map(|l| l.replacen(" = ", "=", 1).into_bytes())
            .collect::<Vec<_>>().join(&b'\n'))
    }
    fn compress_min(&self) -> usize { 48 }
}

pub struct HashCommentRemover;
impl Minifier for HashCommentRemover {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let mut buf = Vec::new();
        for l in v.lines() {
            let l = l?;
            if !(l.is_empty() || l.starts_with('#')) {
                buf.extend(l.bytes());
                buf.push(b'\n');
            }
        }
        Ok(buf)
    }
    fn compress_min(&self) -> usize { 4 }
}