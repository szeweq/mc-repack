use std::{fs, io, path::{PathBuf, Path}, thread::{self, JoinHandle}, any::Any};

use clap::Parser;
use crossbeam_channel::Sender;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack::optimizer::*;
use mc_repack::fop::*;
use mc_repack::errors::{ErrorCollector, SilentCollector};
use zip::write::FileOptions;

mod cli_args;

fn main() -> io::Result<()> {
    let args = cli_args::Args::parse();

    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █\n");

    let fpath = args.actual_path();

    process_task_from(&args, fpath.metadata()?)?
        .process(&fpath, args.out)?;

    Ok(())
}

const PB_STYLE_ZIP: &str = "# {pos}/{len} {wide_msg}";

const ERR_FNAME_INVALID: &str = "Invalid file name";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

fn process_task_from(ca: &cli_args::Args, fmeta: fs::Metadata) -> io::Result<Box<dyn ProcessTask>> {
    let cli_args::Args { silent, use_blacklist , ..} = *ca;
    if fmeta.is_dir() {
        Ok(Box::new(JarDirRepackTask { silent, use_blacklist }))
    } else if fmeta.is_file() {
        Ok(Box::new(JarRepackTask { silent, use_blacklist }))
    } else {
        Err(new_io_error("Not a file or directory"))
    }
}

trait ProcessTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>) -> io::Result<()>;
}
const TASK_ERR: fn(Box<dyn Any + Send>) -> io::Error = |_| new_io_error("Task failed");

struct JarRepackTask {
    silent: bool,
    use_blacklist: bool
}
impl ProcessTask for JarRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>) -> io::Result<()> {
        let Self { silent, use_blacklist } = *self;
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
    
        let pb2 = file_progress_bar();
        let mut ev: Vec<(String, String)> = Vec::new();
        let mut sc = SilentCollector;
        let ec: &mut dyn ErrorCollector = if silent { &mut sc } else { &mut ev };
        let (pj, ps) = thread_progress_bar(pb2);
        
        let nfp = if let Some(pp) = out { pp } else { file_name_repack(fp) };
        optimize_archive(fp.to_owned(), nfp, &ps, ec, &file_opts, use_blacklist)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.display(), e)))?;
        drop(ps);
        pj.join().map_err(TASK_ERR)?;
    
        if !ev.is_empty() {
            eprintln!("Errors found while repacking a file:");
            for (f, e) in ev {
                eprintln!("| # {}: {}", f, e);
            }
        }
    
        Ok(())
    }
}

struct JarDirRepackTask {
    silent: bool,
    use_blacklist: bool,
}
impl ProcessTask for JarDirRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>) -> io::Result<()> {
        let Self { silent, use_blacklist } = *self;
        let mp = MultiProgress::new();

        let rd = fs::read_dir(fp)?;
        let file_opts = FileOptions::default().compression_level(Some(9));

        let ren: &dyn NewPath = if let Some(pp) = &out {
            fs::create_dir_all(pp)?;
            pp
        } else { &() };

        let pb = mp.add(ProgressBar::new_spinner().with_style(
            ProgressStyle::with_template("{wide_msg}").unwrap()
        ));
        let pb2 = mp.add(file_progress_bar());
        
        let mut ev: Vec<(String, String)> = Vec::new();
        let mut sc = SilentCollector;
        let mut jev: Vec<(String, Vec<(String, String)>)> = Vec::new();
        let ec: &mut dyn ErrorCollector = if silent { &mut sc } else { &mut ev };
        let (pj, ps) = thread_progress_bar(pb2);

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
                
                let nfp = ren.new_path(&fp);
                optimize_archive(fp.clone(), nfp.clone(), &ps, ec, &file_opts, use_blacklist)
                    .map_err(|e| io::Error::new(e.kind(), format!("{}: {}",  fp.display(), e)))?;
                let rev = ec.get_results();
                if !rev.is_empty() {
                    jev.push((fname.to_string(), rev));
                }
            }
        }
        mp.clear()?;
        drop(ps);
        pj.join().map_err(TASK_ERR)?;

        if !silent && !jev.is_empty() {
            eprintln!("Errors found while repacking files:");
            for (f, v) in jev {
                eprintln!(" File: {}", f);
                for (pf, e) in v {
                    eprintln!(" # {}: {}", pf, e);
                }
                eprintln!();
            }
        }

        Ok(())
    }
}

fn file_name_repack(p: &Path) -> PathBuf {
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let ext = p.extension().unwrap_or_default().to_string_lossy();
    let x = stem + "$repack." + ext;
    p.with_file_name(x.to_string())
}

trait NewPath { fn new_path(&self, p: &Path) -> PathBuf; }
impl NewPath for () {
    fn new_path(&self, p: &Path) -> PathBuf {
        file_name_repack(p)
    }
}
impl NewPath for PathBuf {
    fn new_path(&self, p: &Path) -> PathBuf {
        let mut np = self.clone();
        np.push(p.file_name().unwrap_or_default());
        np
    }
}

fn new_io_error(s: &str) -> io::Error {
    io::Error::new(io::ErrorKind::Other, s)
}

fn thread_progress_bar(pb: ProgressBar) -> (JoinHandle<()>, Sender<ProgressState>) {
    let (ps, pr) = crossbeam_channel::unbounded();
    let pj = thread::spawn(move || {
        use ProgressState::*;
        for st in pr {
            match st {
                Start(u) => { pb.set_length(u); }
                Push(num, msg) => {
                    pb.set_position(num);
                    pb.set_message(msg);
                }
                Finish => {
                    pb.finish_with_message("Saving...");
                }
            }
        }
    });
    (pj, ps)
}