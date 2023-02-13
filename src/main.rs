mod minify;

use std::{fs::File, io::{self, Write, Read}, collections::HashMap};

use flate2::read::DeflateEncoder;
use indicatif::{ProgressBar, ProgressStyle};
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};

use crate::minify::*;

fn main() -> io::Result<()> {
    println!("MC REPACK!");

    let optim = Optimizer {
        minifiers: all_minifiers(),
        file_opts: FileOptions::default().compression_level(Some(9))
    };

    let of = File::open("mod.jar")?;
    let dsum = optim.optimize_file(&of, "mod_new.jar")?;
    println!("[REPACK] Bytes saved: {}", dsum);

    Ok(())
}

struct Optimizer{
    minifiers: HashMap<&'static str, Box<dyn Minifier>>,
    file_opts: FileOptions
}
impl Optimizer {
    fn optimize_file(
        &self,
        f: &File,
        new_name: &str
    ) -> io::Result<i64> {
        let mut oldjar = ZipArchive::new(f)?;
        let mut newjar = ZipWriter::new(File::create(new_name)?);
        
        let mut dsum = 0;
        let jfc = oldjar.len() as u64;
        let pb = ProgressBar::new(jfc).with_style(
            ProgressStyle::with_template("{bar} {pos}/{len} {wide_msg}").unwrap()
        );
    
    
        for i in 0..jfc {
            let mut jf = oldjar.by_index(i as usize)?;
            let fname = jf.name().to_string();
            pb.set_position(i);
            pb.set_message(fname.clone());
            if jf.is_dir() {
                newjar.raw_copy_file(jf)?;
                continue;
            }
            let comp = match fname.rsplit_once('.') {
                Some((_, x)) => self.minifiers.get(x),
                None => None
            };
            match comp {
                None => {
                    newjar.raw_copy_file(jf)?;
                    continue;
                },
                Some(c) => {
                    let fsz = jf.size() as i64;
                    let buf = match c.minify(&mut jf) {
                        Ok(x) => x,
                        Err(e) => {
                            println!("{}: {}", fname, e);
                            newjar.raw_copy_file(jf)?;
                            continue;
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
    
        pb.finish_with_message("Finished!");
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
