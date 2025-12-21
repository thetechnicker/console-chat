use crate::components::vim::*;
use color_eyre::Result;
use crossterm::event::KeyEvent;
use openapi::models::{AppearancePublic, MessagePublic, UserPublic};
use ratatui::{prelude::*, widgets::*};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::TextArea;

use super::Component;
use crate::{action::Action, config::Config};

struct MessageComponent {
    content: MessagePublic,
    selected: bool,
}
impl MessageComponent {
    fn new(content: MessagePublic) -> Self {
        Self {
            content,
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

impl Widget for &MessageComponent {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let user = self
            .content
            .sender
            .clone()
            .unwrap_or(Box::new(UserPublic::new(AppearancePublic::new(
                "#c0ffee".to_owned(),
            ))));
        let name = user.username.unwrap_or("System".to_owned());
        let color = user.appearance.color.parse().unwrap_or(Color::Gray);
        let message = match self.content.content {
            Some(_) => "",
            None => "",
        };
        Paragraph::new(message)
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .fg(color)
                    .title(name), //       .title_alignment(alignment),
            )
            //.alignment(alignment)
            .render(area, buf);
    }
}
impl From<MessagePublic> for MessageComponent {
    fn from(msg: MessagePublic) -> Self {
        Self::new(msg)
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
            && let Some(m) = self.msgs.get_mut(prev - 1)
        {
            m.unselect()
        }
        if self.index > 0
            && let Some(m) = self.msgs.get_mut(self.index - 1)
        {
            m.select()
        }
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
        if self.active && self.index == 0 {
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
                        self.command_tx
                            .as_mut()
                            .unwrap()
                            .send(Action::SendMessage(content.to_owned()))?;
                        self.textinput = TextArea::default();
                        self.textinput.set_block(this_vim.mode.highlight_block());
                        self.textinput
                            .set_cursor_style(this_vim.mode.cursor_style());
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
            //Action::ReceivedMessage(msg) => self.msgs.push(msg.into()),
            Action::Leave => self.msgs.clear(),
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

            let [mut chat, input] = Layout::vertical([Constraint::Fill(1), Constraint::Max(3)])
                .areas(
                    Layout::horizontal([
                        Constraint::Fill(1),
                        Constraint::Percentage(60),
                        Constraint::Fill(1),
                    ])
                    .split(area)[1],
                );
            for msg in self.msgs.iter().rev() {
                let [new_chat, msg_area] =
                    Layout::vertical([Constraint::Fill(1), Constraint::Max(3)]).areas(chat);
                msg.render(msg_area, buf);
                chat = new_chat;
            }

            self.textinput.render(input, buf);
        }

        Ok(())
    }
}
