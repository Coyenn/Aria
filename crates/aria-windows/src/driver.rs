use std::sync::Mutex;
use std::thread;

use aria_tts::tts::say;
use mki::{Action, Keyboard};
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::UIElement;

struct FocusChangedEventHandler {
    previous_element: Mutex<Option<UIElement>>,
}

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
        let name = sender.get_name().unwrap();
        let content = sender.get_help_text().unwrap();
        let control_type: String = sender.get_control_type().unwrap().to_string();

        log::info!("Focus changed to: {}", name);

        let info_string = format!(
            "{}{}{}",
            name,
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

        say(&info_string).or_else(|e| {
            log::error!("TTS failed on focus change: {:?}", e);
            Err(e)
        })?;

        Ok(())
    }
}

fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);

    say(&key_name.clone())
        .or_else(|e| {
            log::error!("TTS failed on keypress: {:?}", e);
            Err(e)
        })
        .unwrap();
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub fn start() {
        let automation = UIAutomation::new().unwrap();
        let focus_changed_handler = FocusChangedEventHandler {
            previous_element: Mutex::new(None),
        };
        let focus_changed_handler = UIFocusChangedEventHandler::from(focus_changed_handler);

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
                on_keypress(format!("{:?}", key));
            }
        }));
    }

    pub fn stop() {
        say("Aria shutting down.").unwrap();
        log::info!("Stopping Windows driver.");
        thread::sleep(std::time::Duration::from_secs(1));
    }
}
