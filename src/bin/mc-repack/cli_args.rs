use std::{io, path::PathBuf, sync::Arc};

use mc_repack_core::{fop::TypeBlacklist, min};

use crate::config;


#[derive(Debug, clap::Parser)]
#[command(version)]
pub struct Args {
    #[command(subcommand)]
    pub cmd: Cmd
}


#[derive(Debug, clap::Subcommand)]
pub enum Cmd {
    /// Repack archives
    Jars(JarsArgs),

    /// Optimize files
    Files(FilesArgs),

    /// Check the config file
    Check(CommonArgs)
}

#[derive(Debug, clap::Args)]
pub struct JarsArgs {
    /// Path to a file/directory of archives (JAR and ZIP)
    #[arg(short = 'i', long = "in")]
    pub path: PathBuf,

    /// Destination directory. It should not be the same as the source!
    #[arg(short = 'o', long)]
    pub out: PathBuf,

    /// Enable Zopfli compression (better, but much slower) and apply a number of iterations
    #[arg(short = 'z', long)]
    pub zopfli: Option<std::num::NonZeroU8>,

    /// Keep directory entries in the archive
    #[arg(short = 'd', long)]
    pub keep_dirs: bool,

    #[command(flatten)]
    pub common: CommonArgs
}

#[derive(Debug, clap::Args)]
pub struct FilesArgs {
    /// Path to a file/directory
    pub path: PathBuf,

    /// Destination directory. It should not be the same as the source!
    #[arg(long)]
    pub out: PathBuf,

    #[command(flatten)]
    pub common: CommonArgs
}

#[derive(Debug, clap::Args)]
pub struct CommonArgs {
    /// Do not print file errors
    #[arg(long)]
    pub silent: bool,

    /// Add built-in blacklist rules for files. This works separate from the config file
    #[arg(short = 'b', long)]
    pub use_blacklist: bool,

    /// (Optional) Use custom .toml config file. If no path is provided, it will use `mc-repack.toml`
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>
}

pub struct RepackOpts {
    pub silent: bool,
    pub blacklist: Arc<TypeBlacklist>,
    pub cfgmap: mc_repack_core::cfg::ConfigMap
}
impl RepackOpts {
    pub fn from_args(args: &CommonArgs) -> Self {
        let cfgmap = mc_repack_core::cfg::ConfigMap::default();
        let mut blacklist = None;
        match config::read_config(args.config.clone()) {
            Ok(c) => {
                if let Some(x) = c.json {
                    cfgmap.set::<min::json::MinifierJSON>(x);
                }
                if let Some(x) = c.nbt {
                    cfgmap.set::<min::nbt::MinifierNBT>(x);
                }
                if let Some(x) = c.png {
                    cfgmap.set::<min::png::MinifierPNG>(x);
                }
                if let Some(x) = c.toml {
                    cfgmap.set::<min::toml::MinifierTOML>(x);
                }
                blacklist = c.blacklist;
                println!("Config loaded successfully!");
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                eprintln!("Failed to read config: {e}");
            }
            _ => {}
        }
        Self {
            silent: args.silent,
            blacklist: Arc::new(if args.use_blacklist {
                TypeBlacklist::Extend(blacklist)
            } else {
                TypeBlacklist::Override(blacklist)
            }),
            cfgmap
        }
    }
}