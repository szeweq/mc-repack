use std::{fs, io, path::{PathBuf, Path}, thread::{self, JoinHandle}, any::Any};

use clap::Parser;
use crossbeam_channel::Sender;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack::{optimizer::*, fop::*, errors::{EntryRepackError, ErrorCollector}};
use zip::write::FileOptions;

mod cli_args;

#[cfg(not(feature = "anyhow"))]
type Result_<T> = io::Result<T>;
#[cfg(feature = "anyhow")]
type Result_<T> = anyhow::Result<T>;

fn main() -> Result_<()> {
    let args = cli_args::Args::parse();
    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █ by Szeweq\n");
    
    let cli_args::Args { path, silent, use_blacklist, ..} = args;
    let fmeta = path.metadata()?;
    let ropts = RepackOpts { silent, use_blacklist };
    let task: &dyn ProcessTask = if fmeta.is_dir() {
        &JarDirRepackTask
    } else if fmeta.is_file() {
        &JarRepackTask
    } else {
        return Err(TaskError::NotFileOrDir.into());
    };
    print_entry_errors(task.process(&path, args.out, &ropts)?.results());
    Ok(())
}

const PB_STYLE_ZIP: &str = "# {pos}/{len} {wide_msg}";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

struct RepackOpts {
    silent: bool,
    use_blacklist: bool
}

trait ProcessTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> Result_<ErrorCollector>;
}
fn task_err(_: Box<dyn Any + Send>) -> io::Error { new_io_error("Task failed") }

struct JarRepackTask;
impl ProcessTask for JarRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> Result_<ErrorCollector> {
        let RepackOpts { silent, use_blacklist } = *opts;
        let fname = if let Some(x) = fp.file_name() {
            x.to_string_lossy()
        } else {
            return Err(TaskError::InvalidFileName.into())
        };
        match check_file_type(&fname) {
            FileType::Other => { return Err(TaskError::NotZip.into()) }
            FileType::Repacked => { return Err(TaskError::AlreadyRepacked.into()) }
            _ => {}
        }
    
        let file_opts = FileOptions::default().compression_level(Some(9));
    
        let pb2 = file_progress_bar();
        let mut ec = ErrorCollector::new(silent);
        let (pj, ps) = thread_progress_bar(pb2);
        
        let nfp = out.unwrap_or_else(|| file_name_repack(fp));
        optimize_archive(fp.to_owned().into_boxed_path(), nfp.into_boxed_path(), &ps, &mut ec, &file_opts, use_blacklist)
            .map_err(|e| io::Error::new(e.kind(), format!("{}: {}", fp.display(), e)))?;
        drop(ps);
        pj.join().map_err(task_err)?;
    
        Ok(ec)
    }
}

struct JarDirRepackTask;
impl ProcessTask for JarDirRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts) -> Result_<ErrorCollector> {
        let RepackOpts { silent, use_blacklist } = *opts;
        let mp = MultiProgress::new();

        let rd = fs::read_dir(fp)?;
        let file_opts = FileOptions::default().compression_level(Some(9));

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
                return Err(TaskError::InvalidFileName.into())
            };
            let meta = fp.metadata()?;
            if meta.is_file() && check_file_type(fname) == FileType::Original {
                ec.rename(fname);
                pb.set_message(fname.to_string());
                
                let nfp = new_path(out.as_ref(), &fp);
                optimize_archive(fp.into_boxed_path(), nfp.into_boxed_path(), &ps, &mut ec, &file_opts, use_blacklist)
                    .map_err(|e| io::Error::new(e.kind(), format!("{}: {}",  rde.path().display(), e)))?;
            }
        }
        mp.clear()?;
        drop(ps);
        pj.join().map_err(task_err)?;

        Ok(ec)
    }
}

fn file_name_repack(p: &Path) -> PathBuf {
    let stem = p.file_stem().and_then(std::ffi::OsStr::to_str).unwrap_or_default();
    let ext = p.extension().and_then(std::ffi::OsStr::to_str).unwrap_or_default();
    let nw = format!("{}_repack.{}", stem, ext);
    p.with_file_name(nw)
}

fn new_path(src: Option<&PathBuf>, p: &Path) -> PathBuf {
    match src {
        None => file_name_repack(p),
        Some(x) => {
            let mut np = x.clone();
            np.push(p.file_name().unwrap_or_default());
            np
        }
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
                    pb.set_message(msg.to_string());
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
        eprintln!("Errors found in files:");
        for ere in v {
            eprintln!(" # {ere}");
        }
    }
}

#[derive(Debug)]
enum TaskError {
    InvalidFileName,
    NotZip,
    NotFileOrDir,
    AlreadyRepacked
}
impl std::error::Error for TaskError {}
impl std::fmt::Display for TaskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            TaskError::InvalidFileName => "invalid file name",
            TaskError::NotZip => "not a ZIP archive",
            TaskError::NotFileOrDir => "not a file or directory",
            TaskError::AlreadyRepacked => "this archive is marked as repacked, no re-repacking needed"
        })
    }
}
impl From<TaskError> for io::Error {
    fn from(val: TaskError) -> Self {
        io::Error::new(io::ErrorKind::Other, val)
    }
}