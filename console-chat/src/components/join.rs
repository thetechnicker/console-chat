use super::Component;

use crate::action::Result;
use crate::components::{button::*, render_nice_bg, theme::*, vim::*};
use crate::{action::Action, action::ButtonEvent, action::VimEvent, config::Config};

use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Join;

#[derive(Default, Debug)]
pub struct Join<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    theme: Theme,
    static_room: bool,
    room: VimWidget<'a>,
    join: Button,
    cancel: Button,
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
        self.room = VimWidget::new(VimType::SingleLine, self.theme.vi);
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

    fn send(&mut self, action: Action) {
        if let Some(action_tx) = self.command_tx.as_ref() {
            let _ = action_tx.send(action);
        }
    }
}

impl Component for Join<'_> {
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

            self.theme = *theme;
            self.room = VimWidget::new(VimType::SingleLine, self.theme.vi);
            self.join = Button::new(
                "Join",
                "",
                theme.buttons.accepting,
                ButtonEvent::TriggerJoin,
            );
            self.cancel = Button::new("Abort", "<q>", theme.buttons.denying, ButtonEvent::OpenHome);
        }
        self.update_elements();
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            if self.index == 0 {
                if let Some(event) = self.room.handle_event(key)? {
                    match event {
                        VimEvent::Normal => self.send(Action::Normal),
                        VimEvent::Insert => self.send(Action::Insert),
                        VimEvent::Enter(_) => self.down(),
                        VimEvent::Up => self.up(),
                        VimEvent::Down => self.down(),
                        VimEvent::StoreConfig(_) => {}
                    }
                }
            } else {
                match key.code {
                    KeyCode::Enter => {
                        let i = self.index - 1;
                        let room = self.room.lines()[0].clone();
                        let buttons = self.get_buttons();
                        buttons[i].set_state(ButtonState::Active);
                        let button_action = buttons[i].trigger();
                        let result = match button_action {
                            Some(ButtonEvent::TriggerJoin) => {
                                Some(Action::PerformJoin(room, self.static_room))
                            }
                            _ => button_action.map(|a| a.into()),
                        };
                        self.reset()?;
                        return Ok(result);
                    }
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
            Action::OpenJoin(static_room) => {
                self.active = true;
                self.static_room = static_room;
            }
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
                Constraint::Max(3 * 3 + 2),
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
