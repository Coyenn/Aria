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
        SpeechSynthesis::{
            SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesizer, VoiceInformation,
        },
    },
};

#[derive(Debug, Clone)]
pub struct VoiceInfo {
    pub id: String,
    pub display_name: String,
    pub language: String,
    pub gender: String,
}

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

    // Set voice if specified in config
    if let Some(voice_name) = &config.tts_voice {
        if let Ok(Some(voice)) = find_voice_by_name(voice_name).await {
            synthesizer.SetVoice(&voice).map_err(TTSError::Windows)?;
        }
    }

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

async fn get_installed_voices() -> Result<Vec<VoiceInfo>> {
    tokio::task::spawn_blocking(|| {
        let voices = SpeechSynthesizer::AllVoices().map_err(TTSError::Windows)?;

        let mut voice_list = Vec::new();
        let voice_count = voices.Size().map_err(TTSError::Windows)?;

        for i in 0..voice_count {
            if let Ok(voice) = voices.GetAt(i) {
                let display_name = voice.DisplayName().map_err(TTSError::Windows)?.to_string();
                let id = voice.Id().map_err(TTSError::Windows)?.to_string();
                let language = voice.Language().map_err(TTSError::Windows)?.to_string();
                let gender = match voice.Gender().map_err(TTSError::Windows)? {
                    windows::Media::SpeechSynthesis::VoiceGender::Male => "Male".to_string(),
                    windows::Media::SpeechSynthesis::VoiceGender::Female => "Female".to_string(),
                    _ => "Unknown".to_string(),
                };

                voice_list.push(VoiceInfo {
                    id,
                    display_name,
                    language,
                    gender,
                });
            }
        }

        Ok(voice_list)
    })
    .await
    .map_err(|e| TTSError::Synthesis(format!("Task spawn error: {}", e)))?
}

async fn find_voice_by_name(voice_name: &str) -> Result<Option<VoiceInformation>> {
    let voice_name = voice_name.to_string();
    tokio::task::spawn_blocking(move || {
        let voices = SpeechSynthesizer::AllVoices().map_err(TTSError::Windows)?;
        let voice_count = voices.Size().map_err(TTSError::Windows)?;

        for i in 0..voice_count {
            if let Ok(voice) = voices.GetAt(i) {
                let display_name = voice.DisplayName().map_err(TTSError::Windows)?.to_string();
                if display_name == voice_name {
                    return Ok(Some(voice));
                }
            }
        }

        Ok(None)
    })
    .await
    .map_err(|e| TTSError::Synthesis(format!("Task spawn error: {}", e)))?
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

    /// Speak text and wait for completion
    pub async fn speak_and_wait(text: &str, ignore_can_speak: bool) -> Result<()> {
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

        // Wait for playback to complete
        Self::wait_for_playback_completion(player).await?;

        Ok(())
    }

    /// Wait for media player to finish playing
    async fn wait_for_playback_completion(player: &MediaPlayer) -> Result<()> {
        use windows::Media::Playback::MediaPlaybackState;

        // Poll the player state until it's no longer playing
        loop {
            let state = tokio::task::spawn_blocking({
                let player = player.clone();
                move || player.PlaybackSession()?.PlaybackState()
            })
            .await
            .map_err(|e| TTSError::Synthesis(format!("Task spawn error: {}", e)))?
            .map_err(TTSError::Windows)?;

            match state {
                MediaPlaybackState::Playing => {
                    // Still playing, wait a bit and check again
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
                MediaPlaybackState::Paused | MediaPlaybackState::None => {
                    // Playback finished
                    break;
                }
                _ => {
                    // Other states, wait a bit and check again
                    tokio::time::sleep(tokio::time::Duration::from_millis(100)).await;
                }
            }
        }

        Ok(())
    }

    /// Get a list of all installed TTS voices
    pub async fn get_available_voices() -> Result<Vec<VoiceInfo>> {
        get_installed_voices().await
    }

    /// Set the current TTS voice by name
    pub async fn set_voice(voice_name: &str) -> Result<bool> {
        let synthesizer = get_synthesizer().await?;

        if let Ok(Some(voice)) = find_voice_by_name(voice_name).await {
            synthesizer.SetVoice(&voice).map_err(TTSError::Windows)?;
            return Ok(true);
        }
        Ok(false)
    }

    /// Get the current default voice information
    pub async fn get_default_voice() -> Result<VoiceInfo> {
        tokio::task::spawn_blocking(|| {
            let default_voice = SpeechSynthesizer::DefaultVoice().map_err(TTSError::Windows)?;
            let display_name = default_voice
                .DisplayName()
                .map_err(TTSError::Windows)?
                .to_string();
            let id = default_voice.Id().map_err(TTSError::Windows)?.to_string();
            let language = default_voice
                .Language()
                .map_err(TTSError::Windows)?
                .to_string();
            let gender = match default_voice.Gender().map_err(TTSError::Windows)? {
                windows::Media::SpeechSynthesis::VoiceGender::Male => "Male".to_string(),
                windows::Media::SpeechSynthesis::VoiceGender::Female => "Female".to_string(),
                _ => "Unknown".to_string(),
            };

            Ok(VoiceInfo {
                id,
                display_name,
                language,
                gender,
            })
        })
        .await
        .map_err(|e| TTSError::Synthesis(format!("Task spawn error: {}", e)))?
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
