#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use aria::start_highlight_overlay;
use aria_core::driver::WindowsDriver;
#[cfg(debug_assertions)]
use log::Level;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    #[cfg(debug_assertions)]
    simple_logger::init_with_level(Level::Info)?;

    // Start the GUI application
    start_aria().await?;

    Ok(())
}

async fn start_aria() -> Result<(), Box<dyn std::error::Error>> {
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
