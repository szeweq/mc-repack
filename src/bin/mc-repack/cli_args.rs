use std::path::PathBuf;

use clap::Parser;

#[derive(Debug, Parser)]
#[command(version)]
pub struct Args {
    /// (Optional) Path to a file/directory of archives (JAR and ZIP)
    pub path: Option<PathBuf>,

    /// Use this option to optimize files from directory directly [Reserved for future use]
    #[arg(short)]
    pub g: bool,

    /// Use this option to pack optimized files into a JAR/ZIP file (only works with -g)
    #[arg(short)]
    pub z: bool,

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

impl Args {
    pub fn actual_path(&self) -> PathBuf {
        self.path.clone().unwrap_or_else(|| {
            use dialoguer::{theme::ColorfulTheme, Input};
            let fstr: String = Input::with_theme(&ColorfulTheme::default())
                .with_prompt("Path to a file/directory").interact_text().unwrap();
            PathBuf::from(fstr)
        })
    }
}