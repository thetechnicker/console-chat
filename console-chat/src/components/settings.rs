//use color_eyre::Result;
use crate::LockErrorExt;
use crate::action::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default)]
pub struct Settings {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    content: String,
    config: Arc<RwLock<Config>>,
    scroll: usize,
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }
}

impl Component for Settings {
    fn init(&mut self, _: Size) -> Result<()> {
        let conf_arc = self.config.clone();
        let config = conf_arc.read().error()?;
        //let _themes = config.themes.get(&crate::app::Mode::Login);
        self.content = serde_json::to_string_pretty(&*config)?;
        Ok(())
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn hide(&mut self) {
        self.active = false;
    }

    fn register_config_handler(&mut self, config: Arc<RwLock<Config>>) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenSettings => self.active = true,
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
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match key.code {
                KeyCode::Down | KeyCode::Char('j') => {
                    self.scroll = self.scroll.saturating_add(1);
                    Ok(None)
                }
                KeyCode::Up | KeyCode::Char('k') => {
                    self.scroll = self.scroll.saturating_sub(1);
                    Ok(None)
                }
                _ => Ok(None),
            }
        } else {
            Ok(None)
        }
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.active {
            let buf = frame.buffer_mut();
            let center = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Percentage(90),
                Constraint::Fill(1),
            ])
            .split(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(60),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            )[1];
            let paragraph = Paragraph::new(self.content.clone())
                .style(Style::default().bg(Color::Blue))
                .wrap(Wrap { trim: true })
                .scroll((self.scroll as u16, 0));
            paragraph.render(center, buf);
        }
        Ok(())
    }
}
