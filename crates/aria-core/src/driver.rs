use std::sync::Mutex;
use std::thread;

use aria_tts::tts::{destroy_tts, say, stop_tts};
use aria_utils::config::get_config;
use mki::{Action, Keyboard};
use once_cell::sync::Lazy;
use uiautomation::controls::ControlType;
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::UIElement;

use crate::overlay::start_overlay;
use crate::sound::{play_sound, INPUT_FOCUSSED_SOUND, SHUTDOWN_SOUND, STARTUP_SOUND};

struct FocusChangedEventHandler {
    previous_element: Mutex<Option<UIElement>>,
}

static IS_FOCUSSED_ON_INPUT: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));

impl CustomFocusChangedEventHandler for FocusChangedEventHandler {
    fn handle(&self, sender: &uiautomation::UIElement) -> uiautomation::Result<()> {
        let mut previous = self.previous_element.lock().unwrap();

        // Check if the new element is the same as the previous one
        if let Some(prev_elem) = previous.as_ref() {
            if prev_elem.get_runtime_id()? == sender.get_runtime_id()? {
                // Same element, do nothing
                return Ok(());
            }
        }

        // Update the previous element
        *previous = Some(sender.clone());

        // Proceed with handling the new focus
        let name = sender.get_name().unwrap().trim().to_string();
        let content = sender.get_help_text().unwrap().trim().to_string();
        let control_type_name: String = sender
            .get_localized_control_type()
            .unwrap()
            .to_string()
            .trim()
            .to_string();
        let control_type = sender.get_control_type().unwrap();

        log::info!("Focus changed to: {}", name);

        if control_type == ControlType::Edit || control_type == ControlType::ComboBox {
            play_sound(INPUT_FOCUSSED_SOUND);

            *IS_FOCUSSED_ON_INPUT.lock().unwrap() = true;
        } else {
            *IS_FOCUSSED_ON_INPUT.lock().unwrap() = false;
        }

        let mut parts = Vec::new();

        if !name.is_empty() {
            parts.push(name);
        }
        if !content.is_empty() {
            parts.push(content);
        }
        if !control_type_name.is_empty() {
            parts.push(control_type_name);
        }

        let info_string = parts.join(", ");

        stop_tts().unwrap();
        say(&info_string).or_else(|e| {
            log::error!("TTS failed on focus change: {:?}", e);
            Err(e)
        })?;

        Ok(())
    }
}

fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);

    if IS_FOCUSSED_ON_INPUT.lock().unwrap().clone() {
        stop_tts().unwrap();
        say(&key_name.clone())
            .or_else(|e| {
                log::error!("TTS failed on keypress: {:?}", e);
                Err(e)
            })
            .unwrap();
    }
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub fn start(with_graphics: bool) {
        let config = get_config().unwrap();
        let automation = UIAutomation::new().unwrap();
        let focus_changed_handler = FocusChangedEventHandler {
            previous_element: Mutex::new(None),
        };
        let focus_changed_handler = UIFocusChangedEventHandler::from(focus_changed_handler);

        if config.startup_shutdown_sounds {
            play_sound(STARTUP_SOUND);
            std::thread::sleep(std::time::Duration::from_secs(3));
            say("Welcome to Aria.").unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        if with_graphics {
            start_overlay().unwrap();
        }

        // Listen for focus changes, e.g. when a window or control is focused.
        log::info!("Listening for focus changes.");
        automation
            .add_focus_changed_event_handler(None, &focus_changed_handler)
            .expect("Could not add focus changed event handler.");

        // Listen for keypresses.
        log::info!("Listening for keypresses.");
        mki::bind_any_key(Action::handle_kb(|key| {
            use Keyboard::*;

            if !matches!(key, Enter | LeftControl | C) {
                match key {
                    Escape => stop_tts().unwrap(),
                    _ => on_keypress(format!("{:?}", key)),
                }
            }
        }));
    }

    pub fn stop() {
        let config = get_config().unwrap();

        log::info!("Stopping Windows driver.");

        stop_tts().unwrap();
        say("Aria shutting down.").unwrap();
        thread::sleep(std::time::Duration::from_secs(1));
        destroy_tts().expect("Failed to destroy TTS. This may cause a memory leak.");

        if config.startup_shutdown_sounds {
            play_sound(SHUTDOWN_SOUND);
            thread::sleep(std::time::Duration::from_secs(2));
        }
    }
}
