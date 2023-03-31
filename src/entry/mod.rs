
/// Entry reader and saver for a file system.
pub mod fs;
/// Entry reader and saver for ZIP archives.
pub mod zip;

use std::io;
use crate::{optimizer::{EntryType, ProgressState}, errors::ErrorCollector};

use crossbeam_channel::{Sender, Receiver};

/// Trait for reading entries for further optimization. Typically used with `EntrySaver`.
pub trait EntryReader {
    /// Reads entries and sends them via `tx`.
    /// The `use_blacklist` parameter is used to ignore predefined file types written in `blacklist` module.
    fn read_entries(
        self,
        tx: Sender<EntryType>,
        use_blacklist: bool
    ) -> io::Result<()>;
}

/// Trait for saving entries that have been optimizer. Typically used with `EntryReader`.
pub trait EntrySaver {
    /// Receives entries from `rx`, optimizes, sends progress (via `ps`), and saves them.
    /// Errors are collected with entry names.
    fn save_entries(
        self,
        rx: Receiver<EntryType>,
        ev: &mut dyn ErrorCollector,
        ps: &Sender<ProgressState>
    ) -> io::Result<i64>;
}