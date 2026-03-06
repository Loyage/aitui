use crossterm::event::{self, Event as CrosstermEvent, KeyEvent};
use std::time::Duration;
use tokio::sync::mpsc;

#[derive(Debug)]
pub enum Event {
    Key(KeyEvent),
    ApiToken(String),
    ApiDone,
    ApiError(String),
    Tick,
}

pub struct EventLoop {
    pub rx: mpsc::UnboundedReceiver<Event>,
    tx: mpsc::UnboundedSender<Event>,
}

impl EventLoop {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { rx, tx }
    }

    pub fn sender(&self) -> mpsc::UnboundedSender<Event> {
        self.tx.clone()
    }

    pub fn start_input_loop(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            loop {
                if event::poll(Duration::from_millis(50)).unwrap_or(false) {
                    if let Ok(evt) = event::read() {
                        match evt {
                            CrosstermEvent::Key(key) => {
                                if tx.send(Event::Key(key)).is_err() {
                                    return;
                                }
                            }
                            _ => {}
                        }
                    }
                }
                // Send tick for cursor blink etc.
                if tx.send(Event::Tick).is_err() {
                    return;
                }
            }
        });
    }
}
