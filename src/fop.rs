use std::{io::{self, Read, Write, Seek}, error::Error};

use flate2::read::DeflateEncoder;
use zip::{CompressionMethod, ZipWriter, write::FileOptions};


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

pub enum FileOp<'a> {
    Retain,
    Recompress(usize),
    Minify(&'a Box<dyn crate::minify::Minifier>),
    CheckContent,
    Ignore,
    Warn(Box<dyn Error>)
}

pub fn pack_file<W: Write + Seek>(
    z: &mut ZipWriter<W>,
    name: &str,
    opts: &FileOptions,
    data: &[u8],
    compress_min: usize
) -> io::Result<()> {
    z.start_file(name, opts.clone().compression_method(compress_check(data, compress_min)))?;
    z.write_all(data)
}

pub fn compress_check(b: &[u8], compress_min: usize) -> CompressionMethod {
    let lb = b.len();
    let nc = if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false };
    if nc { CompressionMethod::DEFLATE } else { CompressionMethod::STORE }
}