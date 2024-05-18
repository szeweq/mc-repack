use std::{any::Any, ffi::OsString, fs::{self, File}, io, path::{Path, PathBuf}, thread::{self, JoinHandle}};
use clap::Parser;
use cli_args::RepackOpts;
use crossbeam_channel::Sender;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack_core::{cfg, entry::{self, EntryReader, EntrySaver, EntrySaverSpec, ZipEntryReader, ZipEntrySaver}, errors::{EntryRepackError, ErrorCollector}, fop::FileType, ProgressState};

mod cli_args;
mod config;

#[cfg(not(feature = "anyhow"))]
type Error_ = io::Error;
#[cfg(feature = "anyhow")]
type Error_ = anyhow::Error;

type Result_<T> = Result<T, Error_>;

fn main() -> Result_<()> {
    let args = cli_args::Args::parse();
    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀ by Szeweq\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █ (https://szeweq.xyz/mc-repack)\n");
    
    if args.check {
        if config::check(args.config)? {
            println!("Config file is valid!");
        } else {
            println!("New config file created!");
        }
        return Ok(());
    }

    let path = args.path.as_ref().expect("Path is required");
    let repack_opts = RepackOpts::from_args(&args);
    let ftyp = path.metadata()?.file_type();
    let task: &dyn ProcessTask = if ftyp.is_dir() {
        &JarDirRepackTask
    } else if ftyp.is_file() {
        &JarRepackTask
    } else {
        return Err(TaskError::NotFileOrDir.into());
    };
    let mut ec = ErrorCollector::new(repack_opts.silent);
    task.process(path, args.out, &repack_opts, &mut ec)?;
    print_entry_errors(ec.results());
    Ok(())
}

const PB_STYLE_ZIP: &str = "# {pos}/{len} {wide_msg}";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

trait ProcessTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts, ec: &mut ErrorCollector) -> Result_<()>;
}

const TASK_ERR_MSG: &str = "Task failed";

#[cfg(not(feature = "anyhow"))]
fn task_err(_: Box<dyn Any + Send>) -> Error_ {
    io::Error::new(io::ErrorKind::Other, TASK_ERR_MSG)
}
#[cfg(feature = "anyhow")]
fn task_err(_: Box<dyn Any + Send>) -> Error_ {
    anyhow::anyhow!(TASK_ERR_MSG)
}

#[cfg(not(feature = "anyhow"))]
fn wrap_err_with(e: Error_, p: &Path) -> Error_ {
    io::Error::new(e.kind(), format!("{}: {}", p.display(), e))
}
#[cfg(feature = "anyhow")]
fn wrap_err_with(e: Error_, p: &Path) -> Error_ {
    anyhow::anyhow!("{}: {}", p.display(), e)
}

struct JarRepackTask;
impl ProcessTask for JarRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts, ec: &mut ErrorCollector) -> Result_<()> {
        let fname = if let Some(x) = fp.file_name() {
            x.to_string_lossy()
        } else {
            return Err(TaskError::InvalidFileName.into())
        };
        match FileType::by_name(&fname) {
            FileType::Other => { return Err(TaskError::NotZip.into()) }
            FileType::Repacked => { return Err(TaskError::AlreadyRepacked.into()) }
            FileType::Original => {},
        }
    
        let pb2 = file_progress_bar();
        let (pj, ps) = thread_progress_bar(pb2);
        
        let Some(nfp) = out.or_else(|| file_name_repack(fp)) else {
            return Err(TaskError::InvalidFileName.into())
        };
        optimize_with(
            ZipEntryReader::new_buf(File::open(fp)?),
            ZipEntrySaver::custom_compress(
                File::create(nfp)?,
                9 + opts.zopfli.map_or(0, |x| x.get() as i64)
            ),
            &opts.cfgmap, &ps, ec, opts.use_blacklist
        ).map_err(|e| wrap_err_with(e, fp))?;
        drop(ps);
        pj.join().map_err(task_err)?;
    
        Ok(())
    }
}

struct JarDirRepackTask;
impl ProcessTask for JarDirRepackTask {
    fn process(&self, fp: &Path, out: Option<PathBuf>, opts: &RepackOpts, ec: &mut ErrorCollector) -> Result_<()> {
        let RepackOpts { use_blacklist, .. } = *opts;
        let clvl = 9 + opts.zopfli.map_or(0, |x| x.get() as i64);
        let cfgmap = &opts.cfgmap;
        let mp = MultiProgress::new();

        let rd = fs::read_dir(fp)?;

        let pb = mp.add(ProgressBar::new_spinner().with_style(
            ProgressStyle::with_template("{wide_msg}").unwrap()
        ));
        let pb2 = mp.add(file_progress_bar());
        
        let (pj, ps) = thread_progress_bar(pb2);

        for rde in rd {
            let rde = rde?;
            let fp = rde.path();
            let rfn = rde.file_name();
            let Some(fname) = rfn.to_str() else {
                return Err(TaskError::InvalidFileName.into())
            };
            let meta = fp.metadata()?;
            if meta.is_file() && matches!(FileType::by_name(fname), FileType::Original) {
                ec.rename(fname);
                pb.set_message(fname.to_string());
                
                let Some(nfp) = new_path(out.as_ref(), &fp) else {
                    return Err(TaskError::InvalidFileName.into())
                };
                match optimize_with(
                    entry::zip::ZipEntryReader::new_buf(fs::File::open(&fp)?),
                    entry::zip::ZipEntrySaver::custom_compress(
                        fs::File::create(&nfp)?,
                        clvl
                    ),
                    cfgmap, &ps, ec, use_blacklist
                ) {
                    Ok(_) => {},
                    Err(e) => {
                        println!("Cannot repack {}: {}\n\n", fp.display(), e);
                        if let Err(fe) = fs::remove_file(&nfp) {
                            println!("Cannot remove {}: {}", nfp.display(), fe);
                        }
                    }
                }
            }
        }
        mp.clear()?;
        drop(ps);
        pj.join().map_err(task_err)?;

        Ok(())
    }
}

fn file_name_repack(p: &Path) -> Option<PathBuf> {
    let stem = p.file_stem();
    let ext = p.extension();
    match (stem, ext) {
        (Some(s), Some(e)) => {
            let mut oss = OsString::new();
            oss.push(s);
            oss.push("_repack.");
            oss.push(e);
            Some(p.with_file_name(oss))
        }
        _ => None
    }
}

fn new_path(src: Option<&PathBuf>, p: &Path) -> Option<PathBuf> {
    src.map_or_else(|| file_name_repack(p), |x| {
        p.file_name().map(|pfn| {
            let mut np = x.clone();
            np.push(pfn);
            np
        })
    })
}

fn thread_progress_bar(pb: ProgressBar) -> (JoinHandle<()>, Sender<ProgressState>) {
    let (ps, pr) = crossbeam_channel::unbounded();
    let pj = thread::spawn(move || {
        for st in pr {
            match st {
                ProgressState::Start(u) => { pb.set_length(u as u64); }
                ProgressState::Push(num, msg) => {
                    pb.set_position(num as u64);
                    pb.set_message(msg.to_string());
                }
                ProgressState::Finish => {
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
            Self::InvalidFileName => "invalid file name",
            Self::NotZip => "not a ZIP archive",
            Self::NotFileOrDir => "not a file or directory",
            Self::AlreadyRepacked => "this archive is marked as repacked, no re-repacking needed"
        })
    }
}
impl From<TaskError> for io::Error {
    fn from(val: TaskError) -> Self {
        Self::new(io::ErrorKind::Other, val)
    }
}

pub fn optimize_with<R: EntryReader + Send + 'static, S: EntrySaverSpec>(
    reader: R,
    saver: EntrySaver<S>,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    use_blacklist: bool
) -> crate::Result_<()> {
    let (tx, rx) = crossbeam_channel::bounded(8);
    let t1 = thread::spawn(move || reader.read_entries(|e| wrap_send(&tx, e), use_blacklist));
    saver.save_entries(rx, errors, cfgmap, |p| wrap_send(ps, p))?;
    t1.join().expect("Cannot join thread")?;
    Ok(())
}

const CHANNEL_CLOSED_EARLY: &str = "channel closed early";
fn wrap_send<T>(s: &Sender<T>, t: T) -> Result_<()> {
    wrap_err(s.send(t), CHANNEL_CLOSED_EARLY)
}

#[cfg(not(feature = "anyhow"))]
#[inline]
pub(crate) fn wrap_err<T, E>(r: Result<T, E>, s: &'static str) -> Result_<T> {
    r.map_err(|_| std::io::Error::new(std::io::ErrorKind::Other, s))
}

#[cfg(feature = "anyhow")]
#[inline]
pub(crate) fn wrap_err<T, E>(r: Result<T, E>, s: &'static str) -> Result_<T> {
    r.map_err(|_| anyhow::anyhow!(s))
}