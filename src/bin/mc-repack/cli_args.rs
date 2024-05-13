use std::path::PathBuf;

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

impl Args {
    #[inline]
    pub fn env() -> Self {
        use clap::Parser;
        Self::parse()
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