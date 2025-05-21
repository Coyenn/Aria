use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;
use tokio::task;

pub const STARTUP_SOUND: &[u8] = include_bytes!("../assets/sounds/startup.mp3");
pub const SHUTDOWN_SOUND: &[u8] = include_bytes!("../assets/sounds/shutdown.mp3");
pub const INPUT_FOCUSSED_SOUND: &[u8] = include_bytes!("../assets/sounds/input-focussed.mp3");

/// Plays the given sound data on a separate blocking thread using the default audio output.
/// Errors during sound playback are logged.
pub fn play_sound(sound_data: &'static [u8]) {
    task::spawn_blocking(move || match OutputStream::try_default() {
        Ok((_stream, stream_handle)) => match Sink::try_new(&stream_handle) {
            Ok(sink) => {
                let cursor = Cursor::new(sound_data);
                match Decoder::new(cursor) {
                    Ok(source) => {
                        sink.append(source);
                        sink.sleep_until_end();
                    }
                    Err(e) => log::error!("Failed to decode sound: {}", e),
                }
            }
            Err(e) => log::error!("Failed to create audio sink: {}", e),
        },
        Err(e) => log::error!("Failed to get default audio output stream: {}", e),
    });
}
