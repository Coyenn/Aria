use aria_core::driver::WindowsDriver;
use aria_gui::start_highlight_overlay;
use log::Level;
use tokio::sync::mpsc;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    simple_logger::init_with_level(Level::Info)?;

    // Start the GUI application
    start_aria_gui().await?;

    Ok(())
}

async fn start_aria_gui() -> Result<(), Box<dyn std::error::Error>> {
    // Set up shutdown signal handling
    let (shutdown_tx, mut shutdown_rx) = mpsc::channel(1);
    ctrlc::set_handler(move || {
        let _ = shutdown_tx.blocking_send(());
    })?;

    // Start the highlight overlay GUI and get its sender
    let highlight_sender = start_highlight_overlay();

    // Start the Windows driver with highlight functionality
    WindowsDriver::start_with_highlight(Some(highlight_sender)).await?;

    // Wait for shutdown signal
    shutdown_rx
        .recv()
        .await
        .ok_or("Failed to receive shutdown signal")?;

    // Stop the Windows driver
    WindowsDriver::stop().await?;

    Ok(())
}
