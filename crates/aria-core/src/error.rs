use thiserror::Error;

#[derive(Error, Debug)]
pub enum CoreError {
    #[error("UI Automation error: {0}")]
    UIAutomation(#[from] uiautomation::Error),

    #[error("TTS error: {0}")]
    TTS(String),

    #[error("Configuration error: {0}")]
    Config(String),

    #[error("Initialization error: {0}")]
    Init(&'static str),

    #[error("Synchronization error: {0}")]
    Sync(String),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("MKI error: {0}")]
    Mki(String),

    #[error("Regex error: {0}")]
    Regex(#[from] regex::Error),
}
