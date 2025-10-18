use ratatui::{
    buffer::Buffer,
    layout::{Alignment, Constraint, Layout, Rect},
    style::{Color, Style},
    widgets::{Block, BorderType, Paragraph, Widget},
};

const DEFAULT_BORDER: BorderType = BorderType::Double;

use crate::app::App;
use crate::widgets::InputMode;
//use crate::widgets as appWidgets;

impl Widget for &App {
    /// Renders the user interface widgets.
    ///
    // This is where you add new widgets.
    // See the following resources:
    // - https://docs.rs/ratatui/latest/ratatui/widgets/index.html
    // - https://github.com/ratatui/ratatui/tree/master/examples
    fn render(self, area: Rect, buf: &mut Buffer) {
        let outer_block = Block::bordered()
            .border_type(BorderType::Double)
            .title("TUI-CHAT")
            .title_alignment(Alignment::Center);
        let inner = outer_block.inner(area);

        outer_block.render(area, buf);
        let [left, main, right] = Layout::horizontal([
            Constraint::Percentage(20),
            Constraint::Percentage(60),
            Constraint::Percentage(20),
        ])
        .areas(inner);

        // LEFT

        let left_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _left_inner = left_block.inner(left);
        left_block.render(left, buf);

        // RIGHT

        let right_block = Block::bordered().border_type(DEFAULT_BORDER);
        let _right_inner = right_block.inner(right);
        right_block.render(right, buf);

        // MAIN
        let chat_block = Block::bordered().border_type(DEFAULT_BORDER);
        let chat_inner = chat_block.inner(main);

        chat_block.render(main, buf);
        let [chat, input] =
            Layout::vertical([Constraint::Min(10), Constraint::Max(3)]).areas(chat_inner);

        let x = Paragraph::new(format!(
            "{:?}\n{:?}\n{}",
            self.last_event, self.input, self.tab_index
        ));
        x.render(chat, buf);

        // Input
        let style = match self.input.input_mode {
            InputMode::Normal => Style::default(),
            InputMode::Editing => Color::Yellow.into(),
        };
        let width = area.width.max(3) - 3;
        let scroll = self.input.input.visual_scroll(width as usize);
        let input_elem = Paragraph::new(format!("{}", self.input.input.value()))
            .style(style)
            .scroll((0, scroll as u16))
            .block(
                Block::bordered()
                    .border_type(BorderType::Rounded)
                    .title("Chat"),
            );
        input_elem.render(input, buf);
    }
}
