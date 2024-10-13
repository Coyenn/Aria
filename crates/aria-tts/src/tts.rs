use std::sync::Arc;
use tokio::sync::Mutex;
use windows::{
    core::HSTRING,
    Media::{Core::MediaSource, Playback::MediaPlayer, SpeechSynthesis::SpeechSynthesizer},
};

struct SpeechState {
    player: MediaPlayer,
    synthesizer: SpeechSynthesizer,
}

lazy_static::lazy_static! {
    static ref SPEECH_STATE: Arc<Mutex<Option<SpeechState>>> = Arc::new(Mutex::new(None));
}

pub async fn say(text: &str) -> windows::core::Result<()> {
    log::info!("Saying: {}", text);

    let mut state = SPEECH_STATE.lock().await;

    // Cancel the previous playback if it exists
    if let Some(prev_state) = state.as_ref() {
        prev_state.player.Pause()?;
    }

    // Create a new SpeechState if it doesn't exist
    if state.is_none() {
        *state = Some(SpeechState {
            player: MediaPlayer::new()?,
            synthesizer: SpeechSynthesizer::new()?,
        });
    }

    let speech_state = state.as_ref().unwrap();

    // Synthesize speech
    let stream = speech_state
        .synthesizer
        .SynthesizeTextToStreamAsync(&HSTRING::from(text))
        .unwrap()
        .get()
        .unwrap();

    let media_source =
        MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType()?))?;

    // Set the source and play
    speech_state.player.SetSource(&media_source)?;
    speech_state.player.Play()?;

    Ok(())
}
