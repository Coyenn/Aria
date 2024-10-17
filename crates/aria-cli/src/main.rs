use aria_core::driver::WindowsDriver;
use aria_utils::config::get_config;
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

fn main() {
    let args = Args::parse();

    simple_logger::init_with_level(Level::Info).unwrap();

    match args.command {
        Some(Command::Start) => start_aria(),
        _ => start_aria(),
    }

    // Wait for user input to exit, due to the keylogger, only Enter, LeftControl, and C can be used.
    let mut input = String::new();

    std::io::stdin()
        .read_line(&mut input)
        .expect("Failed to read line.");

    stop_aria();
}

pub fn start_aria() {
    get_config()
        .or_else(|e| {
            log::error!("Failed to load config: {:?}", e);
            Err(e)
        })
        .unwrap();

    log::info!("Starting Aria Windows driver.");
    WindowsDriver::start(false);
}

pub fn stop_aria() {
    WindowsDriver::stop();
}
