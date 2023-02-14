mod minify;
mod optimizer;

use std::{fs::{File, self}, io, env::args, error::Error};

use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanBytes};

use crate::optimizer::*;

fn main() -> io::Result<()> {
    println!("MC REPACK!");

    let dir = match args().skip(1).next() {
        Some(d) => d,
        None => return Err(io::Error::new(io::ErrorKind::Other, "No directory path provided"))
    };

    let mp = MultiProgress::new();

    let rd = fs::read_dir(dir)?;
    let optim = Optimizer::new();
    let mut dsum = 0;
    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(ProgressBar::new(0).with_style(
        ProgressStyle::with_template("# {bar} {pos}/{len} {wide_msg}").unwrap()
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
        let meta = fs::metadata(&fp)?;
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
        }
    }

    if dsum > 0 {
        pb.finish_with_message(format!("[REPACK] Bytes saved: {}", HumanBytes(dsum as u64)));
    }
    
    if !jev.is_empty() {
        mp.clear()?;
        eprintln!();
        eprintln!("Errors found while repacking:");
        for (f, v) in jev {
            eprintln!("| File: {}", f);
            for (pf, e) in v {
                eprintln!("| # {}: {}", pf, e);
            }
            eprintln!("|");
        }
    }

    Ok(())
}
