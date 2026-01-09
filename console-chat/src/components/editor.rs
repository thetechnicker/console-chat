use crate::LockErrorExt;
use crate::action::{AppError, Result};
use crate::components::theme::Theme;
use crate::components::vim::*;
use std::sync::{Arc, RwLock};
//use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::Config};

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Home;

#[derive(Default)]
pub struct ConfigFileEditor<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Arc<RwLock<Config>>,
    textinput: TextArea<'a>,
    vim: Option<Vim>,
}

impl ConfigFileEditor<'_> {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for ConfigFileEditor<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
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
        let lines = serde_json::to_string_pretty(&*config)?;
        let vim = Vim::new(VimMode::Normal, VimType::MultiLine, theme.vi);
        self.textinput = TextArea::from(lines.split("\n"));
        self.textinput.set_block(vim.mode.block());
        self.textinput
            .set_cursor_style(vim.mode.cursor_style(theme.vi));
        self.vim = Some(vim);
        Ok(())
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn register_config_handler(&mut self, config: Arc<RwLock<Config>>) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            self.vim = if let Some(this_vim) = self.vim.take() {
                Some({
                    let mut pending = false;
                    let mut new_vim = match this_vim.transition(key.into(), &mut self.textinput) {
                        Transition::Mode(mode) if this_vim.mode != mode => {
                            self.textinput.set_block(mode.block());
                            self.textinput
                                .set_cursor_style(mode.cursor_style(this_vim.style));
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
                        Transition::Nop | Transition::Mode(_) => this_vim,
                        Transition::Pending(input) => {
                            pending = true;
                            this_vim.with_pending(input)
                        }
                        Transition::Up => this_vim,
                        Transition::Down => this_vim,
                        Transition::Enter(content) => {
                            debug!("{}", content);
                            this_vim
                        }
                        Transition::Store => {
                            debug!("Storing new config");
                            let mut config = self.config.write().error()?;
                            *config = serde_json::from_str(&self.textinput.lines().join("\n"))?;
                            config.save()?;
                            if let Some(command_tx) = self.command_tx.as_ref() {
                                command_tx.send(Action::ReloadConfig)?;
                            } else {
                                return Err(AppError::MissingActionTX);
                            }
                            this_vim
                        }
                    };
                    if !pending {
                        new_vim.reset_pending();
                    }
                    self.textinput
                        .set_block(new_vim.mode.block().title_bottom(new_vim.input_seq()));
                    new_vim
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
            let [_, center, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Percentage(60),
                Constraint::Fill(1),
            ])
            .areas(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(60),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            );
            block.render(center, buf);

            self.textinput.render(center, buf);
        }

        Ok(())
    }
}
