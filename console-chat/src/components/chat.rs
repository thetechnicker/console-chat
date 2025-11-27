#![allow(dead_code)]
use crate::components::vim::*;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::Config};

struct MessageComponent {
    content: String,
    selected: bool,
}
impl MessageComponent {
    fn new(content: impl ToString) -> Self {
        Self {
            content: content.to_string(),
            selected: false,
        }
    }
    fn select(&mut self) {
        self.selected = true;
    }
    fn unselect(&mut self) {
        self.selected = false;
    }
}

#[derive(Default)]
pub struct Chat<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    textinput: TextArea<'a>,
    vim: Option<Vim>,
    index: usize,
    msgs: Vec<MessageComponent>,
}

impl Chat<'_> {
    pub fn new() -> Self {
        Self::default()
    }

    fn up(&mut self) {
        let i = self.index;
        self.index = if self.index == 0 {
            self.msgs.len() - 1
        } else {
            self.index - 1
        };
        self.update_selection(i);
    }

    fn down(&mut self) {
        let i = self.index;
        self.index = (self.index + 1) % self.msgs.len();
        self.update_selection(i);
    }

    fn update_selection(&mut self, prev: usize) {
        if prev > 0
            && let Some(m) = self.msgs.get_mut(prev - 1) { m.unselect() }
        if self.index > 0
            && let Some(m) = self.msgs.get_mut(self.index - 1) { m.select() }
        if self.index != 0 {
            self.textinput
                .set_block(Block::default().borders(Borders::ALL).title("Chat"));
        }
    }
}

impl Component for Chat<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        let _themes = self.config.themes.get(&crate::app::Mode::Chat);
        let vim = Vim::new(VimMode::Normal, VimType::SingleLine);
        self.textinput.set_block(vim.mode.highlight_block());
        self.textinput.set_cursor_style(vim.mode.cursor_style());
        self.vim = Some(vim);
        Ok(())
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active
            && self.index == 0 {
                self.vim = if let Some(this_vim) = self.vim.take() {
                    Some(match this_vim.transition(key.into(), &mut self.textinput) {
                        Transition::Mode(mode) if this_vim.mode != mode => {
                            self.textinput.set_block(mode.highlight_block());
                            self.textinput.set_cursor_style(mode.cursor_style());
                            match mode {
                                VimMode::Insert => {
                                    self.command_tx.as_mut().unwrap().send(Action::Insert)?
                                }
                                VimMode::Normal if this_vim.mode == VimMode::Insert => {
                                    self.command_tx.as_mut().unwrap().send(Action::Normal)?
                                }
                                _ => {}
                            };
                            this_vim.update_mode(mode)
                        }
                        Transition::Nop | Transition::Mode(_) => this_vim,
                        Transition::Pending(input) => this_vim.with_pending(input),
                        Transition::Up => this_vim,
                        Transition::Down => this_vim,
                        Transition::Enter(content) => {
                            debug!("{}", content);
                            this_vim
                        }
                        Transition::Store => {
                            debug!("Storing new config");
                            self.config =
                                serde_json::from_str(&self.textinput.lines().join("\n"))?;
                            self.config.save()?;
                            self.command_tx
                                .as_mut()
                                .unwrap()
                                .send(Action::ReloadConfig)?;
                            this_vim
                        }
                    })
                } else {
                    Some(Vim::default())
                }
            }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenChat => self.active = true,
            Action::Tick => {
                // add any logic here that should run on every tick
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

            let [_chat, input] = Layout::vertical([Constraint::Fill(1), Constraint::Max(3)]).areas(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(60),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            );

            self.textinput.render(input, buf);
        }

        Ok(())
    }
}
