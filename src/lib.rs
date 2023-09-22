#![warn(clippy::nursery)]
#![warn(missing_docs)]
//! MC-Repack is initially built as a CLI app, but can also be used as a library.
//! This crate contains methods necessary to work with files that need optimizations.
//! 
//! You should be interested in `optimizer` module. There are important methods used for repaching
//! and optimizing files.
//! 
//! This crate considers that the repacked files are used in Minecraft mods. You can still use the library
//! for other types, like Android or Gradle files.

/// Minifiers for various file types.
pub mod minify;
/// Optimizer (file system or ZIP archive).
pub mod optimizer;
/// File operations used for repacking.
pub mod fop;
/// Error collecting for entries.
pub mod errors;
/// Reading and saving entries (file system or ZIP archive).
pub mod entry;