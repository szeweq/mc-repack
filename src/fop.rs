use std::{io::{self, Read, Write, Seek}};

use flate2::bufread::DeflateEncoder;
use zip::{CompressionMethod, ZipWriter, write::FileOptions};

use crate::minify::MinifyType;


pub const REPACKED: &str = "$repack";

#[derive(PartialEq)]
pub enum FileType {
    Other, Original, Repacked
}
pub fn check_file_type(s: &str) -> FileType {
    use FileType::*;
    match s.rsplit_once('.') {
        Some((n, "jar" | "zip")) => if n.ends_with(REPACKED) { Repacked } else { Original }
        _ => Other
    }
}

pub enum FileOp {
    Recompress(usize),
    Minify(MinifyType),
    Ignore,
    Signfile,
    Warn(String)
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