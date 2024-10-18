use std::thread;

use aria::overlay::Overlay;
use aria_core::driver::WindowsDriver;
use aria_utils::config::get_config;
use iced::{
    window::{self, settings::PlatformSpecific, Settings as IcedWindowSettings},
    Settings as IcedSettings,
};
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

    let window_settings = IcedWindowSettings {
        exit_on_close_request: false,
        level: window::Level::AlwaysOnTop,
        decorations: false,
        resizable: false,
        transparent: true,
        platform_specific: PlatformSpecific {
            drag_and_drop: false,
            skip_taskbar: true,
            ..PlatformSpecific::default()
        },
        ..IcedWindowSettings::default()
    };

    let settings = IcedSettings {
        antialiasing: true,
        ..Default::default()
    };

    iced::application("Aria", Overlay::update, Overlay::view)
        .subscription(Overlay::subscription)
        .window(window_settings)
        .settings(settings)
        .theme(Overlay::theme)
        .centered()
        .run_with(Overlay::new)
        .unwrap();
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
