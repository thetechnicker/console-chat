use crate::action::AppError;
use crate::action::Result;
use crate::components::{EventWidget, button::*, render_nice_bg, theme::*, vim::*};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

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
    theme: PageColors,
    username: VimWidget<'a>,
    password: VimWidget<'a>,
    login: Button,
    exit: Button,
    index: usize,
    size: Size,
}

impl Login<'_> {
    pub const MAX_ELEMENTS: usize = 4;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) -> Result<()> {
        let size = self.size;
        *self = Self::default();
        self.init(size)
    }

    fn up(&mut self) {
        self.index = if self.index == 0 {
            Self::MAX_ELEMENTS - 1
        } else {
            self.index - 1
        };
        self.update_elements();
    }

    fn down(&mut self) {
        self.index = (self.index + 1) % Self::MAX_ELEMENTS;
        self.update_elements();
    }

    fn update_elements(&mut self) {
        self.login.set_state(ButtonState::Normal);
        self.exit.set_state(ButtonState::Normal);
        self.username
            .set_block(Block::default().borders(Borders::ALL).title("Username"));
        self.password
            .set_block(Block::default().borders(Borders::ALL).title("Password"));
        match self.index {
            0 => self.username.set_block(VimMode::Normal.highlight_block()),
            1 => self.password.set_block(VimMode::Normal.highlight_block()),
            2 => {
                self.login.set_state(ButtonState::Selected);
                self.exit.set_state(ButtonState::Normal);
            }
            3 => {
                self.exit.set_state(ButtonState::Selected);
                self.login.set_state(ButtonState::Normal);
            }
            _ => {
                self.index %= Self::MAX_ELEMENTS;
            }
        }
    }

    const fn get_inputs(&mut self) -> [&mut dyn EventWidget; 4] {
        [
            &mut self.username,
            &mut self.password,
            &mut self.login,
            &mut self.exit,
        ]
    }

    fn send(&mut self, action: Action) {
        if let Some(action_tx) = self.command_tx.as_ref() {
            let _ = action_tx.send(action);
        }
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
            self.theme = theme.page;
            self.username = VimWidget::new(VimType::SingleLine, theme.vi);
            self.password = VimWidget::new(VimType::SingleLine, theme.vi);

            self.password.set_block(VimMode::Normal.highlight_block());

            self.login = Button::new(
                "Login",
                "",
                theme.buttons.accepting,
                ButtonEvent::TriggerLogin,
            );
            self.exit = Button::new("Abort", "<q>", theme.buttons.denying, ButtonEvent::OpenHome);
        }
        self.update_elements();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            let username = self.username.lines()[0].clone().trim().to_owned();
            let password = self.password.lines()[0].clone().trim().to_owned();
            let index = self.index;
            let inputs = self.get_inputs();
            let input_event = inputs[index].handle_event(key).clone()?;
            match key.code {
                KeyCode::Enter => {
                    if let Some(event) = input_event {
                        match event {
                            ActionSubsetWrapper::VimEvent(vim_event) => match vim_event {
                                VimEvent::Normal => self.send(Action::Normal),
                                VimEvent::Insert => self.send(Action::Insert),
                                VimEvent::Enter(_) => self.down(),
                                VimEvent::Up => self.up(),
                                VimEvent::Down => self.down(),
                                VimEvent::StoreConfig(_) => {}
                            },
                            ActionSubsetWrapper::ButtonEvent(ButtonEvent::TriggerLogin) => {
                                let login_action = match (username.is_empty(), password.is_empty())
                                {
                                    (true, true) => {
                                        Action::Error(AppError::MissingPasswordAndUsername)
                                    }
                                    (false, true) => Action::Error(AppError::MissingPassword),
                                    (true, false) => Action::Error(AppError::MissingUsername),
                                    (false, false) => {
                                        self.reset()?;
                                        Action::PerformLogin(username, password)
                                    }
                                };
                                return Ok(Some(login_action));
                            }
                            ActionSubsetWrapper::ButtonEvent(event) => {
                                return Ok(Some(event.into()));
                            }
                            _ => {}
                        }
                    }
                }
                KeyCode::Char('k') => self.up(),
                KeyCode::Char('j') => self.down(),
                _ => {}
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
                Constraint::Max(3 * 4 + 2),
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

            let center = render_nice_bg(center, self.theme, buf);

            let [a, b, c, d] = Layout::vertical([
                Constraint::Max(3),
                Constraint::Max(3),
                Constraint::Max(3),
                Constraint::Max(3),
                //Constraint::Fill(1),
            ])
            .areas(center);

            self.username.render(a, buf);
            self.password.render(b, buf);

            self.login.draw_button(c, buf);
            self.exit.draw_button(d, buf);

            //Line::raw(self.index.to_string())
            //  .centered()
            //  .render(empty, buf);
        }
        Ok(())
    }
}
