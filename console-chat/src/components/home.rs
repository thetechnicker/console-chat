use crate::LockErrorExt;
use crate::action::Result;
use crate::components::{button::*, theme::*};
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};
const STYLE_KEY: crate::app::Mode = crate::app::Mode::Home;

#[derive(Default)]
pub struct Home {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Arc<RwLock<Config>>,
    home_theme: PageColors,
    join: Button,
    random: Button,
    login: Button,
    settings: Button,
    raw_settings: Button,
    exit: Button,
    reset_config: Button,

    index: usize,
}

impl Home {
    pub const MAX_ELEMENTS: usize = 7;

    pub fn new() -> Self {
        Self::default()
    }

    fn up(&mut self) {
        let i = self.index;
        self.index = if self.index == 0 {
            Self::MAX_ELEMENTS - 1
        } else {
            self.index - 1
        };
        self.update_selection(i);
    }

    fn down(&mut self) {
        let i = self.index;
        self.index = (self.index + 1) % Self::MAX_ELEMENTS;
        self.update_selection(i);
    }

    fn update_selection(&mut self, prev: usize) {
        self.get_buttons()[prev].set_state(ButtonState::Normal);
        let i = self.index;
        self.get_buttons()[i].set_state(ButtonState::Selected)
    }

    const fn get_buttons(&mut self) -> [&mut Button; Self::MAX_ELEMENTS] {
        [
            &mut self.join,
            &mut self.random,
            &mut self.login,
            &mut self.settings,
            &mut self.raw_settings,
            &mut self.reset_config,
            &mut self.exit,
        ]
    }
}

impl Component for Home {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        self.active = true;
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
            self.login = Button::new("Login", "", theme.buttons.mid_accept, Action::OpenLogin);
            self.join = Button::new("Join", "", theme.buttons.mid_accept, Action::OpenJoin);
            self.join = Button::new("Join Random", "", theme.buttons.normal, Action::JoinRandom);
            self.settings = Button::new("Settings", "", theme.buttons.normal, Action::OpenSettings);
            self.raw_settings = Button::new(
                "Settings File",
                "",
                theme.buttons.normal,
                Action::OpenRawSettings,
            );
            self.reset_config = Button::new(
                "Reset Config",
                "",
                theme.buttons.normal,
                Action::ResetConfig,
            );
            self.exit = Button::new("Exit", "", theme.buttons.denying, Action::Quit);
            self.home_theme = theme.page;
        }

        self.update_selection(0);
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match key.code {
                KeyCode::Enter => {
                    self.active = false;
                    let i = self.index;
                    let buttons = self.get_buttons();
                    buttons[i].set_state(ButtonState::Active);
                    return Ok(buttons[i].trigger());
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

    fn register_config_handler(&mut self, config: Arc<RwLock<Config>>) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenHome => self.active = true,
            Action::Tick => {
                // add any logic here that should run on every tick
                for button in self.get_buttons() {
                    if button.is_active() {
                        button.set_state(ButtonState::Selected);
                    }
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

            let [_, center, _] = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Percentage(40),
                Constraint::Fill(1),
            ])
            .areas(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(40),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            );

            let text = self.home_theme.foreground;
            let background = self.home_theme.background;
            let highlight = self.home_theme.muted;
            let shadow = self.home_theme.border;

            buf.set_style(center, Style::new().bg(background).fg(text));
            // render top line if there's enough space
            if center.height > 2 {
                buf.set_string(
                    center.x,
                    center.y,
                    "▔".repeat(center.width as usize),
                    Style::new().fg(highlight).bg(background),
                );
            }
            // render bottom line if there's enough space
            if center.height > 1 {
                buf.set_string(
                    center.x,
                    center.y + center.height - 1,
                    "▁".repeat(center.width as usize),
                    Style::new().fg(shadow).bg(background),
                );
            }

            let buttons: [&mut Button; Self::MAX_ELEMENTS] = self.get_buttons();
            let mut constraints = [Constraint::Fill(1)].to_vec();
            constraints.extend_from_slice(&[Constraint::Max(3); Self::MAX_ELEMENTS]);
            constraints.push(Constraint::Fill(1));

            let areas = Layout::vertical(constraints).split(center);
            let _titel = areas[0];

            for (button, area) in buttons
                .into_iter()
                .zip(areas[1..Self::MAX_ELEMENTS + 1].iter())
            {
                button.draw_button(*area, buf);
            }
        }
        Ok(())
    }
}
