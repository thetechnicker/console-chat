use crate::network::NetworkEvent;
use crate::screens::CurrentScreen;
use color_eyre::eyre::OptionExt;
use crossterm::event::{Event as CrosstermEvent, KeyCode, KeyEvent, KeyModifiers};
use futures::{FutureExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;

/// The frequency at which tick events are emitted.
const TICK_FPS: f64 = 30.0; // 1.0;

/// Representation of all possible events.
#[derive(Clone, Debug)]
pub enum Event {
    /// An event that is emitted on a regular schedule.
    ///
    /// Use this event to run any code which has to run outside of being a direct response to a user
    /// event. e.g. polling exernal systems, updating animations, or rendering the UI based on a
    /// fixed frame rate.
    Tick,
    /// Crossterm events.
    ///
    /// These events are emitted by the terminal.
    Crossterm(CrosstermEvent),
    /// Application events.
    ///
    /// Use this event to emit custom events that are specific to your application.
    App(AppEvent),
}

impl From<AppEvent> for Event {
    fn from(app_event: AppEvent) -> Self {
        Self::App(app_event)
    }
}

impl From<CrosstermEvent> for Event {
    fn from(event: CrosstermEvent) -> Self {
        Self::Crossterm(event)
    }
}

/// Application events.
///
/// You can extend this enum with your own custom events.
#[derive(Clone, Debug)]
pub enum AppEvent {
    /// Quit the application.
    Quit,

    Focus,
    NoFocus,
    FocusItem(usize),
    Clear(bool),

    NetworkEvent(NetworkEvent),

    KeyEvent(KeyEvent),

    OnWidgetEnter(String, Option<Arc<[String]>>),

    SwitchScreen(CurrentScreen),

    SendMessage(String),
    SimpleMSG(String),
}

/// Terminal event handler.
#[derive(Debug)]
pub struct EventHandler {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
    /// Event receiver channel.
    receiver: mpsc::UnboundedReceiver<Event>,
    handle: tokio::task::JoinHandle<Result<(), color_eyre::eyre::Error>>,
}

impl Default for EventHandler {
    fn default() -> Self {
        Self::new()
    }
}

impl EventHandler {
    /// Constructs a new instance of [`EventHandler`] and spawns a new thread to handle events.
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::unbounded_channel();
        let actor = EventTask::new(sender.clone());
        let handle = tokio::spawn(async { actor.run().await });
        Self {
            sender,
            receiver,
            handle,
        }
    }

    pub fn get_event_sender(&self) -> EventSender {
        EventSender::new(self.sender.clone())
    }

    pub fn stop(&self) {
        self.handle.abort();
    }

    /// Receives an event from the sender.
    ///
    /// This function blocks until an event is received.
    ///
    /// # Errors
    ///
    /// This function returns an error if the sender channel is disconnected. This can happen if an
    /// error occurs in the event thread. In practice, this should not happen unless there is a
    /// problem with the underlying terminal.
    pub async fn next(&mut self) -> color_eyre::Result<Event> {
        self.receiver
            .recv()
            .await
            .ok_or_eyre("Failed to receive event")
    }

    /// Queue an app event to be sent to the event receiver.
    ///
    /// This is useful for sending events to the event handler which will be processed by the next
    /// iteration of the application's event loop.
    pub fn send(&mut self, app_event: AppEvent) {
        // Ignore the result as the reciever cannot be dropped while this struct still has a
        // reference to it
        let _ = self.sender.send(Event::App(app_event));
    }
}
///Simple struct so Screens can send back events to app
#[derive(Debug, Clone)]
pub struct EventSender {
    sender: mpsc::UnboundedSender<Event>,
}

impl EventSender {
    pub fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }
    pub fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}

impl From<EventSender> for AppEventSender {
    fn from(val: EventSender) -> Self {
        AppEventSender::new(val.sender)
    }
}

impl From<EventSender> for NetworkEventSender {
    fn from(val: EventSender) -> Self {
        NetworkEventSender::new(val.sender)
    }
}

#[derive(Debug, Clone)]
pub struct AppEventSender {
    sender: mpsc::UnboundedSender<Event>,
}

impl AppEventSender {
    pub fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }
    pub fn send(&self, event: AppEvent) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(Event::App(event));
    }
}

#[derive(Debug, Clone)]
pub struct NetworkEventSender {
    sender: mpsc::UnboundedSender<Event>,
}

impl NetworkEventSender {
    pub fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }
    pub fn send(&self, event: NetworkEvent) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(Event::App(AppEvent::NetworkEvent(event)));
    }
}

/// A thread that handles reading crossterm events and emitting tick events on a regular schedule.
struct EventTask {
    /// Event sender channel.
    sender: mpsc::UnboundedSender<Event>,
}

/*
async fn get_event() -> Option<CrosstermEvent> {
    if event::poll(Duration::from_millis(1000)).unwrap_or(false) {
        if let Ok(event) = event::read() {
            return Some(event);
        }
    }
    None
}
*/

impl EventTask {
    /// Constructs a new instance of [`EventThread`].
    fn new(sender: mpsc::UnboundedSender<Event>) -> Self {
        Self { sender }
    }

    /// Runs the event thread.
    ///
    /// This function emits tick events at a fixed rate and polls for crossterm events in between.
    async fn run(self) -> color_eyre::Result<()> {
        let tick_rate = Duration::from_secs_f64(1.0 / TICK_FPS);
        let mut reader = crossterm::event::EventStream::new();
        let mut tick = tokio::time::interval(tick_rate);
        loop {
            let tick_delay = tick.tick();
            //let crossterm_event = get_event();
            let crossterm_event = reader.next().fuse();
            tokio::select! {
              _ = self.sender.closed() => {
                break;
              }
              _ = tick_delay => {
                self.send(Event::Tick);
              }
              Some(Ok(evt)) = crossterm_event => match evt {
                    crossterm::event::Event::Key(key_event) => {
                        self.send(handle_key_events(key_event))
                    }
                    _ => self.send(Event::Crossterm(evt))
                },
            };
        }
        Ok(())
    }

    /// Sends an event to the receiver.
    fn send(&self, event: Event) {
        // Ignores the result because shutting down the app drops the receiver, which causes the send
        // operation to fail. This is expected behavior and should not panic.
        let _ = self.sender.send(event);
    }
}

#[inline]
fn handle_key_events(key_event: KeyEvent) -> Event {
    match key_event.code {
        KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
            Event::App(AppEvent::Quit)
        }
        _ => Event::App(AppEvent::KeyEvent(key_event)),
        //_ if key_event.is_repeat() => Event::App(AppEvent::KeyEvent(key_event)),
        //_ => Event::Crossterm(CrosstermEvent::Key(key_event)),
    }
}

#[cfg(test)]
pub mod test_utils {
    use super::*;

    ///Object
    #[derive(Debug)]
    pub struct EventReceiver {
        receiver: mpsc::UnboundedReceiver<Event>,
    }

    impl EventReceiver {
        pub async fn next(&mut self) -> color_eyre::Result<Event> {
            self.receiver
                .recv()
                .await
                .ok_or_eyre("Failed to receive event")
        }
    }

    /// Used for testing only
    pub fn dummy_event_sender() -> (EventSender, EventReceiver) {
        let (tx, rx) = mpsc::unbounded_channel();
        (EventSender { sender: tx }, EventReceiver { receiver: rx })
    }
}
