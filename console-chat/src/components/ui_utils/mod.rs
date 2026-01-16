pub mod button;
pub mod theme;
pub mod vim;
use crate::action::ActionSubsetWrapper;
use crate::error::Result;
use crossterm::event::KeyEvent;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::Style;
use theme::PageColors;

const CONTRAINT: [Constraint; 3] = [Constraint::Max(1), Constraint::Fill(1), Constraint::Max(1)];

pub fn render_nice_bg(area: Rect, theme: PageColors, buf: &mut Buffer) -> Rect {
    let text = theme.foreground;
    let background = theme.background;
    let highlight = theme.muted;
    let shadow = theme.border;

    buf.set_style(area, Style::new().bg(background).fg(text));
    // render top line if there's enough space
    if area.height > 2 {
        buf.set_string(
            area.x,
            area.y,
            "▔".repeat(area.width as usize),
            //format!("▗{}▖", "▄".repeat(area.width as usize)),
            Style::new().fg(highlight).bg(background),
        );
    }
    // render bottom line if there's enough space
    if area.height > 1 {
        buf.set_string(
            area.x,
            area.y + area.height - 1,
            "▁".repeat(area.width as usize),
            //format!("▝{}▘", "▀".repeat(area.width as usize)),
            Style::new().fg(shadow).bg(background),
        );
    }
    //for y in area.y..(area.height + center.y) {
    //    let style = Style::new().fg(background); //.bg(background);
    //    buf.set_string(area.x - 1, y, "▐", style);
    //    buf.set_string(area.width + area.x, y, "▌", style);
    //}

    //let area = Layout::horizontal(CONTRAINT).split(area)[1];
    Layout::vertical(CONTRAINT).split(area)[1]
}

pub trait EventWidget {
    fn handle_event(&mut self, event: KeyEvent) -> Result<Option<ActionSubsetWrapper>>;
}
