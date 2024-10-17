use rodio::{Decoder, OutputStream, Sink};
use std::io::Cursor;

pub const STARTUP_SOUND: &[u8] = include_bytes!("../assets/sounds/startup.mp3");
pub const SHUTDOWN_SOUND: &[u8] = include_bytes!("../assets/sounds/shutdown.mp3");
pub const INPUT_FOCUSSED_SOUND: &[u8] = include_bytes!("../assets/sounds/input-focussed.mp3");

pub fn play_sound(sound: &[u8]) {
    let sound_copy = sound.to_vec();

    std::thread::spawn(move || {
        // Play using rodio
        let (_stream, stream_handle) = OutputStream::try_default().unwrap();
        let sink = Sink::try_new(&stream_handle).unwrap();

        // Add a dummy source of the sake of the example.
        let cursor = Cursor::new(sound_copy);
        let source = Decoder::new(cursor).unwrap();

        sink.append(source);

        // The sound plays in a separate thread. This call will block the current thread until the sink
        // has finished playing all its queued sounds.
        sink.sleep_until_end();
    });
}
