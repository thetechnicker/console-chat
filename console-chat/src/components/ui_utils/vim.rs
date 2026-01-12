use crate::action::VimEvent;
use crate::components::theme::ViModePalettes;

use ratatui::style::{Color, Style};
use ratatui::widgets::{Block, Borders};
use tui_textarea::{CursorMove, Input, Key, Scrolling, TextArea};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimMode {
    Normal,
    Insert,
    Visual,
    Operator(char),
}

impl VimMode {
    pub fn block<'a>(&self) -> Block<'a> {
        let help = match self {
            Self::Normal => "type i to enter insert mode",
            Self::Insert => "type Esc to back to normal mode",
            Self::Visual => "type y to yank, type d to delete, type Esc to back to normal mode",
            Self::Operator(_) => "move cursor to apply operator",
        };
        let title = format!("{} MODE ({})", self, help);
        Block::default()
            .borders(Borders::ALL)
            .title(title)
            .title_bottom(self.input_seq())
    }

    pub fn highlight_block<'a>(&self) -> Block<'a> {
        self.block()
            .border_style(Style::default().fg(Color::Yellow))
    }

    pub fn cursor_style(&self, style: ViModePalettes) -> Style {
        style.cursor_style_for_mode(self)
    }

    fn input_seq(&self) -> String {
        if let VimMode::Operator(c) = self {
            let pending_str = format!(
                "{c}",
                //if self.pending.ctrl { "ctrl-" } else { "" },
                //if self.pending.alt { "alt-" } else { "" },
                //if self.pending.shift { "shift-" } else { "" },
            );
            return pending_str;
        }
        String::new()
    }
}

impl std::fmt::Display for VimMode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
            Self::Normal => write!(f, "NORMAL"),
            Self::Insert => write!(f, "INSERT"),
            Self::Visual => write!(f, "VISUAL"),
            Self::Operator(c) => write!(f, "OPERATOR({})", c),
        }
    }
}

// How the Vim emulation state transitions
pub enum Transition {
    Nop,
    Up,
    Down,
    Store,
    Mode(VimMode),
    Pending(Input),
    Enter(String),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VimType {
    MultiLine,
    SingleLine,
}

// State of Vim emulation
#[derive(Debug, Clone)]
pub struct Vim {
    pub vim_type: VimType,
    pub mode: VimMode,
    pub pending: Input, // Pending input to handle a sequence with two keys like gg
    pub style: ViModePalettes,
}
impl PartialEq for Vim {
    fn eq(&self, other: &Self) -> bool {
        self.vim_type == other.vim_type && self.mode == other.mode && self.pending == other.pending
    }
}

impl Default for Vim {
    fn default() -> Self {
        Self::new(
            VimMode::Normal,
            VimType::SingleLine,
            ViModePalettes::default(),
        )
    }
}

impl Vim {
    pub fn new(mode: VimMode, vim_type: VimType, style: ViModePalettes) -> Self {
        Self {
            vim_type,
            mode,
            pending: Input::default(),
            style,
        }
    }
    pub fn copy(&self) -> Self {
        Self {
            vim_type: self.vim_type,
            mode: self.mode,
            pending: Input::default(),
            style: self.style,
        }
    }

    pub fn input_seq(&self) -> String {
        // if let VimMode::Operator(c) = self.mode {
        //     let pending_str = format!(
        //         "{}{}{}{c}",
        //         if self.pending.ctrl { "ctrl-" } else { "" },
        //         if self.pending.alt { "alt-" } else { "" },
        //         if self.pending.shift { "shift-" } else { "" },
        //     );
        //     return pending_str;
        // }
        // String::new()
        format!(
            "{}{}{}{:?}",
            if self.pending.ctrl { "ctrl-" } else { "" },
            if self.pending.alt { "alt-" } else { "" },
            if self.pending.shift { "shift-" } else { "" },
            self.pending.key,
        )
    }

    pub fn update_mode(mut self, mode: VimMode) -> Self {
        self.mode = mode;
        self
    }

    // pub fn reset_pending(&mut self) {
    //     self.pending = Input::default();
    // }

    pub fn with_pending(self, pending: Input) -> Self {
        Self {
            vim_type: self.vim_type,
            mode: self.mode,
            pending,
            style: self.style,
        }
    }

    pub fn transition(&self, input: Input, textarea: &mut TextArea<'_>) -> Transition {
        if input.key == Key::Null {
            return Transition::Nop;
        }

        match self.mode {
            VimMode::Normal | VimMode::Visual | VimMode::Operator(_) => {
                match input {
                    Input {
                        key: Key::Char('w'),
                        ..
                    } if self.pending.key == Key::Char(':')
                        && self.vim_type == VimType::MultiLine =>
                    {
                        return Transition::Store;
                    }
                    Input {
                        key: Key::Char('h'),
                        ..
                    } => textarea.move_cursor(CursorMove::Back),
                    Input {
                        key: Key::Char('j'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::Down);
                        return Transition::Down;
                    }

                    Input {
                        key: Key::Char('k'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::Up);
                        return Transition::Up;
                    }
                    Input {
                        key: Key::Char('l'),
                        ..
                    } => textarea.move_cursor(CursorMove::Forward),
                    Input {
                        key: Key::Char('w'),
                        ..
                    } => textarea.move_cursor(CursorMove::WordForward),
                    Input {
                        key: Key::Char('e'),
                        ctrl: false,
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::WordEnd);
                        if matches!(self.mode, VimMode::Operator(_)) {
                            textarea.move_cursor(CursorMove::Forward); // Include the text under the cursor
                        }
                    }
                    Input {
                        key: Key::Char('b'),
                        ctrl: false,
                        ..
                    } => textarea.move_cursor(CursorMove::WordBack),
                    Input {
                        key: Key::Char('^'),
                        ..
                    } => textarea.move_cursor(CursorMove::Head),
                    Input {
                        key: Key::Char('$'),
                        ..
                    } => textarea.move_cursor(CursorMove::End),
                    Input {
                        key: Key::Char('D'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('C'),
                        ..
                    } => {
                        textarea.delete_line_by_end();
                        textarea.cancel_selection();
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('p'),
                        ..
                    } => {
                        textarea.paste();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('u'),
                        ctrl: false,
                        ..
                    } => {
                        textarea.undo();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('r'),
                        ctrl: true,
                        ..
                    } => {
                        textarea.redo();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('x'),
                        ..
                    } => {
                        textarea.delete_next_char();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('i'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('a'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::Forward);
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('A'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('o'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::End);
                        textarea.insert_newline();
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('O'),
                        ..
                    } => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.insert_newline();
                        textarea.move_cursor(CursorMove::Up);
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('I'),
                        ..
                    } => {
                        textarea.cancel_selection();
                        textarea.move_cursor(CursorMove::Head);
                        return Transition::Mode(VimMode::Insert);
                    }
                    Input {
                        key: Key::Char('e'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((1, 0)),
                    Input {
                        key: Key::Char('y'),
                        ctrl: true,
                        ..
                    } => textarea.scroll((-1, 0)),
                    Input {
                        key: Key::Char('d'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageDown),
                    Input {
                        key: Key::Char('u'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::HalfPageUp),
                    Input {
                        key: Key::Char('f'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageDown),
                    Input {
                        key: Key::Char('b'),
                        ctrl: true,
                        ..
                    } => textarea.scroll(Scrolling::PageUp),
                    Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(VimMode::Visual);
                    }
                    Input {
                        key: Key::Char('V'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Normal => {
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        textarea.move_cursor(CursorMove::End);
                        return Transition::Mode(VimMode::Visual);
                    }
                    Input { key: Key::Esc, .. }
                    | Input {
                        key: Key::Char('v'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Visual => {
                        textarea.cancel_selection();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('g'),
                        ctrl: false,
                        ..
                    } if matches!(
                        self.pending,
                        Input {
                            key: Key::Char('g'),
                            ctrl: false,
                            ..
                        }
                    ) =>
                    {
                        textarea.move_cursor(CursorMove::Top)
                    }
                    Input {
                        key: Key::Char('G'),
                        ctrl: false,
                        ..
                    } => textarea.move_cursor(CursorMove::Bottom),
                    Input {
                        key: Key::Char(c),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Operator(c) => {
                        // Handle yy, dd, cc. (This is not strictly the same behavior as Vim)
                        textarea.move_cursor(CursorMove::Head);
                        textarea.start_selection();
                        let cursor = textarea.cursor();
                        textarea.move_cursor(CursorMove::Down);
                        if cursor == textarea.cursor() {
                            textarea.move_cursor(CursorMove::End); // At the last line, move to end of the line instead
                        }
                    }
                    Input {
                        key: Key::Char(op @ ('y' | 'd' | 'c')),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Normal => {
                        textarea.start_selection();
                        return Transition::Mode(VimMode::Operator(op));
                    }
                    Input {
                        key: Key::Char('y'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.copy();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('d'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.cut();
                        return Transition::Mode(VimMode::Normal);
                    }
                    Input {
                        key: Key::Char('c'),
                        ctrl: false,
                        ..
                    } if self.mode == VimMode::Visual => {
                        textarea.move_cursor(CursorMove::Forward); // Vim's text selection is inclusive
                        textarea.cut();
                        return Transition::Mode(VimMode::Insert);
                    }
                    input => return Transition::Pending(input),
                }

                // Handle the pending operator
                match self.mode {
                    VimMode::Operator('y') => {
                        textarea.copy();
                        Transition::Mode(VimMode::Normal)
                    }
                    VimMode::Operator('d') => {
                        textarea.cut();
                        Transition::Mode(VimMode::Normal)
                    }
                    VimMode::Operator('c') => {
                        textarea.cut();
                        Transition::Mode(VimMode::Insert)
                    }
                    _ => Transition::Nop,
                }
            }
            VimMode::Insert => match input {
                Input { key: Key::Esc, .. }
                | Input {
                    key: Key::Char('c'),
                    ctrl: true,
                    ..
                } => Transition::Mode(VimMode::Normal),
                input => {
                    if self.vim_type == VimType::SingleLine && input.key == Key::Enter {
                        let content = textarea.lines().join("\n").trim().to_string();
                        if content.is_empty() {
                            Transition::Mode(VimMode::Insert)
                        } else {
                            Transition::Enter(content)
                        }
                    } else {
                        textarea.input(input); // Use default key mappings in insert mode
                        Transition::Mode(VimMode::Insert)
                    }
                }
            },
        }
    }
}

use crate::error::Result;
use crossterm::event::KeyEvent;
//use tracing::debug;

use std::ops::{Deref, DerefMut};
#[derive(Debug, Default)]
pub struct VimWidget<'a> {
    vim: Vim,
    textinput: TextArea<'a>,
}

impl<'a> Deref for VimWidget<'a> {
    type Target = TextArea<'a>;
    fn deref(&self) -> &Self::Target {
        &self.textinput
    }
}
impl<'a> DerefMut for VimWidget<'a> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.textinput
    }
}

impl<'a> VimWidget<'a> {
    pub fn new(vim_type: VimType, style: ViModePalettes) -> Self {
        let vim = Vim::new(VimMode::Normal, vim_type, style);
        let mut textinput = TextArea::default();
        textinput.set_block(vim.mode.block());
        textinput.set_cursor_style(vim.mode.cursor_style(vim.style));
        Self { vim, textinput }
    }

    pub fn with_text(mut self, text: impl Into<String>) -> Self {
        self.textinput = TextArea::from(text.into().split("\n"));
        self.textinput.set_block(self.vim.mode.block());
        self.textinput
            .set_cursor_style(self.vim.mode.cursor_style(self.vim.style));
        self
    }

    pub fn handle_event(&mut self, key: KeyEvent) -> Result<Option<VimEvent>> {
        let mut to_return = None;
        let new_vim = self.vim.copy();
        self.vim = match self.vim.transition(key.into(), &mut self.textinput) {
            Transition::Mode(mode) if self.vim.mode != mode => {
                match mode {
                    VimMode::Insert => to_return = Some(VimEvent::Insert),
                    VimMode::Normal if self.vim.mode != mode => to_return = Some(VimEvent::Normal),
                    _ => {}
                }
                new_vim.update_mode(mode)
            }
            Transition::Nop | Transition::Mode(_) => new_vim,
            Transition::Pending(input) => new_vim.with_pending(input),
            Transition::Up => {
                to_return = Some(VimEvent::Up);
                new_vim
            }
            Transition::Down => {
                to_return = Some(VimEvent::Down);
                new_vim
            }
            Transition::Enter(content) => {
                to_return = Some(VimEvent::Enter(content));
                new_vim
            }
            Transition::Store => {
                to_return = Some(VimEvent::StoreConfig);
                new_vim
            }
        };
        self.textinput.set_block(
            self.vim
                .mode
                .highlight_block()
                .title_bottom(self.vim.input_seq()),
        );
        self.textinput
            .set_cursor_style(self.vim.mode.cursor_style(self.vim.style));
        Ok(to_return)
    }
}

use ratatui::prelude::*;

impl Widget for &VimWidget<'_> {
    fn render(self, area: Rect, buf: &mut Buffer) {
        self.textinput.render(area, buf);
    }
}

use super::EventWidget;
use crate::action::ActionSubsetWrapper;

impl<'a> EventWidget for VimWidget<'a> {
    fn handle_event(&mut self, key: KeyEvent) -> Result<Option<ActionSubsetWrapper>> {
        VimWidget::handle_event(self, key).map(|o| o.map(|a| a.into()))
    }
}
