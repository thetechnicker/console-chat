use crate::components::{button::*, theme::*, vim::*};
//use color_eyre::Result;
use crate::action::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tracing::trace;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, action::AppError, config::Config};

#[derive(Default, Debug)]
pub struct Join<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    room: TextArea<'a>,
    join: Button,
    cancel: Button,
    vim: Option<Vim>,
    index: usize,
    size: Size,
}

impl Join<'_> {
    pub const MAX_ELEMENTS: usize = 3;

    pub fn new() -> Self {
        Self::default()
    }

    pub fn reset(&mut self) -> Result<()> {
        let size = self.size;
        self.index = 0;
        self.room = TextArea::default();
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
        self.join.set_state(ButtonState::Normal);
        self.cancel.set_state(ButtonState::Normal);
        self.room
            .set_block(Block::default().borders(Borders::ALL).title("Room"));
        match self.index {
            0 => self.room.set_block(VimMode::Normal.highlight_block()),
            1 => {
                self.join.set_state(ButtonState::Selected);
            }
            2 => {
                self.cancel.set_state(ButtonState::Selected);
            }
            _ => {
                self.index %= Self::MAX_ELEMENTS;
            }
        }
    }

    const fn get_buttons(&mut self) -> [&mut Button; 2] {
        [&mut self.join, &mut self.cancel]
    }

    fn send(&mut self, action: Action) -> Result<()> {
        trace!("sending action: {action}");
        let action_tx = self.command_tx.as_ref().ok_or(AppError::MissingActionTX)?;

        Ok(action_tx.send(action)?)
    }
}

impl<'a> Join<'a> {
    fn get_selected_input(&mut self) -> Option<(&mut TextArea<'a>, Vim)> {
        let vim = self.vim.take().unwrap_or_default();
        match self.index {
            0 => Some((&mut self.room, vim)),
            _ => None,
        }
    }
}

impl Component for Join<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        let _themes = self.config.themes.get(&crate::app::Mode::Join);
        self.vim = Some(Vim::default());
        self.room.set_cursor_line_style(Style::default());
        self.room.set_style(Style::default().fg(Color::LightGreen));
        self.room.set_block(VimMode::Normal.highlight_block());

        self.join = Button::new("Join", "", GREEN, Action::TriggerJoin);
        self.cancel = Button::new("Abort", "<q>", RED, Action::OpenHome);
        self.update_elements();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match self.get_selected_input() {
                Some((textinput, this_vim)) => {
                    self.vim = Some(match this_vim.transition(key.into(), textinput) {
                        Transition::Mode(mode) if this_vim.mode != mode => {
                            textinput.set_block(mode.highlight_block());
                            textinput.set_cursor_style(mode.cursor_style());
                            match mode {
                                VimMode::Insert => self.send(Action::Insert)?,
                                VimMode::Normal if this_vim.mode == VimMode::Insert => {
                                    self.send(Action::Normal)?
                                }
                                _ => {}
                            };
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
                        let i = self.index - 1;
                        let room = self.room.lines()[0].clone();
                        let buttons = self.get_buttons();
                        buttons[i].set_state(ButtonState::Active);
                        let button_action = buttons[i].trigger();
                        let result = match button_action {
                            Some(Action::TriggerJoin) => Some(Action::PerformJoin(room)),
                            _ => button_action,
                        };
                        self.reset()?;
                        return Ok(result);
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

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenJoin => self.active = true,
            Action::Tick => {
                // add any logic here that should run on every tick
                if self.join.is_active() {
                    self.join.set_state(ButtonState::Selected);
                }
                if self.cancel.is_active() {
                    self.cancel.set_state(ButtonState::Selected);
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
                Constraint::Max(3 * 3),
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

            Clear.render(center, buf);
            let block = Block::new().bg(Color::DarkGray);
            block.render(center, buf);

            let [a, b, c] =
                Layout::vertical([Constraint::Max(3), Constraint::Max(3), Constraint::Max(3)])
                    .areas(center);

            self.room.render(a, buf);

            self.join.draw_button(b, buf);
            self.cancel.draw_button(c, buf);
        }
        Ok(())
    }
}
