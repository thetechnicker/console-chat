use crate::components::vim::*;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Editor<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    textinput: TextArea<'a>,
    vim: Option<Vim>,
}

impl Editor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Editor<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        let _themes = self.config.themes.get(&crate::app::Mode::RawSettings);
        let lines = serde_json::to_string_pretty(&self.config)?;
        let vim = Vim::new(VimMode::Normal, VimType::MultiLine);
        self.textinput = TextArea::from(lines.split("\n"));
        self.textinput.set_block(vim.mode.block());
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
        if self.active {
            self.vim = if let Some(this_vim) = self.vim.take() {
                Some(match this_vim.transition(key.into(), &mut self.textinput) {
                    Transition::Mode(mode) if this_vim.mode != mode => {
                        self.textinput.set_block(mode.block());
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
                        self.config = serde_json::from_str(&self.textinput.lines().join("\n"))?;
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
            };
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenRawSettings => self.active = true,
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

            self.textinput.render(area, buf);
        }

        Ok(())
    }
}
