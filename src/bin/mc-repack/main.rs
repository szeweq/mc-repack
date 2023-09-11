use std::{fs, io, path::{PathBuf, Path}, thread::{self, JoinHandle}, any::Any, error::Error};

use clap::Parser;
use crossbeam_channel::Sender;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack::{optimizer::*, fop::*, errors::{EntryRepackError, ErrorCollector}};
use zip::write::FileOptions;

mod cli_args;

fn main() -> io::Result<()> {
    let args = cli_args::Args::parse();
    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █\n");
    process_task_from(args)
}

const PB_STYLE_ZIP: &str = "# {pos}/{len} {wide_msg}";

const ERR_FNAME_INVALID: &str = "Invalid file name";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

fn process_task_from(ca: cli_args::Args) -> io::Result<()> {
    let cli_args::Args { silent, use_blacklist, ..} = ca;
    let fmeta = ca.path.metadata()?;
    let ropts = RepackOpts { silent, use_blacklist };
    let task: Box<dyn ProcessTask> = if fmeta.is_dir() {
        Box::new(JarDirRepackTask)
    } else if fmeta.is_file() {
        Box::new(JarRepackTask)
    } else {
        return Err(new_io_error("Not a file or directory"))
    };
    print_entry_errors(task.process(&ca.path, ca.out, &ropts)?.results());
    Ok(())
}

struct RepackOpts {
    silent: bool,
    use_blacklist: bool
}

trait ProcessTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> io::Result<ErrorCollector>;
}
fn task_err(_: Box<dyn Any + Send>) -> io::Error { new_io_error("Task failed") }

struct JarRepackTask;
impl ProcessTask for JarRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> io::Result<ErrorCollector> {
        let RepackOpts { silent, use_blacklist } = *opts;
        let fname = if let Some(x) = fp.file_name() {
            x.to_string_lossy()
        } else {
            return Err(new_io_error(ERR_FNAME_INVALID))
        };
        match check_file_type(&fname) {
            FileType::Other => { return Err(new_io_error("Not a JAR/ZIP archive")) }
            FileType::Repacked => { return Err(new_io_error("This archive is marked as repacked, no re-repacking needed")) }
            _ => {}
        }
    
        let file_opts = FileOptions::default().compression_level(Some(9));
    
        let pb2 = file_progress_bar();
        let mut ec = ErrorCollector::new(silent);
        let (pj, ps) = thread_progress_bar(pb2);
        
        let nfp = out.unwrap_or_else(|| file_name_repack(fp));
        optimize_archive(fp.to_owned(), nfp, &ps, &mut ec, &file_opts, use_blacklist)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.display(), e)))?;
        drop(ps);
        pj.join().map_err(task_err)?;
    
        Ok(ec)
    }
}

struct JarDirRepackTask;
impl ProcessTask for JarDirRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> io::Result<ErrorCollector> {
        let RepackOpts { silent, use_blacklist } = *opts;
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
        
        let mut ec = ErrorCollector::new(silent);
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
                ec.rename(&fname);
                pb.set_message(fname.to_string());
                
                let nfp = ren.new_path(&fp);
                optimize_archive(fp.clone(), nfp.clone(), &ps, &mut ec, &file_opts, use_blacklist)
                    .map_err(|e| io::Error::new(e.kind(), format!("{}: {}",  fp.display(), e)))?;
            }
        }
        mp.clear()?;
        drop(ps);
        pj.join().map_err(task_err)?;

        Ok(ec)
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

fn print_entry_errors(v: &[EntryRepackError]) {
    if !v.is_empty() {
        eprintln!("Errors found in file entries:");
        for ere in v {
            eprintln!(" # {}: {}", ere.name, ere.source().map_or("no error".to_string(), |e| e.to_string()));
        }
    }
}