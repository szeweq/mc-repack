use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    /// (Required) Path to a file/directory of archives (JAR and ZIP)
    pub path: PathBuf,

    /// Use this option to optimize files from directory directly [Reserved for future use]
    #[arg(short)]
    pub g: bool,

    /// (Optional) Destination path. It cannot be the same as the source!
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Optimize more file formats (potentially breaking their debugging) [Reserved for future use]
    #[arg(short, long)]
    pub aggressive: bool,

    /// Use built-in blacklist for files
    #[arg(short = 'b', long)]
    pub use_blacklist: bool,

    /// Do not print file errors
    #[arg(long)]
    pub silent: bool
}
