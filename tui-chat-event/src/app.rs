use crate::event::{AppEvent, Event, EventHandler, WidgetEvent};
use crate::screens;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub events: EventHandler,
    pub current_screen: screens::CurrentScreen,
    pub chat_screen: screens::ChatScreen,
    pub login_screen: screens::LoginScreen,
    pub last_event: Option<AppEvent>,
}

impl Default for App {
    fn default() -> Self {
        let event_handler = EventHandler::new();
        let event_sender = event_handler.get_event_sender();
        Self {
            running: true,
            events: event_handler,
            current_screen: screens::CurrentScreen::default(),
            chat_screen: screens::ChatScreen::new(event_sender.clone()),
            login_screen: screens::LoginScreen::new(event_sender.clone()),
            last_event: None,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        //self.events.send(AppEvent::SwitchScreen(screens::CurrentScreen::Chat));
        while self.running {
            terminal.draw(|frame| match self.current_screen {
                screens::CurrentScreen::Login => {
                    frame.render_widget(&self.login_screen, frame.area())
                }
                screens::CurrentScreen::Chat => {
                    frame.render_widget(&self.chat_screen, frame.area())
                }
            })?;

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    ratatui::crossterm::event::Event::Key(key_event) => {
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => {
                    self.last_event = Some(app_event.clone());
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::WidgetEvent(w_event) => self.send_current_screen(w_event),
                        AppEvent::SwitchScreen(new_screen) => self.current_screen = new_screen,
                        _ => {}
                    };
                }
            }
        }
        Ok(())
    }

    fn send_current_screen(&mut self, event: WidgetEvent) {
        if let Some(screen) = self.get_current_screen() {
            screen.handle_event(event);
        }
    }
    pub fn get_current_screen(&mut self) -> Option<&mut dyn screens::Screen> {
        match self.current_screen {
            screens::CurrentScreen::Chat => Some(&mut self.chat_screen as &mut dyn screens::Screen),
            screens::CurrentScreen::Login => {
                Some(&mut self.login_screen as &mut dyn screens::Screen)
            } //_ => None,
        }
    }
    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            // Other handlers you could add here.
            _ => {
                self.events
                    .send(AppEvent::WidgetEvent(WidgetEvent::KeyEvent(key_event)));
            }
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
