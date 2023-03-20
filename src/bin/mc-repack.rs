use std::{fs, io, path::{PathBuf, Path}, time::Instant};

use clap::Parser;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress, HumanBytes};

use mc_repack::optimizer::*;
use mc_repack::fop::*;
use mc_repack::errors::{ErrorCollector, SilentCollector};
use zip::write::FileOptions;

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

    /// Assume that the provided path is a "mods" directory (or its parent). This will make a new "mods" directory with repacked jars
    /// while the original ones will be stored in "mods_orig" directory. [Reserved for future use]
    #[arg(short = 'm', long)]
    mods_dir: bool,

    /// Do not print file errors
    #[arg(long)]
    silent: bool
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

    let file_opts = FileOptions::default().compression_level(Some(9));
    let mut dsum = 0;
    let mut zsum = 0;

    let pb2 = file_progress_bar();
    let mut ev: Vec<(String, String)> = Vec::new();
    let mut sc = SilentCollector;
    let ec: &mut dyn ErrorCollector = if ca.silent { &mut sc } else { &mut ev };
    
    let nfp = file_name_repack(fp);
    let fsum = optimize_archive(fp.to_owned(), nfp.clone(), pb2, ec, &file_opts, ca.use_blacklist)
        .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.display(), e)))?;
    dsum += fsum;
    zsum += file_size_diff(&fp, &nfp)?;

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
    let file_opts = FileOptions::default().compression_level(Some(9));
    let mut dsum = 0;
    let mut zsum = 0;
    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(file_progress_bar());
    
    let mut ev: Vec<(String, String)> = Vec::new();
    let mut sc = SilentCollector;
    let mut jev: Vec<(String, Vec<(String, String)>)> = Vec::new();
    let ec: &mut dyn ErrorCollector = if ca.silent { &mut sc } else { &mut ev };

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
            let fsum = optimize_archive(fp.clone(), nfp.clone(), pb2.clone(), ec, &file_opts, ca.use_blacklist)
                .map_err(|e| io::Error::new(e.kind(), format!("{}: {}",  fp.display(), e)))?;
            dsum += fsum;
            let rev = ec.get_results();
            if !rev.is_empty() {
                jev.push((fname.to_string(), rev));
            }
            zsum += file_size_diff(&fp, &nfp)?;
        }
    }
    mp.clear()?;

    if !ca.silent && !jev.is_empty() {
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

fn file_size_diff(a: &Path, b: &Path) -> io::Result<i64> {
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