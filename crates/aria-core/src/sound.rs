use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub const STARTUP_SOUND: &[u8] = include_bytes!("../assets/sounds/startup.mp3");
pub const SHUTDOWN_SOUND: &[u8] = include_bytes!("../assets/sounds/shutdown.mp3");
pub const INPUT_FOCUSSED_SOUND: &[u8] = include_bytes!("../assets/sounds/input-focussed.mp3");

/// Plays the given sound data asynchronously on the default audio output.
pub fn play_sound(sound_data: &[u8]) {
    let sound_data_clone = sound_data.to_vec();

    std::thread::spawn(move || {
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        let cursor = Cursor::new(sound_data_clone);
        let source = Decoder::new(cursor).unwrap();

        sink.append(source);

        sink.sleep_until_end();
    });
}
