use zip::read::ZipFile;
use std::io::{self, BufReader, Read};

pub trait Minifier {
    fn minify(&self, f: &mut ZipFile) -> io::Result<(Vec<u8>, bool)>;
}

pub struct JSONMinifier;
impl Minifier for JSONMinifier {
    fn minify(&self, f: &mut ZipFile) -> io::Result<(Vec<u8>, bool)> {
        let br = BufReader::new(f);
        let v: serde_json::Value = serde_json::from_reader(br)?;
        let buf = serde_json::to_vec(&v)?;
        Ok((buf, true))
    }
}

pub struct PNGMinifier {
    pub opts: oxipng::Options
}
impl Minifier for PNGMinifier {
    fn minify(&self, f: &mut ZipFile) -> io::Result<(Vec<u8>, bool)> {
        let mut d = Vec::new();
        f.read_to_end(&mut d)?;
        let buf = match oxipng::optimize_from_memory(&d, &self.opts) {
            Err(e) => return Err(io::Error::new(io::ErrorKind::Other, e)),
            Ok(x) => x
        };
        let c = buf.len() > 512;
        Ok((buf, c))
    }
}