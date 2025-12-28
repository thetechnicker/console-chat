use super::Component;
use crate::LockErrorExt;
use crate::action::Result;
use crate::components::ui_utils::theme::Theme;
use crate::components::vim::*;
use crate::network::{Message, USERNAME};
use crate::{action::Action, config::Config};
use chrono::Local;
use crossterm::event::{KeyCode, KeyEvent};
use openapi::models::{AppearancePublic, UserPublic};
use ratatui::{prelude::*, widgets::*};
use std::collections::HashMap;
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Chat;

struct MessageComponent {
    content: Message,
    alignment: Alignment,
    selected: bool,
}
impl MessageComponent {
    fn new(content: Message) -> Self {
        let user = content
            .user
            .clone()
            .unwrap_or(UserPublic::new(AppearancePublic::new("#c0ffee".to_owned())));
        let alignment = if let Ok(me) = USERNAME.read() {
            if *me == user.username {
                Alignment::Right
            } else {
                Alignment::Left
            }
        } else {
            Alignment::Left
        };
        Self {
            content,
            alignment,
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

impl StatefulWidget for &MessageComponent {
    type State = Theme;
    fn render(self, area: Rect, buf: &mut Buffer, active: &mut Theme) {
        let user = self
            .content
            .user
            .clone()
            .unwrap_or(UserPublic::new(AppearancePublic::new("".to_owned())));
        let name = user.username.clone().unwrap_or("System".to_owned());
        let color = user.appearance.color.parse().unwrap_or(Color::Gray);
        let message = self.content.content.clone();

        let mut block = Block::bordered()
            .border_type(BorderType::Rounded)
            .fg(color)
            .title(name);

        if self.selected {
            let theme: Style = active.to_owned().into();
            block = block.style(theme.bg(color));
        }

        if let Some(send_time) = self.content.send_at {
            let time_str = send_time
                .with_timezone(&Local)
                .format("%H:%M:%S %Y-%d-%m") // TODO: SETTINGS
                .to_string();
            block = block.title_bottom(time_str);
        }

        block = block.title_alignment(self.alignment);

        let para: Paragraph = Paragraph::new(message)
            .wrap(Wrap { trim: false })
            .block(block)
            .alignment(self.alignment);

        para.render(area, buf);
    }
}

impl From<Message> for MessageComponent {
    fn from(msg: Message) -> Self {
        Self::new(msg)
    }
}

#[derive(Default)]
pub struct Chat<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Arc<RwLock<Config>>,
    selected_theme: Theme,
    textinput: TextArea<'a>,
    vim: Option<Vim>,
    index: usize, // index of currently selected message in msgs (0 means none / input)
    msgs: Vec<MessageComponent>,
}

impl Chat<'_> {
    pub fn new() -> Self {
        Self::default()
    }

    fn safe_len(&self) -> usize {
        self.msgs.len().max(1) // ensure math using len doesn't underflow; index 0 is input
    }

    fn up(&mut self) {
        // navigate messages; index==0 is input; messages are 1..=msgs.len()
        let max = self.safe_len();
        self.index = if self.index == 0 {
            // move to last message if any
            if self.msgs.is_empty() {
                0
            } else {
                self.msgs.len()
            }
        } else {
            (self.index - 1) % max
        };
        // update selection
        self.update_selection();
    }

    fn down(&mut self) {
        let max = self.safe_len();
        self.index = (self.index + 1) % max;
        self.update_selection();
    }

    fn update_selection(&mut self) {
        // unselect all, then select the message at self.index (if > 0)
        for m in &mut self.msgs {
            m.unselect();
        }
        if self.index > 0 {
            if let Some(m) = self.msgs.get_mut(self.index - 1) {
                m.select();
            }
            // when a message is selected, style textarea as normal (no special)
            self.textinput
                .set_block(Block::default().borders(Borders::ALL).title("Chat"));
        } else {
            // input selected -> ensure textarea block reflects vim mode if present
            if let Some(v) = &self.vim {
                self.textinput.set_block(v.mode.highlight_block());
            } else {
                self.textinput
                    .set_block(Block::default().borders(Borders::ALL).title("Chat"));
            }
        }
    }
}

impl Component for Chat<'_> {
    fn hide(&mut self) {
        self.active = false;
    }
    fn init(&mut self, _: Size) -> Result<()> {
        let mut config = self.config.write().error()?;
        let themes: &mut HashMap<String, Theme> = match config.themes.get_mut(&STYLE_KEY) {
            Some(themes) => themes,
            None => {
                config.themes.insert(STYLE_KEY, HashMap::new());
                config.themes.get_mut(&STYLE_KEY).ok_or("This is bad")?
            }
        };
        let vim = Vim::new(VimMode::Normal, VimType::SingleLine);
        self.textinput.set_block(vim.mode.highlight_block());
        self.textinput.set_cursor_style(vim.mode.cursor_style());
        self.vim = Some(vim);
        let selected_theme_option = themes.get("selected").cloned();
        let selected_theme = selected_theme_option.unwrap_or(Theme {
            text: Color::LightMagenta,
            background: Color::DarkGray,
            highlight: Color::Gray,
            shadow: Color::Black,
        });
        if selected_theme_option.is_none() {
            themes.insert("selected".to_owned(), selected_theme);
        }
        self.selected_theme = selected_theme;
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
        if let Some(command_tx) = self.command_tx.as_ref()
            && self.active
        {
            if self.index == 0 {
                self.vim = if let Some(this_vim) = self.vim.take() {
                    Some(match this_vim.transition(key.into(), &mut self.textinput) {
                        Transition::Mode(mode) if this_vim.mode != mode => {
                            self.textinput.set_block(mode.highlight_block());
                            self.textinput.set_cursor_style(mode.cursor_style());
                            match mode {
                                VimMode::Insert => command_tx.send(Action::Insert)?,
                                VimMode::Normal if this_vim.mode == VimMode::Insert => {
                                    command_tx.send(Action::Normal)?
                                }
                                _ => {}
                            };
                            this_vim.update_mode(mode)
                        }
                        Transition::Store | Transition::Nop | Transition::Mode(_) => this_vim,
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
                            command_tx.send(Action::SendMessage(content.to_owned()))?;
                            self.textinput = TextArea::default();
                            self.textinput.set_block(this_vim.mode.highlight_block());
                            self.textinput
                                .set_cursor_style(this_vim.mode.cursor_style());
                            this_vim
                        }
                    })
                } else {
                    Some(Vim::default())
                };
            } else {
                match key.code {
                    KeyCode::Char('k') => self.up(),
                    KeyCode::Char('j') => self.down(),
                    _ => {}
                }
            }
        }
        Ok(None)
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenChat => {
                self.active = true;
                // ensure selection reset to input on open
                self.index = 0;
                self.update_selection();
            }
            Action::ReceivedMessage(msg) => {
                self.msgs.push(msg.into());
                // keep selection on input, but if a message was selected keep it
                if self.index > self.msgs.len() {
                    self.index = self.msgs.len();
                }
                self.update_selection();
            }
            Action::Leave => {
                self.msgs.clear();
                self.index = 0;
            }
            Action::Tick => {}
            Action::Render => {}
            _ => {}
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.active {
            let buf = frame.buffer_mut();
            let block = Block::new().bg(Color::Blue); // TODO: SETTINGS
            block.render(area, buf);

            let [mut chat_area, input_area] =
                Layout::vertical([Constraint::Fill(1), Constraint::Max(3)]).areas(
                    Layout::horizontal([
                        Constraint::Fill(1),
                        Constraint::Percentage(60), // TODO: SETTINGS
                        Constraint::Fill(1),
                    ])
                    .split(area)[1],
                );

            // render messages from newest at bottom; compute rows conservatively
            for msg in self.msgs.iter().rev() {
                // approximate rows needed: message length divided by width, plus padding
                let a = msg.content.content.len() as u16;
                let b = chat_area.width.max(1);
                let rows = (a + b - 1) / b;
                let max_rows = rows.saturating_add(2);
                let [new_chat, msg_area] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Max(max_rows)])
                        .areas(chat_area);
                msg.render(msg_area, buf, &mut self.selected_theme);
                chat_area = new_chat;
            }

            self.textinput.render(input_area, buf);
        }
        Ok(())
    }
}
