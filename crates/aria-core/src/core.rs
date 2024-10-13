use aria_utils::config::get_config;
use aria_windows::driver::WindowsDriver;
use log::Level;
use std::env;

pub fn start() {
    simple_logger::init_with_level(Level::Info).unwrap();

    match env::consts::OS {
        "windows" => {
            get_config()
                .or_else(|e| {
                    log::error!("Failed to load config: {:?}", e);
                    Err(e)
                })
                .unwrap();

            log::info!("Starting Aria Windows driver.");
            WindowsDriver::start();
        }
        _ => println!("This program is only supported on Windows."),
    }
}

pub fn stop() {
    match env::consts::OS {
        "windows" => {
            WindowsDriver::stop();
        }
        _ => println!("This program is only supported on Windows."),
    }
}
