//! MC-Repack is initially built as a CLI app, but can also be used as a library.
//! This crate contains methods necessary to work with files that need optimizations.
//! 
//! You should be interested in `optimizer` module. There are important methods used for repaching
//! and optimizing files.
//! 
//! This crate considers that the repacked files are used in Minecraft mods. You can still use the library
//! for other types, like Android or Gradle files.

/// Minifiers for various file types.
pub mod min;
/// File operations used for repacking.
pub mod fop;
/// Error collecting for entries.
pub mod errors;
/// Reading and saving entries (file system or ZIP archive).
pub mod entry;
/// Implementations of configuration map and traits for accepting config types.
pub mod cfg;

#[cfg(not(feature = "anyhow"))]
pub(crate) type Result_<T> = std::io::Result<T>;
#[cfg(feature = "anyhow")]
pub(crate) type Result_<T> = anyhow::Result<T>;

/// A progress state to update information about currently optimized entry
#[derive(Debug, Clone)]
pub enum ProgressState {
    /// Starts a progress with a step count
    Start(usize),
    /// Pushes a new step with text
    Push(usize, std::sync::Arc<str>),
    /// Marks a progress as finished
    Finish
}