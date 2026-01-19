use super::Component;
use crate::action::Result;
use crate::components::theme::Theme;
use crate::components::ui_utils::table;
use crate::{action::Action, config::Config};
use crossterm::event::{KeyCode, KeyEvent};
use openapi::models::*;
use ratatui::{
    prelude::*,
    style::{Color, Stylize, palette::tailwind},
    widgets::*,
};
use std::sync::Arc;
use std::time::Instant;
use strum::IntoEnumIterator;
use strum::{Display, EnumIter, FromRepr};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::Input;

impl table::Data<1> for StaticRoomPublic {
    fn get_headers() -> [&'static str; 1] {
        ["Owner"]
    }

    fn ref_array(&self) -> [&String; 1] {
        [self.owner.username.as_ref().unwrap()]
    }

    fn get_row(&self, index: usize) -> &str {
        self.ref_array()[index]
    }
}

const STYLE_KEY: crate::app::Mode = crate::app::Mode::AccountManagement;

#[derive(Default, Clone, Copy, Display, FromRepr, PartialEq, EnumIter)]
pub enum Chategory {
    #[default]
    #[strum(to_string = "Profile")]
    Profile,
    #[strum(to_string = "My Rooms")]
    MyRooms,
}

impl Chategory {
    /// Get the previous tab, if there is no previous tab return the current tab.
    pub fn previous(self) -> Self {
        let current_index: usize = self as usize;
        let previous_index = current_index.saturating_sub(1);
        Self::from_repr(previous_index).unwrap_or(self)
    }

    /// Get the next tab, if there is no next tab return the current tab.
    pub fn next(self) -> Self {
        let current_index = self as usize;
        let next_index = current_index.saturating_add(1);
        Self::from_repr(next_index).unwrap_or(self)
    }

    /// Return tab's name as a styled `Line`
    pub fn title(self) -> Line<'static> {
        format!("  {self}  ")
            .fg(tailwind::SLATE.c200)
            .bg(self.palette().c900)
            .into()
    }

    /// A block surrounding the tab's content
    pub fn block(self) -> Block<'static> {
        Block::bordered()
            .border_set(symbols::border::PROPORTIONAL_TALL)
            .padding(Padding::horizontal(1))
            .border_style(self.palette().c700)
    }

    pub const fn palette(self) -> tailwind::Palette {
        match self {
            Self::MyRooms => tailwind::BLUE,
            Self::Profile => tailwind::GREEN,
        }
    }
    pub fn render(self, area: Rect, buf: &mut Buffer) -> Rect {
        let block = self.block();
        let inner = block.inner(area);
        block.render(area, buf);
        inner
    }
}

pub struct AccountManagement {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    selected_tab: Chategory,
    user: UserPrivate,
    rooms: Arc<[StaticRoomPublic]>,
    refresh_instance: Instant,
}

impl AccountManagement {
    pub fn new() -> Self {
        Self {
            active: Default::default(),
            command_tx: Default::default(),
            config: Default::default(),
            selected_tab: Default::default(),
            user: Default::default(),
            rooms: Default::default(),
            refresh_instance: Instant::now(),
        }
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
        self.render_outer_tabs(tabs_area, buf);

        let _inner = self.selected_tab.render(inner_area, buf);

        render_footer(footer_area, buf);
    }

    fn render_outer_tabs(&self, area: Rect, buf: &mut Buffer) {
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
    Line::raw("◄ H L ► to change tab | Press q to exit")
        .centered()
        .render(area, buf);
}

impl Component for AccountManagement {
    fn init(&mut self, _: Size) -> Result<()> {
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
            Action::OpenStaticRoomManagement => self.active = true,
            Action::Me(user) => self.user = user,
            Action::MyRooms(rooms) => self.rooms = rooms,
            Action::Tick => {
                if self.refresh_instance.elapsed().as_secs_f32() > 10f32 {
                    self.send(Action::RequestMyRooms);
                    self.refresh_instance = Instant::now();
                }
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
            match key.code {
                KeyCode::Char('H') if input.shift => self.previous_tab(),
                KeyCode::Char('L') if input.shift => self.next_tab(),
                _ => match self.selected_tab {
                    Chategory::Profile => {}
                    Chategory::MyRooms => {}
                },
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
