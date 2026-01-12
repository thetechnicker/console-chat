use crate::action::{Result, VimEvent};
use crate::components::theme::Theme;
use crate::components::vim::*;
//use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Home;

#[derive(Default)]
pub struct ConfigFileEditor<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    textinput: VimWidget<'a>,
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
        let lines = serde_json::to_string_pretty(&self.config)?;
        self.textinput = VimWidget::new(VimType::MultiLine, theme.vi).with_text(lines);
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
            if let Some(event) = self.textinput.handle_event(key)? {
                match event {
                    VimEvent::StoreConfig => {
                        if let Some(command_tx) = self.command_tx.as_ref() {
                            let _ = command_tx.send(Action::StoreConfig);
                        }
                    }
                    _ => {}
                }
            }
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
