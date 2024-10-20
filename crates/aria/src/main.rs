use std::thread;

use aria_core::driver::WindowsDriver;
use aria_utils::config::get_config;
use log::Level;

fn main() {
    simple_logger::init_with_level(Level::Info).unwrap();

    thread::spawn(|| {
        start_aria();
        // Sleep indefinitely
        loop {
            thread::sleep(std::time::Duration::from_secs(60));
        }
    });
}

pub fn start_aria() {
    get_config()
        .or_else(|e| {
            log::error!("Failed to load config: {:?}", e);
            Err(e)
        })
        .unwrap();

    log::info!("Starting Aria Windows driver.");
    WindowsDriver::start();
}
