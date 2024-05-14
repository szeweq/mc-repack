use std::{io, path::PathBuf};

use mc_repack_core::min;
use crate::Result_;

#[derive(serde::Deserialize)]
pub struct Config {
    pub json: Option<min::json::JSONConfig>,
    pub nbt: Option<min::nbt::NBTConfig>,
    pub png: Option<min::png::PNGConfig>,
    pub toml: Option<min::toml::TOMLConfig>
}

pub fn read_config(path: Option<PathBuf>) -> Result_<Config> {
    let path = match path {
        Some(p) => {
            let meta = std::fs::metadata(&p)?;
            if meta.is_dir() {
                p.join("mc-repack.toml")
            } else {
                p
            }
        }
        None => PathBuf::from("mc-repack.toml")
    };
    let f = std::fs::read_to_string(path)?;
    toml::from_str(&f).map_err(io::Error::other)
}