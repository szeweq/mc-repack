mod minify;
mod optimizer;
mod blacklist;

use std::{fs::{File, self}, io, error::Error, path::{PathBuf, Path}, time::Instant};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanBytes};

use crate::optimizer::*;

#[derive(Debug, Parser)]
#[command(version)]
struct CliArgs {
    /// Path to a file/directory of .jar archive(s)
    path: PathBuf,

    /// Optimize more file formats (potentially breaking their debugging); Flag reserved for a future version
    #[arg(short, long)]
    aggressive: bool,

    /// Use built-in blacklist for files
    #[arg(short = 'b', long)]
    use_blacklist: bool,

    /// Try to recompress .class files (may produce larger files)
    #[arg(long)]
    optimize_class: bool,

    /// Assume that the provided path is a "mods" directory (or its parent). This will make a new "mods" directory with repacked jars
    /// while the original ones will be stored in "mods_orig" directory.
    #[arg(short = 'm', long)]
    mods_dir: bool
}

fn main() -> io::Result<()> {
    let cli_args = CliArgs::parse();

    println!("MC REPACK!");
    let dt = Instant::now();

    let path_meta = cli_args.path.metadata()?;

    let sums = if path_meta.is_dir() {
        process_dir(&cli_args)
    } else if path_meta.is_file() {
        process_file(&cli_args)
    } else {
        Err(io::Error::new(io::ErrorKind::Other, "Not a file or directory"))
    }?;

    let dsum = sums.0.max(0) as u64;
    let zsum = sums.1.max(0) as u64;

    println!("Bytes saved: {} by minifying, {} by repacking", HumanBytes(dsum), HumanBytes(zsum));
    println!("Done in: {:.3?}", dt.elapsed());

    Ok(())
}

const DOT_JAR: &str = ".jar";
const REPACK_JAR: &str = "$repack.jar";

const PB_STYLE_ZIP: &str = "# {bar} {pos}/{len} {wide_msg}";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

fn process_file(ca: &CliArgs) -> io::Result<(i64, i64)> {
    let fp = &ca.path;
    if let Some(fname) = fp.file_name() {
        let s = fname.to_string_lossy();
        if !s.ends_with(DOT_JAR) {
            return Err(io::Error::new(io::ErrorKind::Other, "Not a .jar file"))
        }
        if s.ends_with(REPACK_JAR) {
            return Err(io::Error::new(io::ErrorKind::Other, "This .jar is marked as repacked, no re-repacking needed"))
        }
    }

    let optim = Optimizer::new(ca.use_blacklist, ca.optimize_class);
    let mut dsum = 0;
    let mut zsum = 0;

    let pb2 = file_progress_bar();
    let mut ev: Vec<(String, Box<dyn Error>)> = Vec::new();

    let Some(fstem) = fp.file_stem() else {
        return Err(io::Error::new(io::ErrorKind::Other, "Not a named file"))
    };
    let nfp = file_name_repack(fp, &fstem.to_string_lossy());
    let inf = File::open(&fp)?;
    let outf = File::create(&nfp)?;
    let fsum = optim.optimize_file(&inf, &outf, &pb2, &mut ev)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.display(), e)))?;
    dsum += fsum;
    zsum += file_size_diff(&inf, &outf)?;

    if !ev.is_empty() {
        eprintln!("Errors found while repacking a file:");
        for (f, e) in ev {
            eprintln!("| # {}: {}", f, e);
        }
    }

    Ok((dsum, zsum))
}

fn process_dir(ca: &CliArgs) -> io::Result<(i64, i64)> {
    let p = &ca.path;
    let mp = MultiProgress::new();

    let rd = fs::read_dir(p)?;
    let optim = Optimizer::new(ca.use_blacklist, ca.optimize_class);
    let mut dsum = 0;
    let mut zsum = 0;
    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(file_progress_bar());
    
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
        if meta.is_file() && fname.ends_with(DOT_JAR) && !fname.ends_with(REPACK_JAR) {
            pb.set_message(fname.to_string());
            let fpart = &fname[..fname.len()-4];
            let nfp = file_name_repack(&fp, &fpart);
            let inf = File::open(&fp)?;
            let outf = File::create(&nfp)?;
            let fsum = optim.optimize_file(&inf, &outf, &pb2, &mut ev)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}",  fp.display(), e)))?;
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
        eprintln!("Errors found while repacking files:");
        for (f, v) in jev {
            eprintln!(" File: {}", f);
            for (pf, e) in v {
                eprintln!(" # {}: {}", pf, e);
            }
            eprintln!();
        }
    }

    Ok((dsum, zsum))
}

fn file_size_diff(a: &File, b: &File) -> io::Result<i64> {
    Ok((a.metadata()?.len() as i64) - (b.metadata()?.len() as i64))
}

fn file_name_repack(p: &Path, s: &str) -> PathBuf {
    p.with_file_name(format!("{}$repack.jar", s))
}