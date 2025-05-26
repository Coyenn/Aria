pub mod cli;

use egui::epaint::RectShape;
use egui::{Context, Rect, Rgba, Shape, Stroke};
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;
use egui_overlay::egui_window_glfw_passthrough::GlfwBackend;
use egui_overlay::{start, EguiOverlay};
use image::ImageFormat;
use std::thread;
use std::time::Instant;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug, Clone)]
struct MonitorInfo {
    x: i32,
    y: i32,
    width: i32,
    height: i32,
}

pub struct FocusHighlighter {
    target_rect: Option<Rect>,
    receiver: mpsc::Receiver<Option<Rect>>,
    initialized: bool,
    close_sender: Option<oneshot::Sender<()>>,
    current_monitor: Option<MonitorInfo>,
    last_update_time: Option<Instant>,
}

impl FocusHighlighter {
    pub fn new(receiver: mpsc::Receiver<Option<Rect>>, close_sender: oneshot::Sender<()>) -> Self {
        Self {
            target_rect: None,
            receiver,
            initialized: false,
            close_sender: Some(close_sender),
            current_monitor: None,
            last_update_time: None,
        }
    }

    /// Find which monitor contains the given rectangle
    fn find_monitor_for_rect(
        &self,
        glfw_backend: &mut GlfwBackend,
        rect: Rect,
    ) -> Option<MonitorInfo> {
        let rect_center_x = rect.center().x;
        let rect_center_y = rect.center().y;

        // Get all monitors using the GLFW API
        let mut target_monitor = None;

        glfw_backend.glfw.with_connected_monitors(|_, monitors| {
            // First pass: Check if the rectangle center is within any monitor's bounds
            for monitor in monitors {
                if let Some(mode) = monitor.get_video_mode() {
                    let (monitor_x, monitor_y) = monitor.get_pos();
                    let monitor_width = mode.width as i32;
                    let monitor_height = mode.height as i32;

                    // Check if the rectangle center is within this monitor's bounds
                    if rect_center_x >= monitor_x as f32
                        && rect_center_x < (monitor_x + monitor_width) as f32
                        && rect_center_y >= monitor_y as f32
                        && rect_center_y < (monitor_y + monitor_height) as f32
                    {
                        target_monitor = Some(MonitorInfo {
                            x: monitor_x,
                            y: monitor_y,
                            width: monitor_width,
                            height: monitor_height,
                        });
                        return; // Break out of the closure
                    }
                }
            }

            // Second pass: If no monitor contains the rect center, find the monitor with the largest overlap
            let mut best_monitor = None;
            let mut best_overlap_area = 0.0;

            for monitor in monitors {
                if let Some(mode) = monitor.get_video_mode() {
                    let (monitor_x, monitor_y) = monitor.get_pos();
                    let monitor_width = mode.width as i32;
                    let monitor_height = mode.height as i32;

                    // Calculate overlap area
                    let overlap_left = rect.min.x.max(monitor_x as f32);
                    let overlap_top = rect.min.y.max(monitor_y as f32);
                    let overlap_right = rect.max.x.min((monitor_x + monitor_width) as f32);
                    let overlap_bottom = rect.max.y.min((monitor_y + monitor_height) as f32);

                    if overlap_right > overlap_left && overlap_bottom > overlap_top {
                        let overlap_area =
                            (overlap_right - overlap_left) * (overlap_bottom - overlap_top);

                        if overlap_area > best_overlap_area {
                            best_overlap_area = overlap_area;
                            best_monitor = Some(MonitorInfo {
                                x: monitor_x,
                                y: monitor_y,
                                width: monitor_width,
                                height: monitor_height,
                            });
                        }
                    }
                }
            }

            target_monitor = best_monitor;
        });

        target_monitor
    }

    /// Update the overlay window to cover the specified monitor
    fn update_to_monitor(&mut self, glfw_backend: &mut GlfwBackend, monitor_info: &MonitorInfo) {
        log::info!(
            "Switching overlay to monitor at {}x{} ({}x{})",
            monitor_info.x,
            monitor_info.y,
            monitor_info.width,
            monitor_info.height
        );

        glfw_backend.window.set_pos(monitor_info.x, monitor_info.y);
        glfw_backend.window.set_size(
            monitor_info.width,
            // -1 because once the window is full size, it turns black. Gotta love Windows.
            monitor_info.height - 1,
        );

        self.current_monitor = Some(monitor_info.clone());
    }
}

impl EguiOverlay for FocusHighlighter {
    fn gui_run(
        &mut self,
        egui_context: &Context,
        _default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut GlfwBackend,
    ) {
        if !self.initialized {
            // Set window title and icon
            glfw_backend.window.set_title("Aria Focus Overlay");

            // Load and set the icon
            if let Some(icon) = load_icon() {
                glfw_backend.window.set_icon_from_pixels(vec![icon]);
            }

            glfw_backend.set_passthrough(true);

            // Start with the primary monitor
            glfw_backend.glfw.with_primary_monitor(|_, monitor_opt| {
                if let Some(monitor) = monitor_opt {
                    if let Some(mode) = monitor.get_video_mode() {
                        let (monitor_x, monitor_y) = monitor.get_pos();
                        let monitor_info = MonitorInfo {
                            x: monitor_x,
                            y: monitor_y,
                            width: mode.width as i32,
                            height: mode.height as i32,
                        };

                        glfw_backend.window.set_pos(monitor_info.x, monitor_info.y);
                        glfw_backend.window.set_size(
                            monitor_info.width,
                            // -1 because once the window is full size, it turns black. Gotta love Windows.
                            monitor_info.height - 1,
                        );
                        self.current_monitor = Some(monitor_info);
                    }
                }
            });
            self.initialized = true;
        }

        // Check if window should close
        if glfw_backend.window.should_close() {
            if let Some(close_sender) = self.close_sender.take() {
                let _ = close_sender.send(());
            }
        }

        // Handle incoming rectangle updates
        if let Ok(Some(rect)) = self.receiver.try_recv() {
            self.target_rect = Some(rect);
            self.last_update_time = Some(Instant::now());

            // Check if we need to switch monitors
            if let Some(target_monitor) = self.find_monitor_for_rect(glfw_backend, rect) {
                // Only update if we're switching to a different monitor
                let should_update = match &self.current_monitor {
                    Some(current) => {
                        current.x != target_monitor.x
                            || current.y != target_monitor.y
                            || current.width != target_monitor.width
                            || current.height != target_monitor.height
                    }
                    None => true,
                };

                if should_update {
                    self.update_to_monitor(glfw_backend, &target_monitor);
                }
            }
        }

        // Draw the highlight rectangle
        if let Some(rect) = self.target_rect {
            // Adjust rect coordinates relative to current monitor if needed
            let adjusted_rect = if let Some(current_monitor) = &self.current_monitor {
                Rect::from_min_max(
                    egui::Pos2::new(
                        rect.min.x - current_monitor.x as f32,
                        rect.min.y - current_monitor.y as f32,
                    ),
                    egui::Pos2::new(
                        rect.max.x - current_monitor.x as f32,
                        rect.max.y - current_monitor.y as f32,
                    ),
                )
            } else {
                rect
            };

            let painter = egui_context.layer_painter(egui::LayerId::debug());
            let stroke = Stroke::new(2.0, Rgba::RED);
            painter.add(Shape::Rect(RectShape::stroke(adjusted_rect, 0.0, stroke)));
        }
    }

    fn run(
        &mut self,
        egui_context: &Context,
        default_gfx_backend: &mut DefaultGfxBackend,
        glfw_backend: &mut GlfwBackend,
    ) -> Option<(egui::PlatformOutput, std::time::Duration)> {
        // Check if window should close first
        if glfw_backend.window.should_close() {
            if let Some(close_sender) = self.close_sender.take() {
                let _ = close_sender.send(());
            }
            return None; // Exit the run loop
        }

        // Gather input and prepare frame
        let input = glfw_backend.take_raw_input();
        default_gfx_backend.prepare_frame(|| {
            let size = glfw_backend.window.get_framebuffer_size();
            [size.0 as _, size.1 as _]
        });
        egui_context.begin_frame(input);
        // Draw
        self.gui_run(egui_context, default_gfx_backend, glfw_backend);
        let egui::FullOutput {
            platform_output,
            textures_delta,
            shapes,
            pixels_per_point,
            ..
        } = egui_context.end_frame();
        let meshes = egui_context.tessellate(shapes, pixels_per_point);
        default_gfx_backend.render_egui(meshes, textures_delta, glfw_backend.window_size_logical);

        if glfw_backend.is_opengl() {
            use egui_overlay::egui_window_glfw_passthrough::glfw::Context as _;
            glfw_backend.window.swap_buffers();
        }

        // Limit framerate to reduce CPU usage - 60 FPS should be sufficient for a focus overlay
        // Only redraw immediately if we have pending rectangle updates
        let frame_duration = if self.receiver.is_empty() {
            // No pending updates - use adaptive frame rate based on how long since last update
            let time_since_update = self
                .last_update_time
                .map(|last| last.elapsed())
                .unwrap_or(std::time::Duration::from_secs(1));

            if time_since_update < std::time::Duration::from_millis(100) {
                // Recent update - maintain 60 FPS for smooth visual feedback
                std::time::Duration::from_millis(16)
            } else if time_since_update < std::time::Duration::from_secs(1) {
                // Moderately idle - reduce to 30 FPS
                std::time::Duration::from_millis(33)
            } else {
                // Very idle - reduce to 10 FPS to save CPU
                std::time::Duration::from_millis(100)
            }
        } else {
            // Pending updates available, redraw immediately for responsiveness
            std::time::Duration::ZERO
        };

        Some((platform_output, frame_duration))
    }
}

/// Starts the overlay and returns a sender to update the highlighted rectangle and a receiver for close events.
pub fn start_highlight_overlay() -> (mpsc::Sender<Option<Rect>>, oneshot::Receiver<()>) {
    let (tx, rx) = mpsc::channel(10);
    let (close_tx, close_rx) = oneshot::channel();

    thread::spawn(move || {
        start(FocusHighlighter::new(rx, close_tx));
    });

    (tx, close_rx)
}

/// Load the icon from the assets directory
fn load_icon() -> Option<egui_overlay::egui_window_glfw_passthrough::glfw::PixelImage> {
    use egui_overlay::egui_window_glfw_passthrough::glfw::PixelImage;

    // Try to load the icon from the assets directory
    let icon_bytes = include_bytes!("../assets/icon.ico");

    // Load the image using the image crate
    if let Ok(img) = image::load_from_memory_with_format(icon_bytes, ImageFormat::Ico) {
        let rgba_img = img.to_rgba8();
        let (width, height) = rgba_img.dimensions();
        // Convert RGBA bytes to u32 pixels (ABGR format for little-endian)
        let pixels: Vec<u32> = rgba_img
            .pixels()
            .map(|rgba| {
                let [r, g, b, a] = rgba.0;
                (a as u32) << 24 | (b as u32) << 16 | (g as u32) << 8 | (r as u32)
            })
            .collect();

        Some(PixelImage {
            width,
            height,
            pixels,
        })
    } else {
        log::warn!("Failed to load icon from assets/icon.ico");
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::sync::mpsc;

    #[test]
    fn new_has_no_rect() {
        let (_tx, rx) = mpsc::channel::<Option<Rect>>(1);
        let hl = FocusHighlighter::new(rx, oneshot::channel().0);
        assert!(hl.target_rect.is_none());
    }
}
