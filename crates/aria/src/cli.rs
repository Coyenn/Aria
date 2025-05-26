use tokio::sync::mpsc;

use aria_core::driver::WindowsDriver;
use aria_tts::tts::TTS;
use clap::Parser;

/// CLI usage for Aria
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub struct Args {
    #[clap(subcommand)]
    pub command: Option<Command>,
}

/// CLI subcommands for Aria.
#[derive(Parser, Debug)]
pub enum Command {
    /// Start Aria
    Start,
    /// List all available TTS voices on the system.
    Voices,
    /// Speak text using TTS with optional voice selection.
    Speak {
        /// Text to speak for testing.
        #[clap(default_value = "Hello, this is a test of the Aria TTS system.")]
        text: String,
        /// Voice to use for this test (by display name).
        #[clap(short, long)]
        voice: Option<String>,
    },
}

pub async fn start_aria_cli() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(1);
    ctrlc::set_handler(move || {
        let _ = tx.blocking_send(());
    })?;

    WindowsDriver::start().await?;
    rx.recv().await.ok_or("Failed to receive Ctrl-C signal")?;
    WindowsDriver::stop().await?;
    Ok(())
}

pub async fn list_voices() -> Result<(), Box<dyn std::error::Error>> {
    let voices = TTS::get_available_voices().await?;

    println!("Available TTS Voices:");

    for voice in voices.iter() {
        println!(
            "- {} ({}, {})",
            voice.display_name, voice.language, voice.gender
        );
    }

    if voices.is_empty() {
        println!("No voices found.");
    } else {
        println!("Total: {} voice(s) available", voices.len());
    }

    Ok(())
}

pub async fn speak_text(text: &str, voice: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Set voice if specified
    if let Some(voice_name) = voice {
        let success = TTS::set_voice(voice_name).await?;
        if !success {
            println!(
                "Warning: Voice '{}' not found, using default voice",
                voice_name
            );
        }
    }

    // Speak the text and wait for completion
    TTS::speak_and_wait(text, false).await?;

    Ok(())
}
