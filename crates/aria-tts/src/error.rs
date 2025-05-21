use thiserror::Error;

#[derive(Error, Debug)]
pub enum TTSError {
    #[error("Windows API error: {0}")]
    Windows(#[from] windows::core::Error),

    #[error("Speech synthesis failed: {0}")]
    Synthesis(String),

    #[error("Media player error: {0}")]
    MediaPlayer(String),

    #[error("Initialization error: {0}")]
    Init(&'static str),

    #[error("Operation forbidden: {0}")]
    Forbidden(String),

    #[error("Synchronization error: {0}")]
    Sync(String),

    #[error("TTS not initialized or already destroyed")]
    NotInitialized,
}
