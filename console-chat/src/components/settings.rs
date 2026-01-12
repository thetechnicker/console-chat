use crate::action::Result;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    style::{Color, Stylize, palette::tailwind},
    widgets::*,
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;

use super::Component;
use crate::{action::Action, config::Config};

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum SelectedTab {
    #[default]
    #[strum(to_string = "Basics")]
    Basic,
    #[strum(to_string = "Home")]
    Home,
    #[strum(to_string = "Login")]
    Login,
    #[strum(to_string = "Join")]
    Join,
    #[strum(to_string = "Chat")]
    Chat,
    #[strum(to_string = "Network")]
    Network,
}

impl SelectedTab {
    /// Get the previous tab, if there is no previous tab return the current tab.
    fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    /// Return tab's name as a styled `Line`
    fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }
    fn render_tab0(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab1(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab2(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab3(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab4(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab5(self, area: Rect, buf: &mut Buffer) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }

    /// A block surrounding the tab's content
    fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    const fn palette(self) -> tailwind::Palette {
        match self {
            Self::Basic => tailwind::BLUE,
            Self::Home => tailwind::AMBER,
            Self::Join => tailwind::EMERALD,
            Self::Chat => tailwind::ROSE,
            Self::Login => tailwind::RED,
            Self::Network => tailwind::SKY,
        }
    }
}
impl Widget for SelectedTab {
    fn render(self, area: Rect, buf: &mut Buffer) {
        // in a real app these might be separate widgets
        match self {
            Self::Basic => self.render_tab0(area, buf),
            Self::Home => self.render_tab1(area, buf),
            Self::Join => self.render_tab2(area, buf),
            Self::Chat => self.render_tab3(area, buf),
            Self::Login => self.render_tab4(area, buf),
            Self::Network => self.render_tab5(area, buf),
        }
    }
}

#[derive(Default)]
pub struct Settings {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    selected_tab: SelectedTab,
}

impl Settings {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    fn render(&self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        self.render_tabs(tabs_area, buf);
        self.selected_tab.render(inner_area, buf);
        render_footer(footer_area, buf);
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = SelectedTab::iter().map(SelectedTab::title);
        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }
}

fn render_title(area: Rect, buf: &mut Buffer) {
    "Ratatui Tabs Example".bold().render(area, buf);
}

fn render_footer(area: Rect, buf: &mut Buffer) {
    Line::raw("◄ h l ► to change tab | Press q to exit")
        .centered()
        .render(area, buf);
}

impl Component for Settings {
    fn init(&mut self, _: Size) -> Result<()> {
        Ok(())
    }

    fn register_action_handler(&mut self, tx: UnboundedSender<Action>) -> Result<()> {
        self.command_tx = Some(tx);
        Ok(())
    }

    fn hide(&mut self) {
        self.active = false;
    }

    fn register_config_handler(&mut self, config: Config) -> Result<()> {
        self.config = config;
        Ok(())
    }

    fn update(&mut self, action: Action) -> Result<Option<Action>> {
        match action {
            Action::OpenSettings => self.active = true,
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
    fn handle_key_event(&mut self, key: KeyEvent) -> Result<Option<Action>> {
        if self.active {
            match key.code {
                KeyCode::Char('h') => self.previous_tab(),
                KeyCode::Char('l') => self.next_tab(),
                _ => {}
            }
        }
        Ok(None)
    }

    fn draw(&mut self, frame: &mut Frame, area: Rect) -> Result<()> {
        if self.active {
            let [_, center] =
                Layout::vertical([Constraint::Max(1), Constraint::Fill(1)]).areas(area);
            let buf = frame.buffer_mut();
            self.render(center, buf);
        }

        Ok(())
    }
}
