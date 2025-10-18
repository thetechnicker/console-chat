use super::Widget;
use crate::event::WidgetEvent;
use ratatui::crossterm::event::{Event, KeyCode};
use tui_input::Input;
use tui_input::backend::crossterm::EventHandler;

#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    #[default]
    Normal,
    Editing,
}

#[derive(Debug)]
pub struct InputWidget {
    pub input_mode: InputMode,
    pub input: Input,
}

impl Default for InputWidget {
    fn default() -> Self {
        Self {
            input_mode: InputMode::default(),
            input: Input::default(),
        }
    }
}

impl InputWidget {
    fn start_editing(&mut self) {
        self.input_mode = InputMode::Editing
    }

    fn stop_editing(&mut self) {
        self.input_mode = InputMode::Normal
    }
}

impl Widget for InputWidget {
    fn handle_event(&mut self, event: WidgetEvent) {
        match event {
            WidgetEvent::NoFocus => self.stop_editing(),
            WidgetEvent::Focus => self.start_editing(),
            WidgetEvent::KeyEvent(key) => match self.input_mode {
                InputMode::Normal => match key.code {
                    //KeyCode::Char('e') => self.start_editing(),
                    _ => {}
                },
                InputMode::Editing => match key.code {
                    //KeyCode::Esc => self.stop_editing(),
                    _ => {
                        self.input.handle_event(&Event::Key(key));
                    }
                },
            },
            _ => {}
        }
    }
}
