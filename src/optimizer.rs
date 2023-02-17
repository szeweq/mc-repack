use std::{collections::HashMap, fs::File, io::{self, Read, Write}, error::Error};

use flate2::read::DeflateEncoder;
use indicatif::ProgressBar;
use zip::{ZipArchive, CompressionMethod, write::FileOptions, ZipWriter};

use crate::{minify::{Minifier, all_minifiers}, blacklist};

pub struct Optimizer{
    minifiers: HashMap<&'static str, Box<dyn Minifier>>,
    file_opts: FileOptions
}
impl Optimizer {
    pub fn new() -> Self {
        Self {
            minifiers: all_minifiers(),
            file_opts: FileOptions::default().compression_level(Some(9))
        }
    }
    pub fn optimize_file(
        &self,
        fin: &File,
        fout: &File,
        pb: &ProgressBar,
        errors: &mut Vec<(String, Box<dyn Error>)>
    ) -> io::Result<i64> {
        let mut oldjar = ZipArchive::new(fin)?;
        let mut newjar = ZipWriter::new(fout);
        
        let mut dsum = 0;
        let jfc = oldjar.len() as u64;
        pb.set_length(jfc);
    
        for i in 0..jfc {
            let mut jf = oldjar.by_index(i as usize)?;
            let fname = jf.name().to_string();
            pb.set_position(i);
            pb.set_message(fname.clone());
            if jf.is_dir() {
                newjar.raw_copy_file(jf)?;
                continue;
            }
            let ftype = match fname.rsplit_once('.') {
                Some((_, x)) => x,
                None => ""
            };
            match self.minifiers.get(ftype) {
                None => {
                    if !blacklist::can_ignore_type(ftype) {
                        newjar.raw_copy_file(jf)?;
                    } else {
                        errors.push((fname.to_string(), Box::new(blacklist::BlacklistedFile)))
                    }
                    continue;
                },
                Some(c) => {
                    let fsz = jf.size() as i64;
                    let mut ubuf = Vec::new();
                    jf.read_to_end(&mut ubuf)?;
                    let buf = match c.minify(&ubuf) {
                        Ok(x) => x,
                        Err(e) => {
                            errors.push((fname.to_string(), e));
                            ubuf
                        }
                    };
                    dsum -= (buf.len() as i64) - fsz;
                    newjar.start_file(&fname, self.file_opts.clone()
                        .compression_method(compress_check(&buf, c.compress_min())?)
                    )?;
                    newjar.write_all(&buf)?;
                }
            }
        }
    
        pb.finish_with_message("Saving...");
        newjar.finish()?;
        
        Ok(dsum)
    }
}

fn compress_check(b: &[u8], compress_min: usize) -> io::Result<CompressionMethod> {
    let lb = b.len();
    let nc = if lb > compress_min {
        let de = DeflateEncoder::new(b, flate2::Compression::best());
        let sum = de.bytes().count();
        sum < lb
    } else { false };
    Ok(if nc { CompressionMethod::Deflated } else { CompressionMethod::Stored })
}