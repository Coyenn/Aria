use tokio::sync::mpsc;

use aria_core::driver::WindowsDriver;
use clap::Parser;
use log::Level;

/// CLI usage for Aria.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    #[clap(subcommand)]
    command: Option<Command>,
}

/// CLI subcommands for Aria.
#[derive(Parser, Debug)]
enum Command {
    /// Start Aria.
    Start,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();

    simple_logger::init_with_level(Level::Info)?;

    match args.command {
        Some(Command::Start) => start_aria().await?,
        _ => start_aria().await?,
    }

    Ok(())
}

pub async fn start_aria() -> Result<(), Box<dyn std::error::Error>> {
    let (tx, mut rx) = mpsc::channel(1);
    ctrlc::set_handler(move || {
        let _ = tx.blocking_send(());
    })?;

    WindowsDriver::start().await?;
    rx.recv().await.ok_or("Failed to receive Ctrl-C signal")?;
    WindowsDriver::stop().await?;
    Ok(())
}
