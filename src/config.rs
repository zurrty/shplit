use std::{io::Write, path::PathBuf};

use serde::{Deserialize, Serialize};

pub fn config_path() -> PathBuf {
    directories::ProjectDirs::from("org", "shplit", "shplit")
        .map(|proj_dirs| proj_dirs.config_dir().to_path_buf())
        .expect("Can't locate user config directory.")
}

pub trait TomlConfig: Serialize + for<'a> Deserialize<'a> {
    fn path() -> PathBuf;
    fn load() -> Result<Self, Box<dyn std::error::Error>> {
        let path = Self::path();
        println!("{:?}", &path);
        let this: Self = toml::from_str(&std::fs::read_to_string(path)?)?;
        Ok(this)
    }
    fn save(&self) -> Result<(), Box<dyn std::error::Error>> {
        std::fs::create_dir_all(Self::path().parent().unwrap())?;
        std::fs::File::create(Self::path())?.write_all(toml::to_string_pretty(self)?.as_bytes())?;
        Ok(())
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub split_file: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self { split_file: None }
    }
}

impl TomlConfig for Config {
    fn path() -> PathBuf {
        config_path().join("config.toml")
    }
}
