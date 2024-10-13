use once_cell::sync::Lazy;
use std::sync::Mutex;
use windows::{
    core::HSTRING,
    Media::{Core::MediaSource, Playback::MediaPlayer, SpeechSynthesis::SpeechSynthesizer},
};

static PLAYER: Lazy<Mutex<MediaPlayer>> = Lazy::new(|| Mutex::new(MediaPlayer::new().unwrap()));

pub fn say(text: &str) -> windows::core::Result<()> {
    log::info!("Saying: {}", text);

    let synthesizer = SpeechSynthesizer::new()?;
    let text = text.to_string();

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
