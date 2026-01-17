use crate::action::Result;
use crate::action::VimEvent;
use crate::app::Mode;
use crate::components::theme::Theme;
use crate::components::vim::VimWidget;
use crossterm::event::KeyModifiers;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    prelude::*,
    style::{Color, Stylize, palette::tailwind},
    widgets::*,
};
use strum::{Display, EnumIter, FromRepr, IntoEnumIterator};
use tokio::sync::mpsc::UnboundedSender;
use tracing::debug;
use tui_textarea::Input;

use super::Component;
use crate::{action::Action, config::Config};

const STYLE_KEY: crate::app::Mode = crate::app::Mode::Settings;

#[derive(Default, Clone, Copy, Display, FromRepr, EnumIter)]
enum Chategory {
    #[default]
    #[strum(to_string = "Basics")]
    Basic,
    #[strum(to_string = "Network")]
    Network,
    #[strum(to_string = "Design")]
    Desing,
    #[strum(to_string = "Shortcuts")]
    Shortcuts,
    #[strum(to_string = "Settings File")]
    File,
}

impl Chategory {
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

    fn render_tab0(self, area: Rect, buf: &mut Buffer, _config: &Config) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab1(self, area: Rect, buf: &mut Buffer, _config: &Config) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab2(self, area: Rect, buf: &mut Buffer, _config: &Config) {
        let block = self.block();
        let _inner = block.inner(area);
        block.render(area, buf);
    }
    fn render_tab3(self, area: Rect, buf: &mut Buffer, _config: &Config) {
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
            Self::Desing => tailwind::AMBER,
            Self::Shortcuts => tailwind::ROSE,
            Self::File => tailwind::SKY,
            Self::Network => tailwind::PURPLE,
        }
    }
    fn render(self, area: Rect, buf: &mut Buffer, config: &Config, vim: &VimWidget) {
        // in a real app these might be separate widgets
        match self {
            Self::Basic => self.render_tab0(area, buf, config),
            Self::Desing => self.render_tab1(area, buf, config),
            Self::Shortcuts => self.render_tab2(area, buf, config),
            Self::Network => self.render_tab3(area, buf, config),
            Self::File => vim.render(area, buf),
        }
    }
}

#[derive(Default)]
pub struct Settings<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    mode: Mode,
    selected_tab: Chategory,
    editor: VimWidget<'a>,
}

impl Settings<'_> {
    pub fn new() -> Self {
        Self::default()
    }
    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }
    pub fn next_mode(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_mode(&mut self) {
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
        self.selected_tab
            .render(inner_area, buf, &self.config, &self.editor);
        render_footer(footer_area, buf);
    }

    fn render_tabs(&self, area: Rect, buf: &mut Buffer) {
        let titles = Chategory::iter().map(Chategory::title);
        let highlight_style = (Color::default(), self.selected_tab.palette().c700);
        let selected_tab_index = self.selected_tab as usize;
        Tabs::new(titles)
            .highlight_style(highlight_style)
            .select(selected_tab_index)
            .padding("", "")
            .divider(" ")
            .render(area, buf);
    }

    fn send(&mut self, action: Action) {
        if let Some(action_tx) = self.command_tx.as_ref() {
            let _ = action_tx.send(action);
        }
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

impl Component for Settings<'_> {
    fn init(&mut self, _: Size) -> Result<()> {
        let content = serde_json::to_string_pretty(&self.config)?;
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
        self.editor = VimWidget::new(super::vim::VimType::MultiLine, theme.vi).with_text(content);
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
            let input: Input = key.into();
            if input.shift {
                match key.code {
                    KeyCode::Char('H') => self.previous_tab(),
                    KeyCode::Char('L') => self.next_tab(),
                    KeyCode::Char('J') => self.previous_mode(),
                    KeyCode::Char('K') => self.next_mode(),
                    _ => {}
                }
            } else {
                match self.selected_tab {
                    Chategory::Basic => {}
                    Chategory::Desing => {}
                    Chategory::Shortcuts => {}
                    Chategory::Network => {}
                    Chategory::File => {
                        if let Some(event) = self.editor.handle_event(key)? {
                            match event {
                                VimEvent::Normal => self.send(Action::Normal),
                                VimEvent::Insert => self.send(Action::Insert),
                                VimEvent::StoreConfig => self.send(Action::StoreConfig),
                                _ => {}
                            }
                        }
                    }
                }
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
