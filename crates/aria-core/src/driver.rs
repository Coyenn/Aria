use std::sync::Mutex;
use std::thread;

use aria_tts::tts::TTS;
use aria_utils::config::get_config;
use mki::{Action, Keyboard};
use once_cell::sync::Lazy;
use uiautomation::controls::ControlType;
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::UIElement;

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
        let mut is_focussed_on_input = IS_FOCUSSED_ON_INPUT.lock().unwrap();

        log::info!("Focus changed to: {}", name);

        if control_type == ControlType::Edit || control_type == ControlType::ComboBox {
            play_sound(INPUT_FOCUSSED_SOUND);

            *is_focussed_on_input = true;
        } else {
            *is_focussed_on_input = false;
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

        TTS::stop(false).unwrap();
        TTS::speak(&info_string, false).or_else(|e| {
            log::error!("TTS failed on focus change: {:?}", e);
            Err(e)
        })?;

        Ok(())
    }
}

fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);

    if IS_FOCUSSED_ON_INPUT.lock().unwrap().clone() {
        TTS::stop(false).unwrap();
        TTS::speak(&key_name.clone(), false)
            .or_else(|e| {
                log::error!("TTS failed on keypress: {:?}", e);
                Err(e)
            })
            .unwrap();
    }
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub fn start() {
        let config = get_config().unwrap();
        let automation = UIAutomation::new().unwrap();
        let focus_changed_handler = FocusChangedEventHandler {
            previous_element: Mutex::new(None),
        };
        let focus_changed_handler = UIFocusChangedEventHandler::from(focus_changed_handler);

        TTS::set_can_stop(false).unwrap();
        TTS::set_can_speak(false).unwrap();

        if config.startup_shutdown_sounds {
            play_sound(STARTUP_SOUND);
            std::thread::sleep(std::time::Duration::from_secs(3));
            TTS::speak("Welcome to Aria.", true).unwrap();
            std::thread::sleep(std::time::Duration::from_secs(1));
        }

        automation
            .add_focus_changed_event_handler(None, &focus_changed_handler)
            .expect("Could not add focus changed event handler.");

        mki::bind_any_key(Action::handle_kb(|key| {
            use Keyboard::*;

            match key {
                Escape => TTS::stop(true).unwrap(),
                _ => on_keypress(format!("{:?}", key)),
            }
        }));

        TTS::set_can_stop(true).unwrap();
        TTS::set_can_speak(true).unwrap();
    }

    pub fn stop() {
        let config = get_config().unwrap();

        log::info!("Stopping Windows driver.");

        TTS::set_can_stop(false).unwrap();
        TTS::set_can_speak(false).unwrap();
        TTS::stop(true).unwrap();
        TTS::speak("Aria shutting down.", true).unwrap();
        thread::sleep(std::time::Duration::from_secs(1));
        TTS::destroy().expect("Failed to destroy TTS. This may cause a memory leak.");

        if config.startup_shutdown_sounds {
            play_sound(SHUTDOWN_SOUND);
            thread::sleep(std::time::Duration::from_secs(2));
        }
    }
}
