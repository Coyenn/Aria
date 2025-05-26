#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aria::cli::{Args, Command};
use aria::start_highlight_overlay;
use aria_core::driver::WindowsDriver;
use clap::Parser;
use log::Level;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logger once at the start
    simple_logger::init_with_level(Level::Info)?;

    // Parse CLI arguments
    let args = Args::parse();

    // Check if any CLI commands were provided
    match args.command {
        Some(Command::Start) => aria::cli::start_aria_cli().await?,
        Some(Command::Voices) => aria::cli::list_voices().await?,
        Some(Command::Speak { text, voice }) => {
            aria::cli::speak_text(&text, voice.as_deref()).await?
        }
        None => {
            // No CLI command provided, start GUI mode
            start_aria_gui().await?;
        }
    }

    Ok(())
}

async fn start_aria_gui() -> Result<(), Box<dyn std::error::Error>> {
    // Set up shutdown signal handling for both Ctrl+C and window close
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);

    // Handle Ctrl+C (mainly for debug builds)
    let shutdown_tx_ctrlc = shutdown_tx.clone();
    ctrlc::set_handler(move || {
        let _ = shutdown_tx_ctrlc.blocking_send(());
    })?;

    // Start the highlight overlay GUI and get its sender and close handle
    let (highlight_sender, window_close_rx) = start_highlight_overlay();

    // Start the Windows driver with highlight functionality
    WindowsDriver::start_with_highlight(Some(highlight_sender)).await?;

    // Wait for either shutdown signal or window close
    tokio::select! {
        _ = shutdown_rx.recv() => {
            log::info!("Received shutdown signal");
        }
        result = window_close_rx => {
            match result {
                Ok(()) => log::info!("GUI window closed"),
                Err(_) => log::warn!("Window close channel closed unexpectedly"),
            }
        }
    }

    // Stop the Windows driver
    WindowsDriver::stop().await?;

    Ok(())
}
