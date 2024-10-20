use std::{fs, path::PathBuf};

use config::{Config, ConfigError};
use serde::{Deserialize, Serialize};

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

pub fn get_config_path() -> PathBuf {
    let overridden_path = std::env::var("ARIA_PATH").ok();

    if let Some(path) = overridden_path {
        return PathBuf::from(path);
    }

    dirs::home_dir()
        .map(|mut path| {
            path.push(".config/aria/aria.toml");
            path
        })
        .unwrap_or(PathBuf::from("aria.toml"))
}

pub fn create_default_config(path: &PathBuf) -> Result<(), std::io::Error> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    let default_config = AriaConfig::default();
    let toml = toml::to_string(&default_config).expect("Failed to serialize default config");

    fs::write(path, toml)
}

pub fn get_config() -> Result<AriaConfig, ConfigError> {
    let config_path = get_config_path();

    if !config_path.exists() {
        create_default_config(&config_path).expect("Failed to create default config file");
    }

    let config = Config::builder()
        // Add in `./aria.toml`
        .add_source(config::File::with_name(config_path.to_str().unwrap()).required(false))
        // Add in settings from the environment (with a prefix of ARIA)
        // Eg.. `ARIA_DEBUG=1 ./target/app` would set the `debug` key
        .add_source(config::Environment::with_prefix("ARIA"))
        // Set default values
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
        .build()
        .unwrap();

    return config.try_deserialize::<AriaConfig>();
}
