use aria_tts::error::TTSError;
use aria_tts::tts::TTS;
use aria_utils::clean_text::{clean_text, RegexCleanerPair};
use aria_utils::config::get_config;
use egui::{Pos2 as EguiPos2, Rect as EguiRect};
use mki::{Action, Keyboard};
use once_cell::sync::{Lazy, OnceCell as StaticOnceCell};
use tokio::runtime::Handle as TokioHandle;
use tokio::sync::{mpsc, Mutex};
use tokio::task;
use uiautomation::controls::ControlType;
use uiautomation::core::UIAutomation;
use uiautomation::events::{CustomFocusChangedEventHandler, UIFocusChangedEventHandler};
use uiautomation::UIElement;

use crate::error::CoreError;
use crate::sound::{play_sound, INPUT_FOCUSSED_SOUND, SHUTDOWN_SOUND, STARTUP_SOUND};

// Static for Tokio Runtime Handle
static TOKIO_RUNTIME_HANDLE: StaticOnceCell<TokioHandle> = StaticOnceCell::new();
static RECT_SENDER: StaticOnceCell<mpsc::Sender<Option<EguiRect>>> = StaticOnceCell::new();

// Result type alias for this module
type Result<T> = std::result::Result<T, CoreError>;

struct FocusChangedEventHandler {
    previous_element: Mutex<Option<UIElement>>,
    // No need to store sender here if using a static OnceCell
}

static IS_FOCUSSED_ON_INPUT: Lazy<Mutex<bool>> = Lazy::new(|| Mutex::new(false));
static CONTENT_CLEAN_LIST: Lazy<Result<Vec<RegexCleanerPair>>> = Lazy::new(|| {
    RegexCleanerPair::prep_list(&[
        (r"\s+", " "),
        (
            r"(?P<s>[0-9a-f]{6})([0-9]+[a-f]|[a-f]+[0-9])[0-9a-f]*",
            "hash $s",
        ),
    ])
    .map_err(CoreError::Regex)
});

impl CustomFocusChangedEventHandler for FocusChangedEventHandler {
    fn handle(&self, sender: &UIElement) -> uiautomation::Result<()> {
        let mut previous_lock = match self.previous_element.try_lock() {
            Ok(guard) => guard,
            Err(_) => {
                log::warn!("Focus event handler skipped: could not acquire previous_element lock immediately.");
                return Ok(()); // Skip event if lock is contested
            }
        };

        if let Some(prev_elem) = previous_lock.as_ref() {
            if prev_elem.get_runtime_id()? == sender.get_runtime_id()? {
                return Ok(());
            }
        }

        *previous_lock = Some(sender.clone());
        // Drop the lock explicitly here if we are done with it before extracting other sender properties.
        // Or let it drop naturally at the end of its scope.
        // For clarity and to minimize lock duration if other calls are slow:
        drop(previous_lock);

        if let Some(rect_tx) = RECT_SENDER.get() {
            match sender.get_bounding_rectangle() {
                Ok(ui_rect) => {
                    let egui_rect = EguiRect::from_min_max(
                        EguiPos2::new(ui_rect.get_left() as f32, ui_rect.get_top() as f32),
                        EguiPos2::new(ui_rect.get_right() as f32, ui_rect.get_bottom() as f32),
                    );
                    let tx_clone = rect_tx.clone();
                    if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
                        handle.spawn(async move {
                            if let Err(e) = tx_clone.send(Some(egui_rect)).await {
                                log::error!("Failed to send highlight rect: {:?}", e);
                            }
                        });
                    } else {
                        log::error!("Tokio runtime not available for sending highlight rect.");
                    }
                }
                Err(e) => {
                    log::error!("Failed to get bounding rectangle for highlight: {:?}", e);
                    // Optionally send None to clear previous rect if element has no bounds
                    let tx_clone = rect_tx.clone();
                    if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
                        handle.spawn(async move {
                            if let Err(send_err) = tx_clone.send(None).await {
                                log::error!("Failed to clear highlight rect: {:?}", send_err);
                            }
                        });
                    } else {
                        log::error!("Tokio runtime not available for clearing highlight rect.");
                    }
                }
            }
        } else {
            log::warn!("RECT_SENDER not initialized, cannot highlight focus.");
        }

        // These calls to sender might be blocking COM calls.
        // If so, they should ideally be wrapped in spawn_blocking.
        let name = sender.get_name()?.trim().to_string();
        let content = sender
            .get_help_text()
            .unwrap_or_default()
            .trim()
            .to_string();
        let control_type_name: String = sender
            .get_localized_control_type()?
            .to_string()
            .trim()
            .to_string();
        let control_type = sender.get_control_type()?;

        if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
            handle.spawn(async move {
                let mut is_focussed_on_input_lock = IS_FOCUSSED_ON_INPUT.lock().await;

                log::info!("Focus changed to: {}", name);

                if control_type == ControlType::Edit || control_type == ControlType::ComboBox {
                    play_sound(INPUT_FOCUSSED_SOUND); // Assuming play_sound is non-blocking or very short
                    *is_focussed_on_input_lock = true;
                } else {
                    *is_focussed_on_input_lock = false;
                }
                drop(is_focussed_on_input_lock); // Release lock

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

                match CONTENT_CLEAN_LIST.as_ref() {
                    Ok(clean_list) => {
                        let cleaned_info_string: String = clean_text(&info_string, clean_list);
                        // Errors from TTS calls in spawned tasks are logged, not mapped to CoreError here.
                        // The type of `e` here will be TTSError.
                        if let Err(e) = TTS::stop(false).await {
                            log::error!("TTS stop failed on focus change: {:?}", e);
                        }
                        if let Err(e) = TTS::speak(&cleaned_info_string, false).await {
                            log::error!("TTS speak failed on focus change: {:?}", e);
                        }
                    }
                    Err(e) => {
                        log::error!("Failed to get content clean list: {:?}", e);
                        if let Err(e_tts) = TTS::speak(&info_string, false).await {
                            log::error!("TTS speak failed on focus change (fallback): {:?}", e_tts);
                        }
                    }
                }
            });
        } else {
            log::error!("Tokio runtime handle not available for focus change task spawn.");
        }
        Ok(())
    }
}

// This function is also likely called from a synchronous context (mki callback).
// Spawn async work to tokio runtime.
fn on_keypress(key_name: String) {
    log::info!("Key pressed: {}", key_name);
    if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
        handle.spawn(async move {
            let is_focussed = IS_FOCUSSED_ON_INPUT.lock().await.clone();
            if is_focussed {
                // Errors from TTS calls in spawned tasks are logged.
                if let Err(e) = TTS::stop(false).await {
                    log::error!("TTS stop failed on keypress: {:?}", e);
                }
                if let Err(e) = TTS::speak(&key_name, false).await {
                    log::error!("TTS speak failed on keypress: {:?}", e);
                }
            }
        });
    } else {
        log::error!("Tokio runtime handle not available for on_keypress task spawn.");
    }
}

pub struct WindowsDriver {}

impl WindowsDriver {
    pub async fn start() -> Result<()> {
        Self::start_with_highlight(None).await
    }

    pub async fn start_with_highlight(
        highlight_sender: Option<mpsc::Sender<Option<EguiRect>>>,
    ) -> Result<()> {
        // Ensure the Tokio runtime handle is initialized and stored.
        TOKIO_RUNTIME_HANDLE.get_or_init(tokio::runtime::Handle::current);
        // Check if it was successfully initialized (it should be, unless Handle::current() panics,
        // which it shouldn't if we are in an async fn called by #[tokio::main])
        if TOKIO_RUNTIME_HANDLE.get().is_none() {
            return Err(CoreError::Init(
                "Failed to initialize and store Tokio runtime handle.",
            ));
        }

        let config = get_config().map_err(|e| CoreError::Config(e.to_string()))?;

        // Set TTS to its operational state (able to speak and be stopped by default)
        TTS::set_can_stop(true)
            .await
            .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;
        TTS::set_can_speak(true)
            .await
            .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;

        if config.startup_shutdown_sounds {
            play_sound(STARTUP_SOUND);
            tokio::time::sleep(std::time::Duration::from_secs(3)).await;
            // Speak welcome message, ignoring the CAN_SPEAK flag for this specific utterance.
            TTS::speak("Welcome to Aria.", true)
                .await
                .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        }

        // Store the highlight sender if provided
        if let Some(sender) = highlight_sender {
            RECT_SENDER
                .set(sender)
                .map_err(|_| CoreError::Init("Failed to set RECT_SENDER for highlighter"))?;
        }

        // Setup event handlers after TTS flags are set for normal operation.
        let automation = UIAutomation::new()?;
        let focus_changed_handler = FocusChangedEventHandler {
            previous_element: Mutex::new(None),
        };
        let focus_changed_event_handler = UIFocusChangedEventHandler::from(focus_changed_handler);
        automation
            .add_focus_changed_event_handler(None, &focus_changed_event_handler)
            .map_err(CoreError::UIAutomation)?;

        task::spawn_blocking(move || {
            mki::bind_any_key(Action::handle_kb(|key| {
                use Keyboard::*;
                match key {
                    Escape => {
                        if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
                            handle.spawn(async {
                                if let Err(e) = TTS::stop(true).await {
                                    log::error!("TTS stop on escape failed: {:?}", e);
                                }
                            });
                        } else {
                            log::error!(
                                "Tokio runtime handle not available for TTS stop on escape."
                            );
                        }
                    }
                    _ => on_keypress(format!("{:?}", key)),
                }
            }));
        });
        Ok(())
    }

    pub async fn stop() -> Result<()> {
        let config = get_config().map_err(|e| CoreError::Config(e.to_string()))?;

        log::info!("Stopping Windows driver.");

        // Disable TTS general speaking/stopping before final shutdown message.
        TTS::set_can_stop(false)
            .await
            .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;
        TTS::set_can_speak(false)
            .await
            .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;

        // Speak shutdown message, ignoring flags.
        TTS::speak("Aria shutting down.", true)
            .await
            .map_err(|e: TTSError| CoreError::TTS(e.to_string()))?;
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        TTS::destroy().await.map_err(|e: TTSError| {
            CoreError::TTS(format!(
                "Failed to destroy TTS: {:?}. This may cause a memory leak.",
                e
            ))
        })?;

        if config.startup_shutdown_sounds {
            play_sound(SHUTDOWN_SOUND);
            tokio::time::sleep(std::time::Duration::from_secs(2)).await;
        }

        // Optionally, tell the highlighter to clear any existing rectangle or close
        if let Some(rect_tx) = RECT_SENDER.get() {
            let tx_clone = rect_tx.clone();
            if let Some(handle) = TOKIO_RUNTIME_HANDLE.get() {
                handle.spawn(async move {
                    if let Err(e) = tx_clone.send(None).await {
                        // Clear rectangle on stop
                        log::warn!("Failed to send None to highlight sender on stop: {:?}", e);
                    }
                    // The overlay will close when its rect_receiver channel detects disconnection,
                    // which happens when RECT_SENDER is dropped.
                    // If RECT_SENDER is in a StaticOnceCell, it's dropped when the program terminates.
                });
            } else {
                log::warn!("Tokio runtime not available for clearing highlight rect on stop.");
            }
        }
        Ok(())
    }
}
