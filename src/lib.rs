#![warn(missing_docs)]
//! MC-Repack can also be used as a library. This crate contains methods necessary to work with files
//! that need optimizations.

/// Minifiers.
pub mod minify;
/// Optimizer.
pub mod optimizer;
/// Blacklisted files.
pub mod blacklist;
/// File operations.
pub mod fop;
/// Error collecting.
pub mod errors;