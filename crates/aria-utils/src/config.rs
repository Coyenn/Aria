use std::{fs, path::PathBuf};

use config::Config as ConfigLib;
use serde::{Deserialize, Serialize};

use crate::error::{ConfigError, Result};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub struct AriaConfig {
    pub speech_rate: f64,
    pub pitch: f64,
    pub append_silence: bool,
    pub punctuation_silence: bool,
    pub startup_shutdown_sounds: bool,
}

impl Default for AriaConfig {
    fn default() -> Self {
        AriaConfig {
            speech_rate: 1.0,
            pitch: 1.0,
            append_silence: true,
            punctuation_silence: true,
            startup_shutdown_sounds: true,
        }
    }
}

pub fn get_config_path() -> Result<PathBuf> {
    if let Ok(path_str) = std::env::var("ARIA_PATH") {
        return Ok(PathBuf::from(path_str));
    }

    dirs::home_dir()
        .map(|mut path| {
            path.push(".config/aria/aria.toml");
            path
        })
        .ok_or(ConfigError::HomeDir)
}

pub fn create_default_config(path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let default_config = AriaConfig::default();
    let toml_string = toml::to_string(&default_config)?;

    fs::write(path, toml_string)?;
    Ok(())
}

pub fn get_config() -> Result<AriaConfig> {
    let config_path = get_config_path()?;

    if !config_path.exists() {
        create_default_config(&config_path)?;
    }

    let config_path_str = config_path.to_str().ok_or_else(|| ConfigError::PathToStr {
        path: config_path.clone(),
    })?;

    let settings = ConfigLib::builder()
        .add_source(config::File::with_name(config_path_str).required(false))
        .add_source(config::Environment::with_prefix("ARIA"))
        .set_default("speech_rate", AriaConfig::default().speech_rate)?
        .set_default("pitch", AriaConfig::default().pitch)?
        .set_default("append_silence", AriaConfig::default().append_silence)?
        .set_default(
            "punctuation_silence",
            AriaConfig::default().punctuation_silence,
        )?
        .set_default(
            "startup_shutdown_sounds",
            AriaConfig::default().startup_shutdown_sounds,
        )?
        .build()?;

    Ok(settings.try_deserialize::<AriaConfig>()?)
}
