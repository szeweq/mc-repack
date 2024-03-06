
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use crate::{errors::ErrorCollector, fop::FileOp, optimizer::{EntryType, ProgressState}};

use crossbeam_channel::{Sender, Receiver};

/// Trait for reading entries for further optimization. Typically used with `EntrySaver`.
/// Any function that matches these arguments (excluding self) can be used as custom entry reader.
pub trait EntryReader {
    /// Reads entries and sends them via `tx`.
    /// The `use_blacklist` parameter is used to ignore predefined file types written in `blacklist` module.
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> crate::Result_<()>;
}

/// A struct for saving entries that have been optimized. Typically used with `EntryReader`.
/// Any function that matches these arguments (excluding self) can be used as custom entry saver.
pub struct EntrySaver<S: EntrySaverSpec>(S);

/// Saves entries in a file-based system.
pub trait EntrySaverSpec {
    /// Saves a directory.
    fn save_dir(&mut self, dir: &str) -> crate::Result_<()>;
    /// Saves a file with a minimum file size constraint for compression.
    fn save_file(&mut self, fname: &str, buf: &[u8], min_compress: u16) -> crate::Result_<()>;
    
}
impl<T: EntrySaverSpec> EntrySaver<T> {
    /// Receives entries from `rx`, optimizes, sends progress (via `ps`), and saves them.
    /// Errors are collected with entry names.
    pub fn save_entries(
        mut self,
        rx: Receiver<EntryType>,
        ev: &mut ErrorCollector,
        ps: &Sender<ProgressState>
    ) -> crate::Result_<()> {
        let mut cv = Vec::new();
        let mut n = 0;
        for et in rx {
            match et {
                EntryType::Count(u) => {
                    wrap_send(ps, ProgressState::Start(u))?;
                }
                EntryType::Directory(dir) => {
                    self.0.save_dir(&dir)?;
                }
                EntryType::File(fname, buf, fop) => {
                    wrap_send(ps, ProgressState::Push(n, fname.clone()))?;
                    n += 1;
                    match fop {
                        FileOp::Ignore(e) => {
                            ev.collect(fname.clone(), e.into());
                        }
                        FileOp::Minify(m) => {
                            let buf: &[u8] = match m.minify(&buf, &mut cv) {
                                Ok(()) => &cv,
                                Err(e) => {
                                    ev.collect(fname.clone(), e);
                                    &buf
                                }
                            };
                            self.0.save_file(&fname, buf, m.compress_min())?;
                            cv.clear();
                        }
                        FileOp::Recompress(x) => {
                            self.0.save_file(&fname, &buf, x as u16)?;
                        }
                    }
                }
            }
        }
        wrap_send(ps, ProgressState::Finish)
    }
}

impl<T: FnOnce(Sender<EntryType>, bool) -> crate::Result_<()>> EntryReader for T {
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> crate::Result_<()> {
        self(tx, use_blacklist)
    }
}

const CHANNEL_CLOSED_EARLY: &str = "channel closed early";
fn wrap_send<T>(s: &Sender<T>, t: T) -> crate::Result_<()> {
    crate::wrap_err(s.send(t), CHANNEL_CLOSED_EARLY)
}