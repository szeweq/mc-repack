use std::{io::{Cursor, BufRead}, collections::HashMap, error::Error};

use json_comments::StripComments;

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

const DUMMIES: &[&str] = &["fsh", "glsl", "html", "js", "md", "nbt", "ogg", "txt", "vert", "vsh", "xml"];

pub fn all_minifiers() -> HashMap<&'static str, Box<dyn Minifier>> {
    let mut popts = oxipng::Options::default();
    popts.fix_errors = true;
    popts.strip = oxipng::Headers::Safe;

    let mut minif: HashMap<&str, Box<dyn Minifier>> = HashMap::new();
    minif.insert("png", Box::new(PNGMinifier { opts: popts }));
    minif.insert("json", Box::new(JSONMinifier));
    minif.insert("mcmeta", Box::new(JSONMinifier));
    minif.insert("toml", Box::new(TOMLMinifier));
    minif.insert("cfg", Box::new(HashCommentRemover));
    for dt in DUMMIES {
        minif.insert(dt, Box::new(DummyMinifier));
    }
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
        let sv: serde_json::Value = serde_json::from_reader(strip_comments)?;
        let buf = serde_json::to_vec(&sv)?;
        Ok(buf)
    }
    fn compress_min(&self) -> usize { 48 }
}

pub struct PNGMinifier {
    pub opts: oxipng::Options
}
impl Minifier for PNGMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let buf = oxipng::optimize_from_memory(&v, &self.opts)?;
        Ok(buf)
    }
    fn compress_min(&self) -> usize { 512 }
}

pub struct TOMLMinifier;
impl Minifier for TOMLMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let fv = std::str::from_utf8(strip_bom(v))?;
        let table: toml::Table = toml::from_str(fv)?;
        let buf = toml::to_string(&table)?
            .lines()
            .map(|l| l.replacen(" = ", "=", 1).into_bytes())
            .collect::<Vec<_>>().join(&b'\n');
        Ok(buf)
    }
    fn compress_min(&self) -> usize { 48 }
}

pub struct DummyMinifier;
impl Minifier for DummyMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let fv = strip_bom(v);
        Ok(fv.to_vec())
    }
    fn compress_min(&self) -> usize { 0 }
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
    fn compress_min(&self) -> usize { 0 }
}