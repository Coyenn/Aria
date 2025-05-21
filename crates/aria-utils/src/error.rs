use config::ConfigError as LibConfigError;
use std::{io, path::PathBuf};
use thiserror::Error;

#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("IO error: {0}")]
    Io(#[from] io::Error),

    #[error("TOML serialization error: {0}")]
    TomlSer(#[from] toml::ser::Error),

    #[error("Config library error: {0}")]
    Lib(#[from] LibConfigError),

    #[error("Failed to get home directory")]
    HomeDir,

    #[error("Path to string conversion failed for: {path:?}")]
    PathToStr { path: PathBuf },
}

pub type Result<T> = std::result::Result<T, ConfigError>;
