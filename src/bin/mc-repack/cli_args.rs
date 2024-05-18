use std::{io, path::PathBuf};

use mc_repack_core::min;

use crate::config;


#[derive(Debug, clap::Parser)]
#[command(version)]
pub struct Args {
    /// Path to a file/directory of archives (JAR and ZIP)
    pub path: Option<PathBuf>,

    /// (Optional) Destination path. It cannot be the same as the source!
    #[arg(long)]
    pub out: Option<PathBuf>,

    /// Do not print file errors
    #[arg(long)]
    pub silent: bool,

    /// Use built-in blacklist for files
    #[arg(short = 'b', long)]
    pub use_blacklist: bool,

    /// (Optional) Use custom .toml config file. If no path is provided, it will use `mc-repack.toml`
    #[arg(short = 'c', long)]
    pub config: Option<PathBuf>,

    /// Enable Zopfli compression (better, but much slower) and apply a number of iterations
    #[arg(short = 'z', long)]
    pub zopfli: Option<std::num::NonZeroU8>,

    /// Check the config file. If it's not found, it will be created. No other tasks will be executed
    #[arg(long)]
    pub check: bool
}
pub struct RepackOpts {
    pub silent: bool,
    pub use_blacklist: bool,
    pub zopfli: Option<std::num::NonZeroU8>,
    pub cfgmap: mc_repack_core::cfg::ConfigMap
}
impl RepackOpts {
    pub fn from_args(args: &Args) -> Self {
        let cfgmap = mc_repack_core::cfg::ConfigMap::default();
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
                println!("Config loaded successfully!");
            }
            Err(e) if e.kind() != io::ErrorKind::NotFound => {
                eprintln!("Failed to read config: {e}");
            }
            _ => {}
        }
        Self {
            silent: args.silent,
            use_blacklist: args.use_blacklist,
            zopfli: args.zopfli,
            cfgmap
        }
    }
}