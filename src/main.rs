mod minify;

use std::{fs::{File, self}, io::{self, Write, Read}, collections::HashMap, env::args};

use flate2::read::DeflateEncoder;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};
use zip::{ZipArchive, ZipWriter, write::FileOptions, CompressionMethod};

use crate::minify::*;

fn main() -> io::Result<()> {
    println!("MC REPACK!");

    let dir = match args().skip(1).next() {
        Some(d) => d,
        None => return Err(io::Error::new(io::ErrorKind::Other, "No directory path provided"))
    };

    let mp = MultiProgress::new();

    let rd = fs::read_dir(dir)?;
    let optim = Optimizer {
        minifiers: all_minifiers(),
        file_opts: FileOptions::default().compression_level(Some(9))
    };
    let mut dsum = 0;
    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));

    for rde in rd {
        let rde = rde?;
        let fp = rde.path();
        let rfn = rde.file_name();
        let Some(fname) = rfn.to_str() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "A path has no file name"))
        };
        let meta = fs::metadata(&fp)?;
        if meta.is_file() && fname.ends_with(".jar") {
            pb.set_message(fname.to_string());
            let (fpart, _) = fname.rsplit_once('.').unwrap();
            let nfp = fp.with_file_name(format!("{}_new.jar", fpart));
            let inf = File::open(&fp)?;
            let outf = File::create(&nfp)?;
            let fsum = optim.optimize_file(&inf, &outf, &mp)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.to_str().unwrap(), e)))?;
            dsum += fsum;
        }
    }

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
        fin: &File,
        fout: &File,
        mp: &MultiProgress
    ) -> io::Result<i64> {
        let mut oldjar = ZipArchive::new(fin)?;
        let mut newjar = ZipWriter::new(fout);
        
        let mut dsum = 0;
        let jfc = oldjar.len() as u64;
        let pb = mp.add(ProgressBar::new(jfc).with_style(
            ProgressStyle::with_template("# {bar} {pos}/{len} {wide_msg}").unwrap()
        ));
    
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
                    let mut ubuf = Vec::new();
                    jf.read_to_end(&mut ubuf)?;
                    let buf = match c.minify(&ubuf) {
                        Ok(x) => x,
                        Err(e) => {
                            println!("{}: {}", fname, e);
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
    
        pb.finish_with_message("Finished!");
        newjar.finish()?;
        mp.remove(&pb);
        
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
