use std::{io::{self, Cursor}, collections::HashMap};

use json_comments::StripComments;

pub fn all_minifiers() -> HashMap<&'static str, Box<dyn Minifier>> {
    let mut popts = oxipng::Options::default();
    popts.fix_errors = true;

    let mut minif: HashMap<&str, Box<dyn Minifier>> = HashMap::new();
    minif.insert("png", Box::new(PNGMinifier { opts: popts }));
    minif.insert("json", Box::new(JSONMinifier));
    minif.insert("mcmeta", Box::new(JSONMinifier));
    minif
}

pub trait Minifier {
    fn minify(&self, v: &Vec<u8>) -> io::Result<Vec<u8>>;
    fn compress_min(&self) -> usize;
}

pub struct JSONMinifier;
impl Minifier for JSONMinifier {
    fn minify(&self, v: &Vec<u8>) -> io::Result<Vec<u8>> {
        let strip_comments = StripComments::new(Cursor::new(v));
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
    fn minify(&self, v: &Vec<u8>) -> io::Result<Vec<u8>> {
        let buf = match oxipng::optimize_from_memory(&v, &self.opts) {
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            Ok(x) => x
        };
        Ok(buf)
    }
    fn compress_min(&self) -> usize { 512 }
}