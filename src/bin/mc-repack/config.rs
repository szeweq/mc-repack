use std::{collections::HashSet, fs, io, path::PathBuf};

use mc_repack_core::min;
use crate::Result_;

#[derive(serde::Deserialize, serde::Serialize)]
pub struct Config {
    pub json: Option<min::json::JSONConfig>,
    pub nbt: Option<min::nbt::NBTConfig>,
    pub png: Option<min::png::PNGConfig>,
    pub toml: Option<min::toml::TOMLConfig>,
    pub jar: Option<min::jar::JARConfig>,
    pub blacklist: Option<HashSet<Box<str>>>
}
impl Config {
    pub fn read(path: Option<PathBuf>) -> io::Result<Self> {
        let path = path_to_config(path)?;
        let f = fs::read_to_string(&path)?;
        toml::from_str(&f).map_err(io::Error::other)
    }
}
impl Default for Config {
    fn default() -> Self {
        Self {
            json: Some(min::json::JSONConfig::default()),
            nbt: Some(min::nbt::NBTConfig::default()),
            png: Some(min::png::PNGConfig::default()),
            toml: Some(min::toml::TOMLConfig::default()),
            jar: Some(min::jar::JARConfig::default()),
            blacklist: Some(HashSet::new())
        }
    }
}

fn path_to_config(path: Option<PathBuf>) -> io::Result<PathBuf> {
    match path {
        Some(p) => {
            let meta = fs::metadata(&p)?;
            Ok(if meta.is_dir() {
                p.join("mc-repack.toml")
            } else {
                p
            })
        }
        None => Ok(PathBuf::from("mc-repack.toml"))
    }
}

pub fn check(path: Option<PathBuf>) -> Result_<bool> {
    let path = path_to_config(path)?;
    let f = match fs::read_to_string(&path) {
        Ok(f) => f,
        Err(e) if e.kind() == io::ErrorKind::NotFound => {
            let cfg = Config::default();
            let s = toml::to_string(&cfg).map_err(io::Error::other)?;
            fs::write(path, s)?;
            return Ok(false);
        }
        Err(e) => return Err(e.into())
    };
    toml::from_str::<Config>(&f).map_err(io::Error::other)?;
    Ok(true)
}