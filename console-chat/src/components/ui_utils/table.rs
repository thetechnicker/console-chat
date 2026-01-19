#![allow(dead_code)]
//! # Stolen from the [Ratatui] Table example
//!
//! [Ratatui]: https://github.com/ratatui/ratatui
//! [examples]: https://github.com/ratatui/ratatui/blob/main/examples
//! [examples readme]: https://github.com/ratatui/ratatui/blob/main/examples/README.md

use crate::components::ui_utils::theme::TableColors;
use ratatui::{
    buffer::Buffer,
    layout::{Constraint, Layout, Margin, Rect},
    style::{self, Modifier, Style, Stylize},
    text::Text,
    widgets::{
        Block, BorderType, Cell, HighlightSpacing, Paragraph, Row, Scrollbar, ScrollbarOrientation,
        ScrollbarState, StatefulWidget, Table, TableState, Widget,
    },
};
use style::palette::tailwind;
use unicode_width::UnicodeWidthStr;

const PALETTES: [tailwind::Palette; 4] = [
    tailwind::BLUE,
    tailwind::EMERALD,
    tailwind::INDIGO,
    tailwind::RED,
];
const INFO_TEXT: [&str; 1] =
    ["(Esc) quit | (↑) move up | (↓) move down | (←) move left | (→) move right"];

const ITEM_HEIGHT: usize = 4;

pub trait Data<const N: usize> {
    fn get_headers() -> [&'static str; N];

    fn ref_array(&self) -> [&str; N];
    fn get_row(&self, i: usize) -> &str;
}

pub struct TableWidget<const N: usize, T: Data<N>> {
    state: TableState,
    items: Vec<T>,
    longest_item_lens: [u16; N], // order is (name, address, email)
    scroll_state: ScrollbarState,
    colors: TableColors,
    color_index: usize,
}

impl<const N: usize, T: Data<N>> Default for TableWidget<N, T> {
    fn default() -> Self {
        Self::new(Vec::new(), TableColors::new(&PALETTES[0]))
    }
}

impl<const N: usize, T: Data<N>> TableWidget<N, T> {
    pub fn new(data: Vec<T>, theme: TableColors) -> Self {
        Self {
            state: TableState::default().with_selected(0),
            longest_item_lens: constraint_len_calculator(&data),
            scroll_state: ScrollbarState::new(if data.len() > 0 {
                (data.len() - 1) * ITEM_HEIGHT
            } else {
                0
            }),
            colors: theme,
            color_index: 0,
            items: data,
        }
    }
    pub fn next_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i >= self.items.len() - 1 {
                    0
                } else {
                    i + 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn previous_row(&mut self) {
        let i = match self.state.selected() {
            Some(i) => {
                if i == 0 {
                    self.items.len() - 1
                } else {
                    i - 1
                }
            }
            None => 0,
        };
        self.state.select(Some(i));
        self.scroll_state = self.scroll_state.position(i * ITEM_HEIGHT);
    }

    pub fn next_column(&mut self) {
        self.state.select_next_column();
    }

    pub fn previous_column(&mut self) {
        self.state.select_previous_column();
    }

    pub fn next_color(&mut self) {
        self.color_index = (self.color_index + 1) % PALETTES.len();
    }

    pub fn previous_color(&mut self) {
        let count = PALETTES.len();
        self.color_index = (self.color_index + count - 1) % count;
    }

    pub fn set_colors(&mut self) {
        self.colors = TableColors::new(&PALETTES[self.color_index]);
    }

    pub fn render(&mut self, area: Rect, buf: &mut Buffer) {
        let vertical = &Layout::vertical([Constraint::Min(5), Constraint::Length(4)]);
        let rects = vertical.split(area);

        self.set_colors();

        self.render_table(rects[0], buf);
        self.render_scrollbar(rects[0], buf);
        self.render_footer(rects[1], buf);
    }

    fn render_table(&mut self, area: Rect, buf: &mut Buffer) {
        let header_style = Style::default()
            .fg(self.colors.header_fg)
            .bg(self.colors.header_bg);
        let selected_row_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_row_style_fg);
        let selected_col_style = Style::default().fg(self.colors.selected_column_style_fg);
        let selected_cell_style = Style::default()
            .add_modifier(Modifier::REVERSED)
            .fg(self.colors.selected_cell_style_fg);

        let header = T::get_headers()
            .into_iter()
            .map(Cell::from)
            .collect::<Row>()
            .style(header_style)
            .height(1);
        let rows = self.items.iter().enumerate().map(|(i, data)| {
            let color = match i % 2 {
                0 => self.colors.normal_row_color,
                _ => self.colors.alt_row_color,
            };
            let item = data.ref_array();
            item.into_iter()
                .map(|content| Cell::from(Text::from(format!("\n{content}\n"))))
                .collect::<Row>()
                .style(Style::new().fg(self.colors.row_fg).bg(color))
                .height(4)
        });
        let bar = " █ ";
        let row_contraints: [Constraint; N] =
            std::array::from_fn(|i| Constraint::Min(self.longest_item_lens[i] + 1));
        let t = Table::new(rows, row_contraints)
            .header(header)
            .row_highlight_style(selected_row_style)
            .column_highlight_style(selected_col_style)
            .cell_highlight_style(selected_cell_style)
            .highlight_symbol(Text::from(vec![
                "".into(),
                bar.into(),
                bar.into(),
                "".into(),
            ]))
            .bg(self.colors.buffer_bg)
            .highlight_spacing(HighlightSpacing::Always);
        StatefulWidget::render(t, area, buf, &mut self.state);
    }

    fn render_scrollbar(&mut self, area: Rect, buf: &mut Buffer) {
        StatefulWidget::render(
            Scrollbar::default()
                .orientation(ScrollbarOrientation::VerticalRight)
                .begin_symbol(None)
                .end_symbol(None),
            area.inner(Margin {
                vertical: 1,
                horizontal: 1,
            }),
            buf,
            &mut self.scroll_state,
        );
    }

    fn render_footer(&self, area: Rect, buf: &mut Buffer) {
        let info_footer = Paragraph::new(Text::from_iter(INFO_TEXT))
            .style(
                Style::new()
                    .fg(self.colors.row_fg)
                    .bg(self.colors.buffer_bg),
            )
            .centered()
            .block(
                Block::bordered()
                    .border_type(BorderType::Double)
                    .border_style(Style::new().fg(self.colors.footer_border_color)),
            );
        Widget::render(info_footer, area, buf);
    }

    pub fn get_theme(&self) -> TableColors {
        self.colors
    }
}

fn constraint_len_calculator<T: Data<N>, const N: usize>(items: &[T]) -> [u16; N] {
    let mut max_lengths = [0u16; N];
    for (i, item) in max_lengths.iter_mut().enumerate().take(N) {
        *item = items
            .iter()
            .map(|e| e.get_row(i))
            .map(UnicodeWidthStr::width)
            .max()
            .unwrap_or(0) as u16;
    }
    max_lengths
}
