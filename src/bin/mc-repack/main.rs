use std::{any::Any, fs, path::Path, sync::Arc, thread::{self, JoinHandle}};
use clap::Parser;
use cli_args::{Cmd, FilesArgs, JarsArgs, RepackOpts};
use crossbeam_channel::Sender;
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack_core::{cfg, entry::{self, EntryReader, EntryReaderSpec, EntrySaver, EntrySaverSpec}, errors::{EntryRepackError, ErrorCollector}, fop::TypeBlacklist, ProgressState};

mod cli_args;
mod config;
mod rrd;

type Error_ = anyhow::Error;
type Result_<T> = Result<T, Error_>;

fn main() -> Result_<()> {
    let args = cli_args::Args::parse();
    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀ by Szeweq\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █ (https://szeweq.xyz/mc-repack)\n");
    
    match &args.cmd {
        Cmd::Jars(ja) => {
            let path = &ja.path;
            let repack_opts = RepackOpts::from_args(&ja.common);
            let fit = FilesIter::from_path(path)?;
            let mut ec = ErrorCollector::new(repack_opts.silent);
            process_jars(fit, ja, &repack_opts, &mut ec)?;
            print_entry_errors(ec.results());
        }
        Cmd::Files(fa) => {
            let path = &fa.path;
            let repack_opts = RepackOpts::from_args(&fa.common);
            let fit = FilesIter::from_path(path)?;
            let mut ec = ErrorCollector::new(repack_opts.silent);
            process_files(fit, fa, &repack_opts, &mut ec)?;
            print_entry_errors(ec.results());
        }
        Cmd::Check(ca) => {
            if config::check(ca.config.clone())? {
                println!("Config file is valid!");
            } else {
                println!("New config file created!");
            }
        }
    }

    Ok(())
}

const PB_STYLE_ZIP: &str = "# {pos}/{len} {wide_msg}";

fn file_progress_bar() -> ProgressBar {
    ProgressBar::new(0).with_style(
        ProgressStyle::with_template(PB_STYLE_ZIP).unwrap()
    )
}

fn task_err(_: Box<dyn Any + Send>) -> Error_ {
    anyhow::anyhow!("Task failed")
}

fn process_jars(fit: FilesIter, jargs: &JarsArgs, opts: &RepackOpts, ec: &mut ErrorCollector) -> Result_<()> {
    let RepackOpts { blacklist, cfgmap, .. } = opts;
    let clvl = 9 + jargs.zopfli.map_or(0, |x| x.get() as i64);
    let mp = MultiProgress::new();

    let base = Box::from(fit.base());
    let mut db = fs::DirBuilder::new();
    db.recursive(true);

    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(file_progress_bar());
    let (pj, ps) = thread_progress_bar(pb2);

    for fp in fit {
        let (ftype, fp) = fp?;
        let Some(fname) = fp.file_name() else {
            return invalid_file_name()
        };
        let Some(relp) = pathdiff::diff_paths(&fp, &base) else {
            return invalid_file_name()
        };
        let relname = relp.to_string_lossy();
        let fname = fname.to_string_lossy();
        if matches!(ftype, Some(false)) && matches!(relp.extension().map(|x| x.as_encoded_bytes()), Some(b"jar" | b"zip")) {
            ec.rename(&relname);
            pb.set_message(fname.to_string());
            let nfp = jargs.out.join(&relp);
            if let Some(np) = nfp.parent() {
                db.create(np)?;
            }
            match optimize_with(
                entry::ZipEntryReader::new_mem(fs::read(&fp)?)?,
                entry::ZipEntrySaver::custom_compress(
                    fs::File::create(&nfp)?,
                    jargs.keep_dirs,
                    clvl
                ),
                cfgmap, &ps, ec, blacklist.clone()
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

fn process_files(fit: FilesIter, fargs: &FilesArgs, opts: &RepackOpts, ec: &mut ErrorCollector) -> Result_<()> {
    let RepackOpts { blacklist, cfgmap, .. } = opts;
    let pb2 = file_progress_bar();
    let (pj, ps) = thread_progress_bar(pb2);
    optimize_with(
        entry::FSEntryReader::custom(fit.base().into(), fit),
        entry::FSEntrySaver::new(fargs.out.clone().into_boxed_path()),
        cfgmap, &ps, ec, blacklist.clone()
    )?;
    drop(ps);
    pj.join().map_err(task_err)?;
    Ok(())
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

pub fn optimize_with<R: EntryReaderSpec + Send + 'static, S: EntrySaverSpec>(
    reader: EntryReader<R>,
    saver: EntrySaver<S>,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    blacklist: Arc<TypeBlacklist>
) -> crate::Result_<()> {
    let (tx, rx) = crossbeam_channel::bounded(16);
    let t1 = thread::spawn(move || reader.read_entries(|e| wrap_send(&tx, e), &blacklist));
    saver.save_entries(rx, errors, cfgmap, |p| wrap_send(ps, p))?;
    t1.join().map_err(|_| anyhow::anyhow!("Cannot join thread"))??;
    Ok(())
}

fn wrap_send<T>(s: &Sender<T>, t: T) -> Result_<()> {
    s.send(t).map_err(|_| anyhow::anyhow!("channel closed early"))
}

fn invalid_file_name<T>() -> Result_<T> {
    anyhow::bail!("invalid file name")
}

enum FilesIter {
    Single(Box<Path>, std::iter::Once<(Option<bool>, Box<Path>)>),
    Dir(Box<Path>, rrd::RecursiveReadDir)
}
impl FilesIter {
    pub fn from_path(p: &Path) -> std::io::Result<Self> {
        let ft = p.metadata()?.file_type();
        let p: Box<Path> = Box::from(p);
        if ft.is_dir() {
            Ok(Self::Dir(p.clone(), rrd::RecursiveReadDir::new(p)))
        } else if ft.is_file() {
            let parent = p.parent().unwrap();
            Ok(Self::Single(parent.into(), std::iter::once((Some(false), p))))
        } else {
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Not a file or directory"))
        }
    }
    pub fn base(&self) -> &Path {
        match self {
            Self::Single(p, _) => p,
            Self::Dir(p, _) => p
        }
    }
}
impl Iterator for FilesIter {
    type Item = std::io::Result<(Option<bool>, Box<Path>)>;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(_, it) => it.next().map(Ok),
            Self::Dir(_, it) => it.next()
        }
    }
}