use std::io::{self, Read};

use flate2::read::DeflateEncoder;
use zip::CompressionMethod;


pub const DOT_JAR: &str = ".jar";
pub const REPACK_JAR: &str = "$repack.jar";

#[derive(PartialEq)]
pub enum FileType {
    Other, Jar, RepackedJar
}
pub fn check_file_type(s: &str) -> FileType {
    use FileType::*;
    if s.ends_with(DOT_JAR) {
        return if s.ends_with(REPACK_JAR) { RepackedJar } else { Jar }
    }
    Other
}

enum FileOp {
    Retain,
    Recompress,
    Minify(Box<dyn crate::minify::Minifier>),
    Ignore,
    Warn(String)
}

pub fn compress_check(b: &[u8], compress_min: usize) -> io::Result<CompressionMethod> {
    let lb = b.len();
    let nc = if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false };
    Ok(if nc { CompressionMethod::DEFLATE } else { CompressionMethod::STORE })
}