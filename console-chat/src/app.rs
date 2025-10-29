use crate::DEFAULT_BORDER;
use crate::event::{AppEvent, Event, EventHandler};
use crate::network::{self, ApiError};
use crate::screens::{self, Screen};
use crossterm::event::Event as CrosstermEvent;
use log; //::{debug, error, info, trace};
use ratatui::DefaultTerminal;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Layout},
    style::{Color, Modifier, Style},
    widgets::{Block, BorderType, Clear, Padding, Paragraph, Wrap},
};

#[derive(Debug)]
struct Popup {
    pub content: ApiError,
    pub timeout: std::time::Duration,
    pub creation: std::time::Instant,
}

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

    error_box: Option<Popup>,
    error_qeue: Vec<ApiError>,
}

impl Default for App {
    fn default() -> Self {
        Self::new(None, None)
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new(server_url: Option<&str>, _max_api_failure_count: Option<u32>) -> Self {
        let url = server_url.unwrap_or("https://localhost");

        let event_handler = EventHandler::new();
        let event_sender = event_handler.get_event_sender();
        let api =
            network::client::ApiClient::new(url, event_sender.clone().into()).unwrap_or_else(|e| {
                log::error!("ApiClient initialization failed: {e}");
                panic!("ApiClient initialization failed: {e}");
            });

        Self {
            running: true,
            events: event_handler,
            current_screen: screens::CurrentScreen::default(),
            chat_screen: screens::ChatScreen::new(event_sender.clone().into()),
            login_screen: screens::LoginScreen::new(event_sender.clone()),
            home_screen: screens::HomeScreen::new(event_sender.clone()),

            exit_time: None,
            api,
            error_box: None,
            error_qeue: Vec::new(),
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

            let event = self.events.next().await?;
            log::trace!("Handling Event: {event:?}");
            match event {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => {
                    if let CrosstermEvent::Resize(_, _) = event {
                        terminal.draw(|frame| self.render(frame))?;
                    }
                }
                Event::App(app_event) => {
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::SwitchScreen(new_screen) => {
                            self.current_screen = new_screen;
                            self.events.send(AppEvent::Clear(true));
                        }
                        AppEvent::SimpleMSG(str) => log::info!("{}", str),
                        AppEvent::NetworkEvent(network::NetworkEvent::Error(e)) => {
                            self.handle_network_error(e);
                        }
                        AppEvent::NetworkEvent(network::NetworkEvent::RequestReconnect) => {
                            if self.current_screen == screens::CurrentScreen::Chat
                                && let Err(e) = self.api.listen_reconnect().await
                            {
                                self.handle_network_error(e)
                            }
                        }
                        AppEvent::SendMessage(msg) => {
                            log::debug!("Sending: {}", msg);
                            if let Err(e) = self.api.send(&msg).await {
                                self.handle_network_error(e)
                            }
                        }
                        AppEvent::ButtonPress(str) => match str.as_str() {
                            _ if str.starts_with("LOGIN") => {
                                let login = if str == "LOGIN_ANONYM" {
                                    None
                                } else {
                                    Some(self.login_screen.get_data())
                                };
                                match self.api.auth(login).await {
                                    Ok(()) => self
                                        .events
                                        .send(AppEvent::SwitchScreen(screens::CurrentScreen::Home)),

                                    Err(e) => {
                                        log::error!("Error when logging in: {e}");
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
                            "JOIN" => {
                                let room_val = self.home_screen.get_data();
                                if let Some(room) = room_val.as_str() {
                                    if let Err(e) = self.api.listen(room).await {
                                        self.handle_network_error(e);
                                    } else {
                                        self.events.send(AppEvent::SwitchScreen(
                                            screens::CurrentScreen::Chat,
                                        ));
                                    }
                                }
                            }
                            /*
                             */
                            str => {
                                log::info!("Unhandled Button: {str}")
                            }
                        },
                        AppEvent::KeyEvent(k) if self.error_box.is_none() => {
                            self.send_to_current_screen(AppEvent::KeyEvent(k))
                        }
                        AppEvent::KeyEvent(k) if self.error_box.is_some() => {
                            if k.is_press() {
                                let mut reset_err_box = false;
                                if let Some(err) = self.error_box.as_ref()
                                    && err.creation.elapsed() > std::time::Duration::from_millis(20)
                                {
                                    reset_err_box = true;
                                }
                                if reset_err_box {
                                    self.error_box = None
                                }
                            }
                        }
                        _ => {
                            log::debug!("Unhandled Event: {app_event:#?}");
                            self.send_to_current_screen(app_event);
                        }
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

    fn handle_network_error(&mut self, e: ApiError) {
        log::error!("Network Error: {e}");
        if self.error_box.is_some() {
            self.error_qeue.push(e);
            return;
        }
        self.error_box = Some(Popup {
            content: e,
            timeout: std::time::Duration::from_secs(5),
            creation: std::time::Instant::now(),
        });
        self.events.send(AppEvent::Clear(false));
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

        // Render Main Screen
        let mut cursor = None;
        if let Some(screen) = self.get_current_screen() {
            let buf = frame.buffer_mut();
            cursor = screen.draw(main, buf);
        }
        log::trace!("{cursor:?}");
        if let Some(cursor) = cursor {
            frame.set_cursor_position((cursor.x, cursor.y));
        }

        // Debug Popup
        if let Some(e) = self.error_box.as_ref() {
            let [_, center, _] =
                Layout::vertical([Constraint::Fill(1), Constraint::Min(5), Constraint::Fill(1)])
                    .areas(
                        Layout::horizontal([
                            Constraint::Fill(1),
                            Constraint::Percentage(60),
                            Constraint::Fill(1),
                        ])
                        .split(main)[1],
                    );
            frame.render_widget(Clear, center);
            let error_text = format!("⚠️  {}", e.content);
            let e_box = Paragraph::new(error_text)
                .style(
                    Style::default()
                        .fg(Color::Red)
                        .bg(Color::Black)
                        .add_modifier(Modifier::BOLD),
                )
                .alignment(Alignment::Center)
                .wrap(Wrap { trim: true })
                .block(
                    Block::bordered()
                        .border_type(BorderType::Double)
                        .title("Error")
                        .title_alignment(Alignment::Center)
                        .border_style(Style::default().fg(Color::Red))
                        .padding(Padding::new(1, 1, 1, 1)),
                );

            frame.render_widget(e_box, center);
        }
    }

    fn send_to_current_screen(&mut self, event: AppEvent) {
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
    pub fn tick(&mut self) {
        // Popup
        {
            let mut del_error_box = false;
            if let Some(e) = self.error_box.as_mut()
                && e.creation.elapsed() > e.timeout
            {
                del_error_box = true;
            }
            if del_error_box {
                self.error_box = None;
                if let Some(e) = self.error_qeue.pop() {
                    self.handle_network_error(e)
                }
            }
        }
    }

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.exit_time = Some(std::time::Instant::now());
        self.events.stop();
        self.running = false;
    }
}
