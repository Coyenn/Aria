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

static PLAYER: Lazy<Mutex<MediaPlayer>> = Lazy::new(|| Mutex::new(MediaPlayer::new().unwrap()));

pub fn say(text: &str) -> windows::core::Result<()> {
    log::info!("Saying: {}", text);

    let synthesizer = SpeechSynthesizer::new()?;
    let synthesizer_options = synthesizer.Options()?;
    let config = get_config().unwrap();
    let text = text.to_string();

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

    let stream = synthesizer
        .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
        .get()?;
    let media_source =
        MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType()?))?;

    // Get the locked player instance
    let player = PLAYER.lock().unwrap();

    // Stop any currently playing audio
    player.Pause()?;

    // Set the new source and play
    player.SetSource(&media_source)?;
    player.Play()?;

    Ok(())
}
