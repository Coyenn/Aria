use aria_tts::tts::say;
use mki::{Action, Keyboard};
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::types::UIProperty;

struct FocusChangedEventHandler {}

impl CustomFocusChangedEventHandler for FocusChangedEventHandler {
    fn handle(&self, sender: &uiautomation::UIElement) -> uiautomation::Result<()> {
        let name = sender
            .get_property_value(UIProperty::Name)
            .unwrap()
            .get_string()
            .unwrap();

        log::info!("Focus changed to: {}", name);

        say(&name);
        Ok(())
    }
}

fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);

    say(&key_name.clone());
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub fn new() -> Self {
        Self {}
    }

    pub fn start(&self) {
        let automation = UIAutomation::new().unwrap();
        let focus_changed_handler = FocusChangedEventHandler {};
        let focus_changed_handler = UIFocusChangedEventHandler::from(focus_changed_handler);

        log::info!("Listening for focus changes.");
        automation
            .add_focus_changed_event_handler(None, &focus_changed_handler)
            .expect("Could not add focus changed event handler.");

        log::info!("Listening for keypresses.");
        mki::bind_any_key(Action::handle_kb(|key| {
            use Keyboard::*;

            if !matches!(key, Enter | LeftControl | C) {
                on_keypress(format!("{:?}", key));
            }
        }));
    }
}
