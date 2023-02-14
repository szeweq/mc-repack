use std::{io::Cursor, collections::HashMap, error::Error};

use json_comments::StripComments;

const BOM_BYTES: [u8; 3] = [239, 187, 191];
fn strip_bom(b: &[u8]) -> &[u8] {
    if b.len() >= 3 && b[..3] == BOM_BYTES { &b[3..] } else { b }
}

pub fn all_minifiers() -> HashMap<&'static str, Box<dyn Minifier>> {
    let mut popts = oxipng::Options::default();
    popts.fix_errors = true;

    let mut minif: HashMap<&str, Box<dyn Minifier>> = HashMap::new();
    minif.insert("png", Box::new(PNGMinifier { opts: popts }));
    minif.insert("json", Box::new(JSONMinifier));
    minif.insert("mcmeta", Box::new(JSONMinifier));
    minif
}

pub type ResultBytes = Result<Vec<u8>, Box<dyn Error>>;

pub trait Minifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes;
    fn compress_min(&self) -> usize;
}

pub struct JSONMinifier;
impl Minifier for JSONMinifier {
    fn minify(&self, v: &Vec<u8>) -> ResultBytes {
        let fv = strip_bom(v);
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