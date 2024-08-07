use std::{fs, path::Path, sync::Arc, io};
use clap::Parser;
use cli_args::{Cmd, FilesArgs, JarsArgs, RepackOpts};
use crossbeam_channel::{Receiver, Sender};
use indicatif::{ProgressBar, ProgressStyle, MultiProgress};

use mc_repack_core::{cfg, entry::{self, process_entry, read_entry, EntryReader, EntrySaver, NamedEntry, ReadEntryIter}, errors::ErrorCollector, fop::TypeBlacklist, ProgressState};

mod cli_args;
mod config;
mod report;

type Error_ = anyhow::Error;
type Result_<T> = Result<T, Error_>;

fn main() -> Result_<()> {
    let args = cli_args::Args::parse();
    println!("█▀▄▀█ █▀▀ ▄▄ █▀█ █▀▀ █▀█ ▄▀█ █▀▀ █▄▀ by Szeweq\n█ ▀ █ █▄▄    █▀▄ ██▄ █▀▀ █▀█ █▄▄ █ █ (https://szeweq.xyz/mc-repack)\n");
    
    match &args.cmd {
        Cmd::Jars(ja) => {
            let path = &ja.path;
            let mut repack_opts = RepackOpts::from_args(&ja.common);
            let (base, fit) = FilesIter::from_path(path)?;
            process_jars(&base, fit, ja, &mut repack_opts)?;
            if let Some(ref report) = repack_opts.report {
                report.save_csv()?;
            }
            print_entry_errors(&repack_opts.err_collect);
        }
        Cmd::Files(fa) => {
            let path = &fa.path;
            let mut repack_opts = RepackOpts::from_args(&fa.common);
            let (base, fit) = FilesIter::from_path(path)?;
            process_files(&base, fit, fa, &mut repack_opts)?;
            if let Some(ref mut report) = repack_opts.report {
                files_report(report, path, &fa.out)?;
                report.save_csv()?;
            }
            print_entry_errors(&repack_opts.err_collect);
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

fn process_jars(base: &Path, fit: FilesIter, jargs: &JarsArgs, opts: &mut RepackOpts) -> Result_<()> {
    let RepackOpts { ref blacklist, ref cfgmap, .. } = opts;
    let ec = &mut opts.err_collect;
    let clvl = 9 + jargs.zopfli.map_or(0, |x| x.get() as i64);
    let mp = MultiProgress::new();

    let mut db = fs::DirBuilder::new();
    db.recursive(true);

    let pb = mp.add(ProgressBar::new_spinner().with_style(
        ProgressStyle::with_template("{wide_msg}").unwrap()
    ));
    let pb2 = mp.add(file_progress_bar());
    let ps = thread_progress_bar(pb2);

    for fp in fit {
        let (ftype, fp) = fp?;
        let Some(fname) = fp.file_name() else {
            return invalid_file_name()
        };
        let Some(relp) = pathdiff::diff_paths(&fp, base) else {
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
                &mut entry::ZipEntryReader::new_mem(fs::read(&fp)?)?,
                &mut entry::ZipEntrySaver::custom_compress(
                    fs::File::create(&nfp)?,
                    jargs.keep_dirs,
                    clvl
                ),
                cfgmap, &ps, ec, blacklist.clone()
            ) {
                Ok(()) => {
                    if let Some(ref mut report) = opts.report {
                        report_sizes(report, &relname, &fp, &nfp);
                    }
                },
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

    Ok(())
}

fn process_files(base: &Path, fit: FilesIter, fargs: &FilesArgs, opts: &mut RepackOpts) -> Result_<()> {
    let RepackOpts { ref blacklist, ref cfgmap, .. } = opts;
    let ec = &mut opts.err_collect;
    let pb2 = file_progress_bar();
    let ps = thread_progress_bar(pb2);
    optimize_with(
        &mut entry::FSEntryReader::custom(base.into(), fit),
        &mut entry::FSEntrySaver::new(fargs.out.clone().into_boxed_path()),
        cfgmap, &ps, ec, blacklist.clone()
    )?;
    drop(ps);
    Ok(())
}

fn files_report(report: &mut report::Report, path: &Path, out: &Path) -> Result_<()> {
    let (base, fit) = FilesIter::from_path(path)?;
    for fp in fit {
        let (ftype, fp) = fp?;
        let Some(relp) = pathdiff::diff_paths(&fp, &base) else {
            return invalid_file_name()
        };
        let relname = relp.to_string_lossy();
        if matches!(ftype, Some(false)) {
            let nfp = out.join(&relp);
            report_sizes(report, &relname, &fp, &nfp);
        }
    }
    Ok(())
}

fn report_sizes(report: &mut report::Report, relname: &str, fp: &Path, nfp: &Path) {
    match (fs::metadata(fp), fs::metadata(nfp)) {
        (Ok(fm), Ok(nm)) => {
            report.push(relname, fm.len(), nm.len());
        }
        (Err(e), Ok(_)) | (Ok(_), Err(e)) => {
            println!("Cannot report {}: {}", relname, e);
        }
        (Err(e1), Err(e2)) => {
            println!("Cannot report {}: {}, {}", relname, e1, e2);
        }
    }
}

fn thread_progress_bar(pb: ProgressBar) -> Sender<ProgressState> {
    let (ps, pr) = crossbeam_channel::unbounded();
    rayon::spawn(move || {
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
    ps
}

fn print_entry_errors(ec: &ErrorCollector) {
    let v = ec.results();
    if !v.is_empty() {
        eprintln!("Errors found in files:");
        for ere in v {
            eprintln!(" # {ere}");
        }
    }
}

pub fn optimize_with<R: EntryReader + Send + 'static, S: EntrySaver + Send + 'static>(
    reader: &mut R,
    saver: &mut S,
    cfgmap: &cfg::ConfigMap,
    ps: &Sender<ProgressState>,
    errors: &mut ErrorCollector,
    blacklist: Arc<TypeBlacklist>
) -> crate::Result_<()> {
    let (tx, rx) = crossbeam_channel::unbounded();
    wrap_send(ps, ProgressState::Start(reader.read_len()))?;
    let mut r1 = anyhow::Ok(());
    let mut r2 = anyhow::Ok(());
    rayon::scope_fifo(|s| {
        let r1 = &mut r1;
        let r2 = &mut r2;
        s.spawn_fifo(move |_| {
            //let mut reader = reader;
            *r1 = reading(reader.read_iter(), tx, blacklist)
        });
        s.spawn_fifo(move |_| {
            //let mut saver = saver;
            *r2 = saving(saver, rx, ps, errors, cfgmap)
        });
    });
    match (r1, r2) {
        (Ok(_), Ok(_)) => Ok(()),
        (Err(e), Ok(_)) | (Ok(_), Err(e)) => Err(e),
        (Err(e1), Err(e2)) => Err(anyhow::anyhow!("Two errors: {} {}", e1, e2)),
    }
}

fn reading<R: EntryReader>(iter: ReadEntryIter<R>, tx: Sender<NamedEntry>, blacklist: Arc<TypeBlacklist>) -> Result_<()> {
    for re in iter {
        if let Some(ne) = read_entry::<R>(re, &blacklist)? {
            wrap_send(&tx, ne)?;
        };
    }
    Ok(())
}
fn saving<S: EntrySaver>(saver: &mut S, rx: Receiver<NamedEntry>, ps: &Sender<ProgressState>, errors: &mut ErrorCollector, cfgmap: &cfg::ConfigMap) -> Result_<()> {
    let mut cv = Vec::new();
    for (n, ne) in rx.into_iter().enumerate() {
        wrap_send(ps, ProgressState::Push(n, ne.0.clone()))?;
        if let Some(se) = process_entry(&mut cv, &ne, errors, cfgmap) {
            saver.save(&ne.0, se)?;
            if !cv.is_empty() {
                cv.clear();
            }
        }
    }
    wrap_send(ps, ProgressState::Finish)
}

fn wrap_send<T>(s: &Sender<T>, t: T) -> Result_<()> {
    s.send(t).map_err(|_| anyhow::anyhow!("channel closed early"))
}

fn invalid_file_name<T>() -> Result_<T> {
    anyhow::bail!("invalid file name")
}

type FileResult = io::Result<(Option<bool>, Box<Path>)>;

enum FilesIter {
    Single(Option<FileResult>),
    Dir(std::vec::IntoIter<FileResult>)
}
impl FilesIter {
    pub fn from_path(p: &Path) -> io::Result<(Box<Path>, Self)> {
        let ft = p.metadata()?.file_type();
        let p: Box<Path> = Box::from(p);
        if ft.is_dir() {
            Ok((p.clone(), Self::Dir(walkdir::WalkDir::new(p).into_iter().map(|r| Ok(check_dir_entry(r?))).collect::<Vec<_>>().into_iter())))
        } else if ft.is_file() {
            let parent = p.parent().unwrap();
            Ok((parent.into(), Self::Single(Some(Ok((Some(false), p))))))
        } else {
            Err(io::Error::new(io::ErrorKind::Other, "Not a file or directory"))
        }
    }
}
impl Iterator for FilesIter {
    type Item = FileResult;
    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Single(it) => it.take(),
            Self::Dir(it) => it.next()
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Single(Some(_)) => (1, Some(1)),
            Self::Single(None) => (0, Some(0)),
            Self::Dir(it) => it.size_hint()
        }
    }
}
impl ExactSizeIterator for FilesIter {
    fn len(&self) -> usize {
        match self {
            Self::Single(Some(_)) => 1,
            Self::Single(None) => 0,
            Self::Dir(it) => it.len()
        }
    }
}

fn check_dir_entry(de: walkdir::DirEntry) -> (Option<bool>, Box<Path>) {
    let ft = de.file_type();
    let p = de.into_path().into_boxed_path();
    (if ft.is_dir() {
        Some(true)
    } else if ft.is_file() {
        Some(false)
    } else {
        None
    }, p)
}