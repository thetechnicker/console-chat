use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, Event, EventHandler};
use crate::network;
use crate::screens;
use crossterm::event::Event as CrosstermEvent;
use log::{error, info};
use ratatui::DefaultTerminal;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::{Block, BorderType},
};
use std::sync::Arc;
//use tokio::sync::Mutex;

/// Application.
#[derive(Debug)]
pub struct App {
    running: bool,
    events: EventHandler,

    current_screen: screens::CurrentScreen,
    chat_screen: screens::ChatScreen,
    login_screen: screens::LoginScreen,
    home_screen: screens::HomeScreen,

    exit_time: Option<std::time::Instant>,

    api: Option<network::client::ApiClientType>,
    max_api_failure_count: u32,
    api_failure_count: u32,
    api_failures: Vec<network::ApiError>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(max_api_failure_count: Option<u32>, server_url: Option<&str>) -> Self {
        // Creating a Client might fail, this shouldnt be a reason to crash the app
        let mut api_failures = Vec::new();

        let mut api_failure_count = 0;
        let api_client_res =
            network::client::ApiClient::new(server_url.unwrap_or("https://localhost:8000"));
        let mut api = None;
        match api_client_res {
            Ok(a) => api = Some(a),
            Err(e) => {
                error!("{}", e);
                api_failure_count += 1;
                api_failures.push(e)
            }
        }

        let event_handler = EventHandler::new();
        let event_sender = event_handler.get_event_sender();

        Self {
            running: true,
            events: event_handler,
            current_screen: screens::CurrentScreen::default(),
            chat_screen: screens::ChatScreen::new(event_sender.clone()),
            login_screen: screens::LoginScreen::new(event_sender.clone()),
            home_screen: screens::HomeScreen::new(event_sender.clone()),
            api,

            exit_time: None,
            api_failure_count,
            max_api_failure_count: max_api_failure_count.unwrap_or(10),
            api_failures,
        }
    }

    pub fn set_ap_url(url: &str) -> Self {
        Self::new(None, Some(url))
    }

    pub fn set_max_error(max_api_failure_count: u32) -> Self {
        Self::new(Some(max_api_failure_count), None)
    }

    fn get_api(&mut self) -> Option<&network::client::ApiClientType> {
        self.api.as_ref()
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
                    //self.last_event = Some(app_event.clone());
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::SwitchScreen(new_screen) => self.current_screen = new_screen,
                        AppEvent::SimpleMSG(str) => info!("{}", str),
                        AppEvent::TriggerApiReconnect => self.reconnect_api(),
                        AppEvent::ButtonPress(str) => match str.as_str() {
                            "LOGIN" => {
                                let sender = self.events.get_event_sender();
                                let login = self.login_screen.get_login_data();
                                //self.help += &format!("sender: {:?}\nlogin: {:?}\n", sender, login);
                                if let Some(api) = self.get_api() {
                                    let api_clone = Arc::clone(api);
                                    tokio::spawn(async move {
                                        let mut api = api_clone.lock().await;
                                        let resp = api.auth(Some(login)).await;
                                        match resp {
                                            Err(e) => {
                                                error!("Error: {e}")
                                            }
                                            Ok(_) => {
                                                sender.send(Event::App(AppEvent::SwitchScreen(
                                                    screens::CurrentScreen::Home,
                                                )))
                                            }
                                        }
                                    });
                                }
                            }
                            "LOGOUT" => {
                                if let Some(api) = self.api.as_ref() {
                                    api.lock().await.reset();
                                    self.events.send(AppEvent::SwitchScreen(
                                        screens::CurrentScreen::Login,
                                    ));
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
        if let Some(exit) = self.exit_time {
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
            Constraint::Fill(1),
            Constraint::Percentage(60),
            Constraint::Fill(1),
        ])
        .areas(inner);

        // LEFT

        let left_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _left_inner = left_block.inner(left);
        frame.render_widget(left_block, left);

        // RIGHT

        let right_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _right_inner = right_block.inner(right);
        frame.render_widget(right_block, right);

        match self.current_screen {
            screens::CurrentScreen::Login => frame.render_widget(&self.login_screen, main),
            screens::CurrentScreen::Chat => frame.render_widget(&self.chat_screen, main),
            screens::CurrentScreen::Home => frame.render_widget(&self.home_screen, main),
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
            screens::CurrentScreen::Home => Some(&mut self.home_screen as &mut dyn screens::Screen),
        }
    }

    pub fn reconnect_api(&mut self) {
        if self.api.is_none() {
            let api_client_res = network::client::ApiClient::new("http://localhost:8000");
            match api_client_res {
                Ok(a) => self.api = Some(a),
                Err(e) => {
                    error!("{}", e);
                    self.api_failure_count += 1;
                    self.api_failures.push(e)
                }
            }
        }
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {
        if self.api_failure_count >= self.max_api_failure_count {
            self.events.send(AppEvent::Quit);
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.exit_time = Some(std::time::Instant::now());
        self.events.stop();
        self.running = false;
    }
}
