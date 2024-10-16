use std::sync::Mutex;
use std::thread;

use aria_tts::tts::{say, say_sync, stop_all_tts_players};
use mki::{Action, Keyboard};
use once_cell::sync::Lazy;
use uiautomation::controls::ControlType;
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::UIElement;

use crate::sound::{play_sound, SHUTDOWN_SOUND, STARTUP_SOUND};

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
        let control_type: String = sender
            .get_localized_control_type()
            .unwrap()
            .to_string()
            .trim()
            .to_string();

        log::info!("Focus changed to: {}", name);

        *IS_FOCUSSED_ON_INPUT.lock().unwrap() =
            sender.get_control_type().unwrap() == ControlType::Edit;

        let info_string = format!(
            "{}{}{}",
            if !name.is_empty() {
                format!("{}", name)
            } else {
                String::new()
            },
            if !content.is_empty() {
                format!(", {}", content)
            } else {
                String::new()
            },
            if !control_type.is_empty() {
                format!(", {}", control_type)
            } else {
                String::new()
            }
        );

        say_sync(&info_string).or_else(|e| {
            log::error!("TTS failed on focus change: {:?}", e);
            Err(e)
        })?;

        Ok(())
    }
}

fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);

    if IS_FOCUSSED_ON_INPUT.lock().unwrap().clone() {
        say(&key_name.clone());
    }
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub fn start() {
        let automation = UIAutomation::new().unwrap();
        let focus_changed_handler = FocusChangedEventHandler {
            previous_element: Mutex::new(None),
        };
        let focus_changed_handler = UIFocusChangedEventHandler::from(focus_changed_handler);

        play_sound(STARTUP_SOUND);
        std::thread::sleep(std::time::Duration::from_secs(3));

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
                    Escape => stop_all_tts_players(),
                    _ => on_keypress(format!("{:?}", key)),
                }
            }
        }));
    }

    pub fn stop() {
        log::info!("Stopping Windows driver.");

        stop_all_tts_players();

        say_sync("Aria shutting down.").unwrap();
        thread::sleep(std::time::Duration::from_secs(1));

        play_sound(SHUTDOWN_SOUND);
        thread::sleep(std::time::Duration::from_secs(2));
    }
}
