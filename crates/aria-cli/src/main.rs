use std::sync::{
    atomic::{AtomicBool, Ordering},
    Arc,
};

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

fn main() {
    let args = Args::parse();

    simple_logger::init_with_level(Level::Info).unwrap();

    match args.command {
        Some(Command::Start) => start_aria(),
        _ => start_aria(),
    }
}

pub fn start_aria() {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();

    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl-C handler");

    WindowsDriver::start();
    while running.load(Ordering::SeqCst) {}
    WindowsDriver::stop();
}

pub fn stop_aria() {
    WindowsDriver::stop();
}
