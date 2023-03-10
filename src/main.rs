mod minify;
mod optimizer;
mod blacklist;
mod fop;

use std::{fs::{File, self}, io, error::Error, path::{PathBuf, Path}, time::Instant};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanBytes};

use crate::optimizer::*;
use crate::fop::*;

#[derive(Debug, Parser)]
#[command(version)]
struct CliArgs {
    /// (Optional) Path to a file/directory of archives (JAR and ZIP)
    path: Option<PathBuf>,

    /// Optimize more file formats (potentially breaking their debugging) [Reserved for future use]
    #[arg(short, long)]
    aggressive: bool,

    /// Use built-in blacklist for files
    #[arg(short = 'b', long)]
    use_blacklist: bool,

    /// Try to recompress .class files (may produce larger files)
    #[arg(long)]
    optimize_class: bool,

    /// Assume that the provided path is a "mods" directory (or its parent). This will make a new "mods" directory with repacked jars
    /// while the original ones will be stored in "mods_orig" directory. [Reserved for future use]
    #[arg(short = 'm', long)]
    mods_dir: bool
}

fn main() -> io::Result<()> {
    let cli_args = CliArgs::parse();

    println!(r"
    █▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀
    █ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █
    ");
    let dt = Instant::now();

    let fpath = cli_args.path.clone().unwrap_or_else(|| {
        use dialoguer::{theme::ColorfulTheme, Input};
        let fstr: String = Input::with_theme(&ColorfulTheme::default())
            .with_prompt("Path to a file/directory").interact_text().unwrap();
        PathBuf::from(fstr)
    });

    let path_meta = fpath.metadata()?;

    let sums = if path_meta.is_dir() {
        process_dir(&cli_args, &fpath)
    } else if path_meta.is_file() {
        process_file(&cli_args, &fpath)
    } else {
        Err(new_io_error("Not a file or directory"))
    }?;

    let dsum = sums.0.max(0) as u64;
    let zsum = sums.1.max(0) as u64;

    println!("Bytes saved: {} by minifying, {} by repacking", HumanBytes(dsum), HumanBytes(zsum));
    println!("Done in: {:.3?}", dt.elapsed());

    Ok(())
}

const PB_STYLE_ZIP: &str = "# {bar} {pos}/{len} {wide_msg}";

const ERR_FNAME_INVALID: &str = "Invalid file name";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

fn process_file(ca: &CliArgs, fp: &Path) -> io::Result<(i64, i64)> {
    let fname = if let Some(x) = fp.file_name() {
        x.to_string_lossy()
    } else {
        return Err(new_io_error(ERR_FNAME_INVALID))
    };
    match check_file_type(&fname) {
        FileType::Other => { return Err(new_io_error("File is not an JAR/ZIP archive")) }
        FileType::Repacked => { return Err(new_io_error("This archive is marked as repacked, no re-repacking needed")) }
        _ => {}
    }

    let optim = Optimizer::new(ca.use_blacklist, ca.optimize_class);
    let mut dsum = 0;
    let mut zsum = 0;

    let pb2 = file_progress_bar();
    let mut ev: Vec<(String, Box<dyn Error>)> = Vec::new();
    
    let nfp = file_name_repack(fp);
    let inf = File::open(&fp)?;
    let outf = File::create(&nfp)?;
    let fsum = optim.optimize_archive(&inf, &outf, &pb2, &mut ev)
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

fn process_dir(ca: &CliArgs, p: &Path) -> io::Result<(i64, i64)> {
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
            return Err(new_io_error(ERR_FNAME_INVALID))
        };
        let meta = fp.metadata()?;
        if meta.is_file() && check_file_type(fname) == FileType::Original {
            pb.set_message(fname.to_string());
            
            let nfp = file_name_repack(&fp);
            let inf = File::open(&fp)?;
            let outf = File::create(&nfp)?;
            let fsum = optim.optimize_archive(&inf, &outf, &pb2, &mut ev)
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

fn file_name_repack(p: &Path) -> PathBuf {
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let ext = p.extension().unwrap_or_default().to_string_lossy();
    let x = stem + "$repack." + ext;
    p.with_file_name(x.to_string())
}

fn new_io_error(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, s)
}