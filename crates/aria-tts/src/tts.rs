use crate::error::TTSError;
use aria_utils::config::get_config;
use aria_utils::error::ConfigError as AriaUtilsConfigError;
use log::warn;
use once_cell::sync::Lazy;
use tokio::sync::{Mutex, OnceCell as TokioOnceCell};
use windows::{
    core::HSTRING,
    Foundation::IAsyncOperation,
    Media::{
        Core::MediaSource,
        Playback::MediaPlayer,
        SpeechSynthesis::{SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesizer},
    },
};

type Result<T> = std::result::Result<T, TTSError>;

async fn create_and_configure_synthesizer() -> Result<SpeechSynthesizer> {
    let synthesizer = SpeechSynthesizer::new().map_err(TTSError::Windows)?;
    let synthesizer_options = synthesizer.Options().map_err(TTSError::Windows)?;
    let config = get_config().map_err(|e: AriaUtilsConfigError| match e {
        AriaUtilsConfigError::Io(io_err) => TTSError::Windows(windows::core::Error::from(io_err)),
        AriaUtilsConfigError::TomlSer(toml_err) => {
            TTSError::Synthesis(format!("Config TOML error: {}", toml_err))
        }
        AriaUtilsConfigError::Lib(lib_err) => {
            TTSError::Synthesis(format!("Config lib error: {}", lib_err))
        }
        AriaUtilsConfigError::HomeDir => {
            TTSError::Synthesis("Failed to get home directory for config".to_string())
        }
        AriaUtilsConfigError::PathToStr { path } => {
            TTSError::Synthesis(format!("Config path error for {:?}", path))
        }
    })?;

    synthesizer_options
        .SetSpeakingRate(config.speech_rate)
        .map_err(TTSError::Windows)?;
    synthesizer_options
        .SetAppendedSilence(if config.append_silence {
            SpeechAppendedSilence::Default
        } else {
            SpeechAppendedSilence::Min
        })
        .map_err(TTSError::Windows)?;
    synthesizer_options
        .SetPunctuationSilence(if config.punctuation_silence {
            SpeechPunctuationSilence::Default
        } else {
            SpeechPunctuationSilence::Min
        })
        .map_err(TTSError::Windows)?;
    Ok(synthesizer)
}

async fn create_media_player() -> Result<MediaPlayer> {
    MediaPlayer::new().map_err(TTSError::Windows)
}

static SYNTHESIZER: TokioOnceCell<SpeechSynthesizer> = TokioOnceCell::const_new();
static PLAYER: TokioOnceCell<MediaPlayer> = TokioOnceCell::const_new();

static CAN_STOP: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
static CAN_SPEAK: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(true));

async fn get_synthesizer() -> Result<&'static SpeechSynthesizer> {
    SYNTHESIZER
        .get_or_try_init(create_and_configure_synthesizer)
        .await
}

async fn get_player() -> Result<&'static MediaPlayer> {
    PLAYER.get_or_try_init(create_media_player).await
}

async fn await_windows_async<T, F>(op_factory: F) -> Result<T>
where
    T: windows::core::RuntimeType + Send + 'static,
    F: FnOnce() -> windows::core::Result<IAsyncOperation<T>> + Send + 'static,
    IAsyncOperation<T>: Send + Sync,
{
    let operation = tokio::task::spawn_blocking(op_factory)
        .await
        .map_err(|e| TTSError::Synthesis(format!("Task spawn join error: {}", e)))??;

    tokio::task::spawn_blocking(move || operation.get())
        .await
        .map_err(|e| TTSError::Synthesis(format!("Task spawn for .get() error: {}", e)))?
        .map_err(TTSError::Windows)
}

pub struct TTS;

impl TTS {
    pub async fn speak(text: &str, ignore_can_speak: bool) -> Result<()> {
        log::info!("{}", text);

        let can_speak_guard = CAN_SPEAK.lock().await;
        if !*can_speak_guard && !ignore_can_speak {
            warn!("Cannot speak: speaking is disabled");
            return Ok(());
        }
        drop(can_speak_guard);

        let synthesizer = get_synthesizer().await?;
        let player = get_player().await?;

        let text_hstring = HSTRING::from(text);

        let stream =
            await_windows_async(move || synthesizer.SynthesizeTextToStreamAsync(&text_hstring))
                .await?;

        let content_type = stream.ContentType().map_err(TTSError::Windows)?;
        let media_source =
            MediaSource::CreateFromStream(&stream, &content_type).map_err(TTSError::Windows)?;

        player.SetSource(&media_source).map_err(TTSError::Windows)?;
        player.Play().map_err(TTSError::Windows)?;

        Ok(())
    }

    pub async fn set_can_stop(can_stop: bool) -> Result<()> {
        let mut can_stop_lock = CAN_STOP.lock().await;
        *can_stop_lock = can_stop;
        Ok(())
    }

    pub async fn set_can_speak(can_speak: bool) -> Result<()> {
        let mut can_speak_lock = CAN_SPEAK.lock().await;
        *can_speak_lock = can_speak;
        Ok(())
    }

    pub async fn stop(ignore_can_stop: bool) -> Result<()> {
        let can_stop_guard = CAN_STOP.lock().await;
        if !*can_stop_guard && !ignore_can_stop {
            warn!("Cannot stop: stopping is disabled");
            return Ok(());
        }
        drop(can_stop_guard);

        if let Some(player) = PLAYER.get() {
            player.Pause().map_err(TTSError::Windows)?;
        } else {
            log::info!("Player not initialized, nothing to stop.");
        }
        Ok(())
    }

    pub async fn destroy() -> Result<()> {
        if let Some(player) = PLAYER.get() {
            player.Close().map_err(TTSError::Windows)?;
        }

        log::info!("TTS resources associated with static player are closed (if initialized).");
        Ok(())
    }
}
