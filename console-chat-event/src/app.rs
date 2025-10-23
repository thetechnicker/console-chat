use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, Event, EventHandler};
use crate::network::{self, ApiError};
use crate::screens::{self, Screen};
use crossterm::event::Event as CrosstermEvent;
use log::{error, info};
use ratatui::DefaultTerminal;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    widgets::{Block, BorderType, Paragraph, Wrap},
};

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
    api: network::client::ApiClient,

    error_box: Option<ApiError>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(server_url: Option<&str>, _max_api_failure_count: Option<u32>) -> Self {
        let url = server_url.unwrap_or("http://localhost:8000");

        let event_handler = EventHandler::new();
        let event_sender = event_handler.get_event_sender();
        let api =
            network::client::ApiClient::new(url, event_sender.clone().into()).unwrap_or_else(|e| {
                error!("ApiClient initialization failed: {e}");
                panic!("ApiClient initialization failed: {e}");
            });

        Self {
            running: true,
            events: event_handler,
            current_screen: screens::CurrentScreen::default(),
            chat_screen: screens::ChatScreen::new(event_sender.clone()),
            login_screen: screens::LoginScreen::new(event_sender.clone()),
            home_screen: screens::HomeScreen::new(event_sender.clone()),

            exit_time: None,
            api,
            error_box: None,
        }
    }

    pub fn with_api_url(url: &str) -> Self {
        Self::new(Some(url), None)
    }

    pub fn with_max_error(max_api_failure_count: u32) -> Self {
        Self::new(None, Some(max_api_failure_count))
    }

    /*
    fn get_api(&mut self) -> Option<&network::client::ApiClientType> {
        self.api.as_ref()
    }
    */

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
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::SwitchScreen(new_screen) => self.current_screen = new_screen,
                        AppEvent::SimpleMSG(str) => info!("{}", str),
                        AppEvent::NetworkEvent(network::NetworkEvent::Error(e)) => {
                            self.error_box = Some(e);
                        }
                        //AppEvent::TriggerApiReconnect => self.reconnect_api(),
                        AppEvent::ButtonPress(str) => match str.as_str() {
                            "LOGIN" => {
                                let login = self.login_screen.get_data();
                                match self.api.auth(Some(login)).await {
                                    Ok(()) => self
                                        .events
                                        .send(AppEvent::SwitchScreen(screens::CurrentScreen::Home)),

                                    Err(e) => {
                                        error!("Error when logging in: {e}");
                                        self.events.send(AppEvent::NetworkEvent(
                                            network::NetworkEvent::Error(e),
                                        ));
                                    }
                                }
                            }
                            "LOGOUT" => {
                                self.api.reset();
                                self.events
                                    .send(AppEvent::SwitchScreen(screens::CurrentScreen::Login));
                            }
                            /*
                                                        "JOIN" => {
                                                            let sender = self.events.get_event_sender();
                                                            let room_val = self.home_screen.get_data();
                                                            let room = if let Some(room) = room_val.as_str() {
                                                                room
                                                            } else {
                                                                break;
                                                            };
                                                            if let Some(api) = self.api.as_ref() {
                                                                let api_clone = Arc::clone(api);
                                                                let locked_api = api_clone.lock().await;
                                                                let mut resp = locked_api.listen(room).await;
                                                                match resp {
                                                                    Err(e) => {
                                                                        error!("Error: {e}")
                                                                    }
                                                                    Ok(_) => {
                                                                        sender.send(Event::App(AppEvent::SwitchScreen(
                                                                            screens::CurrentScreen::Chat,
                                                                        )));

                                                                        //tokio::spawn(async move {
                                                                        //    while let Some(chunk) = resp.next().await {
                                                                        //        sender.send(Event::App(AppEvent::NetworkEvent));
                                                                        //    }
                                                                        //});
                                                                    }
                                                                }
                                                            }
                                                        }
                            */
                            str => {
                                info!("Unhandled Button: {str}")
                            }
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
            Constraint::Max(1),
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
        let right_inner = right_block.inner(right);
        frame.render_widget(right_block, right);
        let error_box = Paragraph::new(
            self.error_box
                .clone()
                .map_or(String::new(), |e| format!("{e}")),
        )
        .wrap(Wrap { trim: true });
        frame.render_widget(error_box, right_inner);
        //match self.current_screen {
        //    screens::CurrentScreen::Login => frame.render_widget(&self.login_screen, main),
        //    screens::CurrentScreen::Chat => frame.render_widget(&self.chat_screen, main),
        //    screens::CurrentScreen::Home => frame.render_widget(&self.home_screen, main),
        //}
        let mut cursor = None;
        if let Some(screen) = self.get_current_screen() {
            let buf = frame.buffer_mut();
            cursor = screen.draw(main, buf);
        }
        info!("{cursor:?}");
        if let Some(cursor) = cursor {
            frame.set_cursor_position((cursor.x, cursor.y));
        }
    }

    fn send_current_screen(&mut self, event: AppEvent) {
        if let Some(screen) = self.get_current_screen_mut() {
            screen.handle_event(event);
        }
    }
    pub fn get_current_screen(&self) -> Option<&dyn screens::Screen> {
        match self.current_screen {
            screens::CurrentScreen::Chat => Some(&self.chat_screen as &dyn screens::Screen),
            screens::CurrentScreen::Login => Some(&self.login_screen as &dyn screens::Screen),
            screens::CurrentScreen::Home => Some(&self.home_screen as &dyn screens::Screen),
        }
    }
    pub fn get_current_screen_mut(&mut self) -> Option<&mut dyn screens::Screen> {
        match self.current_screen {
            screens::CurrentScreen::Chat => Some(&mut self.chat_screen as &mut dyn screens::Screen),
            screens::CurrentScreen::Login => {
                Some(&mut self.login_screen as &mut dyn screens::Screen)
            }
            screens::CurrentScreen::Home => Some(&mut self.home_screen as &mut dyn screens::Screen),
        }
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.exit_time = Some(std::time::Instant::now());
        self.events.stop();
        self.running = false;
    }
}
