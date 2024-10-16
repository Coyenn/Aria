use aria_utils::config::get_config;
use once_cell::sync::Lazy;
use std::sync::{mpsc, Arc, Mutex};
use windows::{
    core::HSTRING,
    Foundation::{EventRegistrationToken, TypedEventHandler},
    Media::{
        Core::MediaSource,
        Playback::MediaPlayer,
        SpeechSynthesis::{SpeechAppendedSilence, SpeechPunctuationSilence, SpeechSynthesizer},
    },
};

static SYNC_PLAYER: Lazy<Mutex<MediaPlayer>> =
    Lazy::new(|| Mutex::new(MediaPlayer::new().unwrap()));
static SYNTHESIZER: Lazy<SpeechSynthesizer> =
    Lazy::new(|| apply_config(&SpeechSynthesizer::new().unwrap()).unwrap());
static ACTIVE_ASYNC_PLAYERS: Lazy<Mutex<Vec<Arc<Mutex<MediaPlayer>>>>> =
    Lazy::new(|| Mutex::new(Vec::new()));

pub fn apply_config(synthesizer: &SpeechSynthesizer) -> windows::core::Result<SpeechSynthesizer> {
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

    Ok(synthesizer.to_owned())
}

pub fn say(text: &str) {
    let text = text.to_string();
    let (tx, rx) = mpsc::channel();

    stop_all_tts_players();

    std::thread::spawn(move || {
        log::info!("{}", text);

        let stream = SYNTHESIZER
            .SynthesizeTextToStreamAsync(&HSTRING::from(text))
            .unwrap()
            .get()
            .unwrap();
        let media_source =
            MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType().unwrap()))
                .unwrap();
        let player = Arc::new(Mutex::new(MediaPlayer::new().unwrap()));

        player.lock().unwrap().SetSource(&media_source).unwrap();

        ACTIVE_ASYNC_PLAYERS
            .lock()
            .unwrap()
            .push(Arc::clone(&player));

        let player_clone = Arc::clone(&player);
        let tx_clone = tx.clone();
        let token: Option<EventRegistrationToken> = Some(
            player
                .lock()
                .unwrap()
                .MediaEnded(&TypedEventHandler::new(move |_, _| {
                    let mut active_async_players = ACTIVE_ASYNC_PLAYERS.lock().unwrap();
                    active_async_players.retain(|p| !Arc::ptr_eq(p, &player_clone));
                    tx_clone.send(()).unwrap();
                    Ok(())
                }))
                .unwrap(),
        );

        player.lock().unwrap().Play().unwrap();

        // Wait for the media to finish playing
        rx.recv().unwrap();

        // Unregister the event handler
        if let Some(t) = token {
            player.lock().unwrap().RemoveMediaEnded(t).unwrap();
        }
    })
    .join()
    .unwrap();
}

pub fn say_sync(text: &str) -> windows::core::Result<()> {
    log::info!("{}", text);

    let text = text.to_string();

    apply_config(&SYNTHESIZER)?;

    let stream = SYNTHESIZER
        .SynthesizeTextToStreamAsync(&HSTRING::from(text))?
        .get()?;
    let media_source =
        MediaSource::CreateFromStream(&stream, &HSTRING::from(stream.ContentType()?))?;

    // Get the locked player instance
    let player = SYNC_PLAYER.lock().unwrap();

    // Stop any currently playing audio
    player.Pause()?;

    // Set the new source and play
    player.SetSource(&media_source)?;
    player.Play()?;

    Ok(())
}

pub fn stop_all_tts_players() {
    let sync_player = SYNC_PLAYER.lock().unwrap();
    let mut active_async_players = ACTIVE_ASYNC_PLAYERS.lock().unwrap();

    for player in active_async_players.iter() {
        player.lock().unwrap().Close().unwrap();
    }

    sync_player.Pause().unwrap();
    active_async_players.clear();
}
