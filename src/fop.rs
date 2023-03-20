use std::{io::{self, Read, Write, Seek}};

use flate2::bufread::DeflateEncoder;
use zip::{CompressionMethod, ZipWriter, write::FileOptions};

use crate::minify::MinifyType;


pub(crate) const REPACKED: &str = "$repack";

/// A file type (not extension) that MC-Repack will check before repacking.
#[derive(PartialEq)]
pub enum FileType {
    /// Type for files which MC-Repack cannot repack.
    Other,
    /// A JAR or ZIP file which was not yet repacked.
    Original,
    /// A repacked file.
    Repacked
}

/// Checks file type based on its file name.
pub fn check_file_type(s: &str) -> FileType {
    use FileType::*;
    match s.rsplit_once('.') {
        Some((n, "jar" | "zip")) => if n.ends_with(REPACKED) { Repacked } else { Original }
        _ => Other
    }
}

/// A file operation needed before a file is saved in repacked archive
pub enum FileOp {
    /// Recompress data (check minimal size to determine if a file can be compressed or not).
    Recompress(usize),
    /// Minify a file.
    Minify(MinifyType),
    /// Ignore a file.
    Ignore,
    /// A "Signfile" was found.
    Signfile,
    /// Give a warning about a file.
    Warn(String)
}

pub(crate) fn pack_file<W: Write + Seek>(
    z: &mut ZipWriter<W>,
    name: &str,
    opts: &FileOptions,
    data: &[u8],
    compress_min: usize
) -> io::Result<()> {
    z.start_file(name, opts.clone().compression_method(compress_check(data, compress_min)))?;
    z.write_all(data)
}

fn compress_check(b: &[u8], compress_min: usize) -> CompressionMethod {
    let lb = b.len();
    let nc = if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false };
    if nc { CompressionMethod::DEFLATE } else { CompressionMethod::STORE }
}