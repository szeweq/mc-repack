mod minify;
mod optimizer;

use std::{fs::{File, self}, io, env::args, error::Error, path::{PathBuf, Path}};

use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanBytes};

use crate::optimizer::*;

fn main() -> io::Result<()> {
    println!("MC REPACK!");

    let spec_path = match args().skip(1).next() {
        Some(d) => PathBuf::from(d),
        None => return Err(io::Error::new(io::ErrorKind::Other, "No path provided"))
    };

    let path_meta = spec_path.metadata()?;

    let (dsum, zsum) = if path_meta.is_dir() {
        process_dir(&spec_path)
    } else if path_meta.is_file() {
        process_file(&spec_path)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Not a file or directory"))
    }?;

    if dsum > 0 {
        println!("[REPACK] Bytes saved by minifying: {}", HumanBytes(dsum as u64));
    }
    if zsum > 0 {
        println!("[REPACK] Bytes saved by repacking: {}", HumanBytes(zsum as u64));
    }

    Ok(())
}

const DOT_JAR: &str = ".jar";
const REPACK_JAR: &str = "$repack.jar";

const PB_STYLE_ZIP: &str = "# {bar} {pos}/{len} {wide_msg}";

fn process_file<P: AsRef<Path>>(p: P) -> io::Result<(i64, i64)> {
    let fp = p.as_ref();
    if let Some(fname) = fp.file_name() {
        let s = fname.to_string_lossy();
        if !s.ends_with(DOT_JAR) {
            return Err(io::Error::new(io::ErrorKind::Other, "Not a .jar file"))
        }
        if s.ends_with(REPACK_JAR) {
            return Err(io::Error::new(io::ErrorKind::Other, "This .jar is marked as repacked, no re-repacking needed"))
        }
    }

    let optim = Optimizer::new();
    let mut dsum = 0;
    let mut zsum = 0;

    let pb2 = ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    );
    let mut ev: Vec<(String, Box<dyn Error>)> = Vec::new();

    let Some(fstem) = fp.file_stem() else {
        return Err(io::Error::new(io::ErrorKind::Other, "Not a named file"))
    };
    let nfp = fp.with_file_name(format!("{}$repack.jar", fstem.to_string_lossy()));
    let inf = File::open(&fp)?;
    let outf = File::create(&nfp)?;
    let fsum = optim.optimize_file(&inf, &outf, &pb2, &mut ev)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.to_str().unwrap(), e)))?;
    dsum += fsum;
    zsum += file_size_diff(&inf, &outf)?;

    if !ev.is_empty() {
        eprintln!();
        eprintln!("Errors found while repacking a file:");
        for (f, e) in ev {
            eprintln!("| # {}: {}", f, e);
        }
    }

    Ok((dsum, zsum))
}

fn process_dir<P: AsRef<Path>>(p: P) -> io::Result<(i64, i64)> {
    let mp = MultiProgress::new();

    let rd = fs::read_dir(p)?;
    let optim = Optimizer::new();
    let mut dsum = 0;
    let mut zsum = 0;
    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    ));
    
    let mut ev: Vec<(String, Box<dyn Error>)> = Vec::new();
    let mut jev: Vec<(String, Vec<(String, Box<dyn Error>)>)> = Vec::new();

    for rde in rd {
        let rde = rde?;
        let fp = rde.path();
        let rfn = rde.file_name();
        let Some(fname) = rfn.to_str() else {
            return Err(io::Error::new(io::ErrorKind::NotFound, "A path has no file name"))
        };
        let meta = fp.metadata()?;
        if meta.is_file() && fname.ends_with(".jar") && !fname.ends_with("$repack.jar") {
            pb.set_message(fname.to_string());
            let (fpart, _) = fname.rsplit_once('.').unwrap();
            let nfp = fp.with_file_name(format!("{}$repack.jar", fpart));
            let inf = File::open(&fp)?;
            let outf = File::create(&nfp)?;
            let fsum = optim.optimize_file(&inf, &outf, &pb2, &mut ev)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.to_str().unwrap(), e)))?;
            dsum += fsum;
            if !ev.is_empty() {
                let nev = ev;
                jev.push((fname.to_string(), nev));
                ev = Vec::new();
            }
            zsum += file_size_diff(&inf, &outf)?;
        }
    }
    mp.clear()?;

    if !jev.is_empty() {
        eprintln!();
        eprintln!("Errors found while repacking files:");
        for (f, v) in jev {
            eprintln!("| File: {}", f);
            for (pf, e) in v {
                eprintln!("| # {}: {}", pf, e);
            }
            eprintln!("|");
        }
    }

    Ok((dsum, zsum))
}

fn file_size_diff(a: &File, b: &File) -> io::Result<i64> {
    Ok((a.metadata()?.len() as i64) - (b.metadata()?.len() as i64))
}