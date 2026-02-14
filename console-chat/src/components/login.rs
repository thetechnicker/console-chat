use crate::action::AppError;
use crate::action::Result;
use crate::components::{EventWidget, button::*, render_nice_bg, theme::*, vim::*};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;

use super::Component;
use crate::{
    action::Action, action::ActionSubsetWrapper, action::ButtonEvent, action::VimEvent,
    config::Config,
};

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Login;

#[derive(Default)]
pub struct Login<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    theme: Theme,
    username: VimWidget<'a>,
    password: VimWidget<'a>,
    login: Button,
    register_button: Button,
    exit: Button,
    index: usize,
    register: bool,
    size: Size,
}

impl Login<'_> {
    pub const MAX_ELEMENTS: usize = 4;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) -> Result<()> {
        let size = self.size;
        let config = self.config.clone();
        *self = Self::default();
        self.register_config_handler(config)?;
        self.init(size)
    }

    fn up(&mut self) {
        self.deselect_current_element();
        self.index = if self.index == 0 {
            Self::MAX_ELEMENTS - 1
        } else {
            self.index - 1
        };
        self.select_current_element();
    }

    fn down(&mut self) {
        self.deselect_current_element();
        self.index = (self.index + 1) % Self::MAX_ELEMENTS;
        self.select_current_element();
    }

    fn deselect_current_element(&mut self) {
        let index = self.index;
        self.get_inputs()[index].deselect();
    }
    fn select_current_element(&mut self) {
        let index = self.index;
        self.get_inputs()[index].select();
    }

    const fn get_inputs(&mut self) -> [&mut dyn EventWidget; 4] {
        [
            &mut self.username,
            &mut self.password,
            if self.register {
                &mut self.register_button
            } else {
                &mut self.login
            },
            &mut self.exit,
        ]
    }

    fn render_normal(&mut self, area: Rect, buf: &mut Buffer) {
        let [a, b, c, d] = Layout::vertical([
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            //Constraint::Fill(1),
        ])
        .areas(area);

        self.username.render(a, buf);
        self.password.render(b, buf);

        self.login.draw_button(c, buf);
        self.exit.draw_button(d, buf);
    }
    fn render_register(&mut self, area: Rect, buf: &mut Buffer) {
        let [_lable, a, b, c, d] = Layout::vertical([
            Constraint::Length(1),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            Constraint::Max(3),
            //Constraint::Fill(1),
        ])
        .areas(area);

        self.username.render(a, buf);
        self.password.render(b, buf);

        self.register_button.draw_button(c, buf);
        self.exit.draw_button(d, buf);
    }
}

impl<'a> Component for Login<'a> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        {
            let theme = match self.config.themes.get(&STYLE_KEY) {
                Some(themes) => themes,
                None => match self.config.themes.get(&crate::app::Mode::Global) {
                    Some(themes) => themes,
                    None => {
                        self.config
                            .themes
                            .insert(crate::app::Mode::Global, Theme::default());
                        self.config
                            .themes
                            .get(&crate::app::Mode::Global)
                            .ok_or("This is bad")?
                    }
                },
            };
            self.theme = theme.clone();
            self.username = VimWidget::new("Username", VimType::SingleLine, theme.vi);
            self.password = VimWidget::new("Password", VimType::SingleLine, theme.vi).password();

            self.password.set_block(VimMode::Normal.highlight_block());

            self.login = Button::new(
                "Login",
                "",
                theme.buttons.accepting,
                ButtonEvent::TriggerLogin,
            );
            self.register_button = Button::new(
                "Register",
                "",
                theme.buttons.accepting,
                ButtonEvent::TriggerRegister,
            );
            //self.register_button = Button::new(
            //    "Login instead",
            //    "",
            //    theme.buttons.accepting,
            //    ButtonEvent::LoginInstead,
            //);
            self.exit = Button::new("Abort", "<q>", theme.buttons.denying, ButtonEvent::OpenHome);
        }
        self.select_current_element();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            let username = self.username.lines()[0].clone().trim().to_owned();
            let password = self.password.lines()[0].clone().trim().to_owned();
            let index = self.index;
            let inputs = self.get_inputs();
            let input_event = inputs[index].handle_event(key).clone()?;
            if let Some(event) = input_event {
                debug!("Handling event: {:?}", event);
                match event {
                    ActionSubsetWrapper::VimEvent(vim_event) => match vim_event {
                        VimEvent::Normal => return Ok(Some(Action::Normal)),
                        VimEvent::Insert => return Ok(Some(Action::Insert)),
                        VimEvent::Enter(_) => {
                            self.down();
                            return Ok(Some(Action::Normal));
                        }
                        VimEvent::Up => self.up(),
                        VimEvent::Down => self.down(),
                        VimEvent::Nop | VimEvent::StoreConfig(_) => {}
                    },
                    ActionSubsetWrapper::ButtonEvent(ButtonEvent::LoginInstead) => {
                        self.reset()?;
                    }
                    ActionSubsetWrapper::ButtonEvent(ButtonEvent::TriggerLogin) => {
                        let login_action = match (username.is_empty(), password.is_empty()) {
                            (true, true) => Action::Error(AppError::MissingPasswordAndUsername),
                            (false, true) => Action::Error(AppError::MissingPassword),
                            (true, false) => Action::Error(AppError::MissingUsername),
                            (false, false) => {
                                self.reset()?;
                                Action::PerformLogin(username, password)
                            }
                        };
                        return Ok(Some(login_action));
                    }
                    ActionSubsetWrapper::ButtonEvent(ButtonEvent::TriggerRegister) => {
                        let login_action = match (username.is_empty(), password.is_empty()) {
                            (true, true) => Action::Error(AppError::MissingPasswordAndUsername),
                            (false, true) => Action::Error(AppError::MissingPassword),
                            (true, false) => Action::Error(AppError::MissingUsername),
                            (false, false) => {
                                self.reset()?;
                                Action::PerformRegister(username, password)
                            }
                        };
                        return Ok(Some(login_action));
                    }
                    ActionSubsetWrapper::ButtonEvent(event) => {
                        return Ok(Some(event.into()));
                    }
                    _ => {}
                }
            } else {
                match key.code {
                    KeyCode::Char('k') => self.up(),
                    KeyCode::Char('j') => self.down(),
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenLogin => self.active = true,
            Action::LoginFailure => {
                self.active = true;
                self.register = true;
                self.login = Button::new(
                    "Login",
                    "",
                    self.theme.buttons.accepting,
                    ButtonEvent::TriggerLogin,
                );
            }
            Action::Tick => {
                // add any logic here that should run on every tick
                if self.login.is_active() {
                    self.login.set_state(ButtonState::Selected);
                }
                if self.exit.is_active() {
                    self.exit.set_state(ButtonState::Selected);
                }
            }
            Action::Render => {
                // add any logic here that should run on every render
            }
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.active {
            let buf = frame.buffer_mut();
            let block = Block::new().bg(Color::Blue);
            block.render(area, buf);

            let center = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Max(3 * 4 + 2 + if self.register { 1 } else { 0 }),
                Constraint::Fill(1),
            ])
            .split(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(40),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            )[1];

            let center = render_nice_bg(center, self.theme.page, buf);

            if self.register {
                self.render_register(center, buf)
            } else {
                self.render_normal(center, buf)
            }
        }
        Ok(())
    }
}
