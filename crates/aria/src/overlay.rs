use aria_core::driver::WindowsDriver;
use iced::event::{self};
use iced::theme::Palette;
use iced::widget::{center, Column};
use iced::{window, Color, Event, Theme};
use iced::{Center, Element, Subscription, Task};

#[derive(Debug, Default)]
pub struct Overlay {
    last: Vec<Event>,
    enabled: bool,
}

#[derive(Debug, Clone)]
pub enum Message {
    EventOccurred(Event),
    Toggled(bool),
    Exit,
}

impl Overlay {
    pub fn new() -> (Self, Task<Message>) {
        let task = window::get_latest().and_then(|id| window::maximize(id, false));
        (Self::default(), task)
    }

    pub fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::EventOccurred(event) if self.enabled => {
                self.last.push(event);

                if self.last.len() > 5 {
                    let _ = self.last.remove(0);
                }

                Task::none()
            }
            Message::EventOccurred(event) => {
                if let Event::Window(window::Event::CloseRequested) = event {
                    window::get_latest().and_then(window::close)
                } else {
                    Task::none()
                }
            }
            Message::Toggled(enabled) => {
                self.enabled = enabled;

                Task::none()
            }
            Message::Exit => {
                WindowsDriver::stop();

                return window::get_latest().and_then(window::close);
            }
        }
    }

    pub fn subscription(&self) -> Subscription<Message> {
        event::listen().map(Message::EventOccurred)
    }

    pub fn view(&self) -> Element<Message> {
        let content = Column::new().align_x(Center);

        center(content).into()
    }

    pub fn theme(&self) -> Theme {
        Theme::custom(
            "main".to_string(),
            Palette {
                background: Color::TRANSPARENT,
                ..Theme::default().palette()
            },
        )
    }
}
