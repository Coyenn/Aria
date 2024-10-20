use aria_utils::config::get_config;
use log::warn;
use std::sync::{LazyLock, Mutex};
use windows::{
    core::HSTRING,
    Media::{
        Core::MediaSource,
        Playback::MediaPlayer,
        SpeechSynthesis::{SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesizer},
    },
};

pub struct TTS {}

fn apply_config(synthesizer: &SpeechSynthesizer) -> windows::core::Result<SpeechSynthesizer> {
    let synthesizer_options = synthesizer.Options()?;
    let config = get_config().unwrap();

    synthesizer_options.SetSpeakingRate(config.speech_rate)?;
    synthesizer_options.SetAppendedSilence(if config.append_silence {
        SpeechAppendedSilence::Default
    } else {
        SpeechAppendedSilence::Min
    })?;
    synthesizer_options.SetPunctuationSilence(if config.punctuation_silence {
        SpeechPunctuationSilence::Default
    } else {
        SpeechPunctuationSilence::Min
    })?;

    Ok(synthesizer.clone())
}

static SYNTHESIZER: LazyLock<SpeechSynthesizer> =
    LazyLock::new(|| apply_config(&SpeechSynthesizer::new().unwrap()).unwrap());
static PLAYER: LazyLock<MediaPlayer> = LazyLock::new(|| MediaPlayer::new().unwrap());
static CAN_STOP: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));
static CAN_SPEAK: LazyLock<Mutex<bool>> = LazyLock::new(|| Mutex::new(false));

impl TTS {
    pub fn speak(text: &str, ignore_can_speak: bool) -> windows::core::Result<()> {
        log::info!("{}", text);

        let can_speak_lock = CAN_SPEAK.lock().unwrap();

        if !*can_speak_lock && !ignore_can_speak {
            warn!("Cannot speak");

            return Ok(());
        }

        let stream = SYNTHESIZER
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
            .get()?;
        let media_source =
            MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType()?))?;

        PLAYER.SetSource(&media_source)?;
        PLAYER.Play()?;

        Ok(())
    }

    pub fn set_can_stop(can_stop: bool) -> windows::core::Result<()> {
        let mut can_stop_lock = CAN_STOP.lock().unwrap();

        *can_stop_lock = can_stop;

        Ok(())
    }

    pub fn set_can_speak(can_speak: bool) -> windows::core::Result<()> {
        let mut can_speak_lock = CAN_SPEAK.lock().unwrap();

        *can_speak_lock = can_speak;

        Ok(())
    }

    pub fn stop(ignore_can_stop: bool) -> windows::core::Result<()> {
        let can_stop_lock = CAN_STOP.lock().unwrap();

        if !*can_stop_lock && !ignore_can_stop {
            warn!("Cannot stop");

            return Ok(());
        }

        PLAYER.Pause()
    }

    pub fn destroy() -> windows::core::Result<()> {
        PLAYER.Close()?;
        SYNTHESIZER.Close()?;

        Ok(())
    }
}
