use std::{collections::HashMap, fs::File, io::{self, Read, BufReader, BufWriter}, error::Error, fmt};

use indicatif::ProgressBar;
use zip::{ZipArchive, write::FileOptions, ZipWriter};

use crate::{minify::{Minifier, all_minifiers, only_recompress}, blacklist, fop::{FileOp, pack_file}};

pub struct Optimizer{
    minifiers: HashMap<&'static str, Box<dyn Minifier>>,
    file_opts: FileOptions,
    use_blacklist: bool,
    optimize_class: bool,
}
impl Optimizer {
    pub fn new(use_blacklist: bool, optimize_class: bool) -> Self {
        Self {
            minifiers: all_minifiers(),
            file_opts: FileOptions::default().compression_level(Some(9)),
            use_blacklist,
            optimize_class
        }
    }
    pub fn optimize_archive(
        &self,
        fin: &File,
        fout: &File,
        pb: &ProgressBar,
        errors: &mut Vec<(String, Box<dyn Error>)>
    ) -> io::Result<i64> {
        let mut oldjar = ZipArchive::new(BufReader::new(fin))?;
        let mut newjar = ZipWriter::new(BufWriter::new(fout));
        
        let mut dsum = 0;
        let jfc = oldjar.len() as u64;
        pb.set_length(jfc);
    
        for i in 0..jfc {
            let mut jf = oldjar.by_index(i as usize)?;
            let fname = jf.name().to_string();
            pb.set_position(i);
            pb.set_message(fname.clone());

            match self.check_file_by_name(&fname) {
                FileOp::Retain => {
                    newjar.raw_copy_file(jf)?;
                }
                FileOp::Recompress(cmin) => {
                    let mut v = Vec::new();
                    jf.read_to_end(&mut v)?;
                    pack_file(
                        &mut newjar,
                        &fname,
                        &self.file_opts,
                        &v,
                        cmin
                    )?;
                }
                FileOp::Minify(m) => {
                    let fsz = jf.size() as i64;
                    let mut ubuf = Vec::new();
                    jf.read_to_end(&mut ubuf)?;
                    let buf = match m.minify(&ubuf) {
                        Ok(x) => x,
                        Err(e) => {
                            errors.push((fname.to_string(), e));
                            ubuf
                        }
                    };
                    dsum -= (buf.len() as i64) - fsz;
                    pack_file(&mut newjar, &fname, &self.file_opts, &buf, m.compress_min())?;
                }
                FileOp::CheckContent => {}
                FileOp::Ignore => {
                    errors.push((fname.to_string(), Box::new(blacklist::BlacklistedFile)));
                }
                FileOp::Warn(x) => {
                    errors.push((fname.to_string(), x));
                    newjar.raw_copy_file(jf)?;
                }
            }
        }
    
        pb.finish_with_message("Saving...");
        newjar.finish()?;
        
        Ok(dsum)
    }
    fn check_file_by_name(&self, fname: &str) -> FileOp {
        use FileOp::*;
        if fname.starts_with(".cache/") { return Ignore }
        if fname.ends_with('/') { return Retain }
        if fname.starts_with("META-INF/") {
            let sub = &fname[9..];
            match sub {
                "MANIFEST.MF" => {return Recompress(64) }
                "SIGNFILE.SF" | "SIGNFILE.DSA" => { return Warn(Box::new(StrError(ERR_SIGNFILE))) }
                x if x.starts_with("SIG-") || [".DSA", ".RSA", ".SF"].into_iter().any(|e| x.ends_with(e)) => {
                    return Warn(Box::new(StrError(ERR_SIGNFILE)))
                }
                x if x.starts_with("services/") => { return Recompress(64) }
                _ => {}
            }
        }
        let ftype = fname.rsplit_once('.').unzip().1.unwrap_or("");
        if ftype == "class" {
            return if self.optimize_class { Recompress(64) } else { Retain }
        }
        if only_recompress(ftype) {
            return Recompress(4)
        }
        match self.minifiers.get(ftype) {
            None => {
                if self.use_blacklist && blacklist::can_ignore_type(ftype) { Ignore } else { Retain }
            }
            Some(x) => { Minify(x) }
        }
    }
}

#[derive(Debug)]
pub struct StrError(pub &'static str);
impl Error for StrError {}
impl fmt::Display for StrError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.0)
    }
}

const ERR_SIGNFILE: &str = "This file cannot be repacked since it contains SHA-256 digests for zipped entries";