use aria_utils::config::get_config;
use once_cell::sync::Lazy;
use std::sync::Mutex;
use windows::{
    core::HSTRING,
    Media::{
        Core::MediaSource,
        Playback::MediaPlayer,
        SpeechSynthesis::{SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesizer},
    },
};

static TTS_MANAGER: Lazy<Mutex<TTSManager>> = Lazy::new(|| Mutex::new(TTSManager::new()));

struct TTSManager {
    player: MediaPlayer,
    synthesizer: SpeechSynthesizer,
}

impl TTSManager {
    fn new() -> Self {
        let player = MediaPlayer::new().unwrap();
        let synthesizer = SpeechSynthesizer::new().unwrap();
        Self::apply_config(&synthesizer).unwrap();
        Self {
            player,
            synthesizer,
        }
    }

    fn apply_config(synthesizer: &SpeechSynthesizer) -> windows::core::Result<()> {
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

        Ok(())
    }

    fn say(&mut self, text: &str) -> windows::core::Result<()> {
        log::info!("{}", text);

        let stream = self
            .synthesizer
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
            .get()?;
        let media_source =
            MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType()?))?;

        self.player.SetSource(&media_source)?;
        self.player.Play()?;

        Ok(())
    }

    fn stop(&mut self) -> windows::core::Result<()> {
        self.player.Pause()
    }

    fn destroy(&mut self) -> windows::core::Result<()> {
        self.player.Close()?;
        self.synthesizer.Close()?;

        Ok(())
    }
}

pub struct TTS {}

impl TTS {
    pub fn say(text: &str) -> windows::core::Result<()> {
        TTS_MANAGER.lock().unwrap().say(text)
    }

    pub fn stop() -> windows::core::Result<()> {
        TTS_MANAGER.lock().unwrap().stop()
    }

    pub fn destroy() -> windows::core::Result<()> {
        TTS_MANAGER.lock().unwrap().destroy()
    }
}
