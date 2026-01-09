use crate::LockErrorExt;
use crate::action::AppError;
use crate::action::Result;
use crate::components::{button::*, render_nice_bg, theme::*, vim::*};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::Config};

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Login;

#[derive(Default)]
pub struct Login<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Arc<RwLock<Config>>,
    theme: PageColors,
    username: TextArea<'a>,
    password: TextArea<'a>,
    login: Button,
    exit: Button,
    vim: [Option<Vim>; 2],
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
}

impl<'a> Login<'a> {
    fn get_selected_input(&mut self) -> Option<(&mut TextArea<'a>, Vim, usize)> {
        if self.index >= 2 {
            return None;
        }
        let vim = self.vim[self.index].take().unwrap_or_default();
        match self.index {
            0 => Some((&mut self.username, vim, self.index)),
            1 => Some((&mut self.password, vim, self.index)),
            _ => None,
        }
    }
}

impl Component for Login<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        {
            let mut config = self.config.write().error()?;
            let theme = match config.themes.get(&STYLE_KEY) {
                Some(themes) => themes,
                None => match config.themes.get(&crate::app::Mode::Global) {
                    Some(themes) => themes,
                    None => {
                        config
                            .themes
                            .insert(crate::app::Mode::Global, Theme::default());
                        config
                            .themes
                            .get(&crate::app::Mode::Global)
                            .ok_or("This is bad")?
                    }
                },
            };
            self.theme = theme.page;
            self.vim = [Some(Vim::default()), Some(Vim::default())];
            self.username.set_cursor_line_style(Style::default());
            self.username
                .set_style(Style::default().fg(Color::LightGreen));
            self.username.set_block(VimMode::Normal.highlight_block());

            self.password.set_cursor_line_style(Style::default());
            self.password.set_mask_char('\u{2022}');
            self.password
                .set_style(Style::default().fg(Color::LightGreen));
            self.password.set_block(VimMode::Normal.highlight_block());

            self.login = Button::new("Login", "", theme.buttons.accepting, Action::TriggerLogin);
            self.exit = Button::new("Abort", "<q>", theme.buttons.denying, Action::OpenHome);
        }
        self.update_elements();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match self.get_selected_input() {
                Some((textinput, this_vim, i)) => {
                    self.vim[i] = Some(match this_vim.transition(key.into(), textinput) {
                        Transition::Mode(mode) if this_vim.mode != mode => {
                            textinput.set_block(mode.highlight_block());
                            textinput.set_cursor_style(mode.cursor_style(this_vim.style));
                            if let Some(command_tx) = self.command_tx.as_ref() {
                                match mode {
                                    VimMode::Insert => command_tx.send(Action::Insert)?,
                                    VimMode::Normal if this_vim.mode == VimMode::Insert => {
                                        command_tx.send(Action::Normal)?
                                    }
                                    _ => {}
                                };
                            }
                            this_vim.update_mode(mode)
                        }
                        Transition::Nop | Transition::Mode(_) | Transition::Store => this_vim,
                        Transition::Pending(input) => this_vim.with_pending(input),
                        Transition::Up => {
                            self.up();
                            this_vim
                        }
                        Transition::Down => {
                            self.down();
                            this_vim
                        }
                        Transition::Enter(content) => {
                            debug!("{}", content);
                            self.down();
                            this_vim.update_mode(VimMode::Normal)
                        }
                    });
                }
                None => match key.code {
                    KeyCode::Enter => {
                        if self.index == 2 {
                            self.login.set_state(ButtonState::Active);
                            let username = self.username.lines()[0].clone().trim().to_owned();
                            let password = self.password.lines()[0].clone().trim().to_owned();
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
                        } else if self.index == 3 {
                            self.exit.set_state(ButtonState::Active);
                            return Ok(self.exit.trigger());
                        }
                    }
                    KeyCode::Char('k') => self.up(),
                    KeyCode::Char('j') => self.down(),
                    _ => {}
                },
            }
        }
        Ok(None)
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Arc<RwLock<Config>>) -> Result<()> {
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
