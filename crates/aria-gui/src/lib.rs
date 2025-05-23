use egui::epaint::RectShape;
use egui::{Context, Rect, Rgba, Shape, Stroke};
use egui_overlay::egui_render_three_d::ThreeDBackend as DefaultGfxBackend;
use egui_overlay::egui_window_glfw_passthrough::GlfwBackend;
use egui_overlay::{start, EguiOverlay};
use std::thread;
use tokio::sync::{mpsc, oneshot};

pub struct FocusHighlighter {
    target_rect: Option<Rect>,
    receiver: mpsc::Receiver<Option<Rect>>,
    initialized: bool,
    close_sender: Option<oneshot::Sender<()>>,
}

impl FocusHighlighter {
    pub fn new(receiver: mpsc::Receiver<Option<Rect>>, close_sender: oneshot::Sender<()>) -> Self {
        Self {
            target_rect: None,
            receiver,
            initialized: false,
            close_sender: Some(close_sender),
        }
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
            glfw_backend.set_passthrough(true);
            glfw_backend.glfw.with_primary_monitor(|_, monitor_opt| {
                if let Some(monitor) = monitor_opt {
                    if let Some(mode) = monitor.get_video_mode() {
                        glfw_backend.window.set_pos(0, 1);
                        glfw_backend
                            .window
                            .set_size(mode.width as i32, mode.height as i32);
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

        if let Ok(Some(rect)) = self.receiver.try_recv() {
            self.target_rect = Some(rect);
        }
        if let Some(rect) = self.target_rect {
            let painter = egui_context.layer_painter(egui::LayerId::debug());
            let stroke = Stroke::new(2.0, Rgba::RED);
            painter.add(Shape::Rect(RectShape::stroke(rect, 0.0, stroke)));
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
        // Present
        if glfw_backend.is_opengl() {
            use egui_overlay::egui_window_glfw_passthrough::glfw::Context as _;
            glfw_backend.window.swap_buffers();
        }

        // Always redraw immediately
        Some((platform_output, std::time::Duration::ZERO))
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
