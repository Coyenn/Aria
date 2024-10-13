use aria_windows::driver::WindowsDriver;
use log::Level;
use std::env;

pub fn start() {
    simple_logger::init_with_level(Level::Info).unwrap();

    match env::consts::OS {
        "windows" => {
            let driver = WindowsDriver::new();

            log::info!("Starting Aria Windows driver.");
            driver.start();
        }
        _ => println!("This program is only supported on Windows."),
    }
}
