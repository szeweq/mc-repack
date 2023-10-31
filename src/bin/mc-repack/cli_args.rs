use std::path::PathBuf;

#[cfg(not(feature = "argh"))]
#[derive(Debug, clap::Parser)]
#[command(version)]
pub struct Args {
    /// (Required) Path to a file/directory of archives (JAR and ZIP)
    pub path: PathBuf,

    /// (Optional) Destination path. It cannot be the same as the source!
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Do not print file errors
    #[arg(long)]
    pub silent: bool,

    /// Use built-in blacklist for files
    #[arg(short = 'b', long)]
    pub use_blacklist: bool
}
#[cfg(feature = "argh")]
#[derive(argh::FromArgs)]
/// Repack jar/zip archives
pub struct Args {
    /// (Required) Path to a file/directory of archives (JAR and ZIP)
    #[argh(positional)]
    pub path: PathBuf,

    /// (Optional) Destination path. It cannot be the same as the source!
    #[argh(option)]
    pub out: Option<PathBuf>,

    /// do not print file errors
    #[argh(switch)]
    pub silent: bool,

    /// use built-in blacklist for files
    #[argh(short = 'b', switch)]
    pub use_blacklist: bool
}

#[cfg(not(feature = "argh"))]
impl Args {
    #[inline]
    pub fn env() -> Self {
        use clap::Parser;
        Self::parse()
    }
}
#[cfg(feature = "argh")]
impl Args {
    #[inline]
    pub fn env() -> Self {
        argh::from_env()
    }
}

pub struct RepackOpts {
    pub silent: bool,
    pub use_blacklist: bool
}
impl RepackOpts {
    pub fn from_args(args: &Args) -> Self {
        Self {
            silent: args.silent,
            use_blacklist: args.use_blacklist
        }
    }
}