use std::time::Instant;
use tokio::sync::mpsc::UnboundedSender;

use color_eyre::Result;
use ratatui::{prelude::*, widgets::*};

use super::Component;

use crate::action::Action;

const ERROR_TIMEOUT: f64 = 5.0;

#[derive(Debug, Clone)]
pub struct ErrorDisplay {
    last_error: Instant,
    errors: Vec<String>,
    current_error: Option<String>,
    command_tx: Option<UnboundedSender<Action>>,
}

impl Default for ErrorDisplay {
    fn default() -> Self {
        Self::new()
    }
}

impl ErrorDisplay {
    pub fn new() -> Self {
        Self {
            last_error: Instant::now(),
            errors: Vec::new(),
            current_error: None,
            command_tx: None,
        }
    }

    fn app_tick(&mut self) -> Result<()> {
        let now = Instant::now();
        let elapsed = (now - self.last_error).as_secs_f64();
        if elapsed >= ERROR_TIMEOUT || self.current_error.is_none() {
            self.current_error = self.errors.pop();
            if self.current_error.is_none()
                && let Some(command_tx) = self.command_tx.as_mut() {
                    command_tx.send(Action::Normal)?;
                }
        }
        Ok(())
    }
}

impl Component for ErrorDisplay {
    fn hide(&mut self) {}
    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::Error(msg) => self.errors.push(msg),
            Action::Tick => self.app_tick()?,
            _ => {}
        };
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if let Some(error) = self.current_error.as_ref() {
            let center = Layout::vertical([
                Constraint::Fill(1),
                Constraint::Percentage(50),
                Constraint::Fill(1),
            ])
            .split(
                Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(50),
                    Constraint::Fill(1),
                ])
                .split(area)[1],
            )[1];

            let display = Paragraph::new(error.clone());
            frame.render_widget(display, center);
        }
        Ok(())
    }
}
