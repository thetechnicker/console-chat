use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, Event, EventHandler};
use crate::network;
use crate::screens;
use crossterm::event::Event as CrosstermEvent;
use log::error;
use ratatui::DefaultTerminal;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::{Block, BorderType, Paragraph},
};
use std::sync::Arc;
use tokio::sync::Mutex;

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub events: EventHandler,
    pub current_screen: screens::CurrentScreen,
    pub chat_screen: screens::ChatScreen,
    pub login_screen: screens::LoginScreen,
    pub api: Option<Arc<Mutex<network::client::ApiClient>>>,
    pub last_event: Option<AppEvent>,
    help: String,
    exit: Option<std::time::Instant>,
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
            api: network::client::ApiClient::new("http://localhost:8000").ok(),
            last_event: None,
            help: String::new(),
            exit: None,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(
        mut self,
        mut terminal: DefaultTerminal,
    ) -> color_eyre::Result<Option<std::time::Duration>> {
        //self.events.send(AppEvent::SwitchScreen(screens::CurrentScreen::Chat));
        while self.running {
            //let start = std::time::Instant::now();
            terminal.draw(|frame| self.render(frame))?;

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    CrosstermEvent::Resize(_, _) => {
                        terminal.draw(|frame| self.render(frame))?;
                    }
                    _ => {}
                },
                Event::App(app_event) => {
                    self.last_event = Some(app_event.clone());
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::SwitchScreen(new_screen) => self.current_screen = new_screen,
                        AppEvent::SimpleMSG(str) => self.help += &(str + "\n"),
                        AppEvent::ButtonPress(str) => match str.as_str() {
                            "LOGIN" => {
                                let sender = self.events.get_event_sender();
                                let login = self.login_screen.get_login_data();
                                //self.help += &format!("sender: {:?}\nlogin: {:?}\n", sender, login);
                                if let Some(api) = self.api.as_ref() {
                                    let api_clone = Arc::clone(api);
                                    tokio::spawn(async move {
                                        let mut api = api_clone.lock().await;
                                        let resp = api.auth(Some(login)).await;
                                        sender.send(Event::App(AppEvent::SimpleMSG(format!(
                                            "{:?}",
                                            resp
                                        ))));
                                        match resp {
                                            Err(e) => {
                                                error!("Error: {e}")
                                            }
                                            Ok(_) => {}
                                        }
                                    });
                                }
                            }
                            _ => {}
                        },
                        _ => self.send_current_screen(app_event),
                    };
                }
            }
            //let duration = start.elapsed();
            //self.help = format!("{:?}", duration);
        }
        if let Some(exit) = self.exit {
            return Ok(Some(exit.elapsed()));
        }
        Ok(None)
    }

    fn render(&self, frame: &mut Frame) {
        let area = frame.area();
        let outer_block = Block::bordered()
            .border_type(BorderType::Double)
            .title("Console-CHAT")
            .title_alignment(Alignment::Center);
        let inner = outer_block.inner(area);

        frame.render_widget(outer_block, area);

        let [left, main, right] = Layout::horizontal([
            Constraint::Max(1),
            Constraint::Percentage(40),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // LEFT

        let left_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _left_inner = left_block.inner(left);
        frame.render_widget(left_block, left);

        // RIGHT

        let right_block = Block::bordered().border_type(DEFAULT_BORDER);
        let right_inner = right_block.inner(right);
        frame.render_widget(right_block, right);
        let x = Paragraph::new(format!("{}", self.help));
        frame.render_widget(x, right_inner);

        match self.current_screen {
            screens::CurrentScreen::Login => frame.render_widget(&self.login_screen, main),
            screens::CurrentScreen::Chat => frame.render_widget(&self.chat_screen, main),
        }
    }

    fn send_current_screen(&mut self, event: AppEvent) {
        if let Some(screen) = self.get_current_screen() {
            screen.handle_event(event);
        }
    }
    pub fn get_current_screen(&mut self) -> Option<&mut dyn screens::Screen> {
        match self.current_screen {
            screens::CurrentScreen::Chat => Some(&mut self.chat_screen as &mut dyn screens::Screen),
            screens::CurrentScreen::Login => {
                Some(&mut self.login_screen as &mut dyn screens::Screen)
            }
        }
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.exit = Some(std::time::Instant::now());
        self.events.stop();
        self.running = false;
    }
}
