use super::Component;
use crate::action::DialogEvent;
use crate::action::Result;
use crate::app::Mode;
use crate::components::theme::Theme;
use crate::components::ui_utils::dialog::Dialog;
use crate::components::ui_utils::table;
use crate::components::ui_utils::table::TableWidget;
use crate::{action::Action, config::Config};
use crossterm::event::{KeyCode, KeyEvent};
//use openapi::models::CreateRoom;
use openapi::models::*;
use ratatui::{
    prelude::*,
    style::{Color, Stylize, palette::tailwind},
    widgets::*,
};
use std::time::Instant;
use strum::IntoEnumIterator;
use strum::{Display, EnumIter, FromRepr};
use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::Input;

fn into_static(s: String) -> &'static str {
    Box::leak(s.into_boxed_str())
}

const N: usize = 3;
impl table::Data<N> for StaticRoomPublic {
    fn get_headers() -> [&'static str; N] {
        ["Name", "Owner", "Security Level"]
    }

    fn ref_array(&self) -> [&str; N] {
        [
            &self.name,
            &self.owner.username,
            into_static(format!("{}", self.level)),
        ]
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

fn render_user(user: &UserPrivate, area: Rect, buf: &mut ratatui::buffer::Buffer) {
    let block = Block::default()
        .title("User Information")
        .borders(Borders::ALL);

    // Create user information spans
    let user_info = Text::from(vec![
        Line::from(format!("Username: {}", user.username)).centered(),
        Line::from(format!("User Type: {:?}", user.user_type)).centered(),
        Line::from(format!("Appearance: {:?}", user.appearance)).centered(),
        Line::from(format!("ID: {}", user.id)).centered(),
    ])
    .centered();

    // Render user information inside the block
    let paragraph = Paragraph::new(user_info)
        .block(block)
        .wrap(ratatui::widgets::Wrap { trim: true });

    paragraph.render(area, buf);
}

pub struct AccountManagement<'a> {
    active: bool,
    command_tx: Option<UnboundedSender<Action>>,
    config: Config,
    selected_tab: Chategory,
    user: UserPrivate,
    rooms: TableWidget<N, StaticRoomPublic>,

    new_room_dialog: Option<Dialog<'a>>,
    refresh_instance: Instant,
}

impl<'a> AccountManagement<'a> {
    pub fn new() -> Self {
        Self {
            active: Default::default(),
            command_tx: Default::default(),
            config: Default::default(),
            selected_tab: Default::default(),
            user: Default::default(),
            rooms: Default::default(),
            refresh_instance: Instant::now(),
            new_room_dialog: Default::default(),
        }
    }

    pub fn next_tab(&mut self) {
        self.selected_tab = self.selected_tab.next();
    }

    pub fn previous_tab(&mut self) {
        self.selected_tab = self.selected_tab.previous();
    }

    fn render(&mut self, area: Rect, buf: &mut Buffer) {
        use Constraint::{Length, Min};
        let vertical = Layout::vertical([Length(1), Min(0), Length(1)]);
        let [header_area, inner_area, footer_area] = vertical.areas(area);

        let horizontal = Layout::horizontal([Min(0), Length(20)]);
        let [tabs_area, title_area] = horizontal.areas(header_area);

        render_title(title_area, buf);
        self.render_outer_tabs(tabs_area, buf);

        let inner = self.selected_tab.render(inner_area, buf);
        match self.selected_tab {
            Chategory::Profile => render_user(&self.user, inner, buf),
            Chategory::MyRooms => self.rooms.render(inner, buf),
        }

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

impl Component for AccountManagement<'_> {
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
        self.rooms = TableWidget::new(Vec::new(), theme.table);
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
            Action::MyRooms(rooms) => {
                self.rooms = TableWidget::new(rooms.to_vec(), self.rooms.get_theme())
            }
            Action::Tick => {
                if self.active && self.refresh_instance.elapsed().as_secs_f32() > 10f32 {
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
            match self.new_room_dialog.as_mut() {
                None => {
                    let input: Input = key.into();
                    match key.code {
                        KeyCode::Char('H') if input.shift => self.previous_tab(),
                        KeyCode::Char('L') if input.shift => self.next_tab(),
                        KeyCode::Char('x') => {
                            self.new_room_dialog = Some(Dialog::new(
                                "TEST",
                                self.config
                                    .themes
                                    .get(&STYLE_KEY)
                                    .or(self.config.themes.get(&Mode::Global))
                                    .expect("expected global theme  but found none")
                                    .clone(),
                            ))
                        }
                        _ => match self.selected_tab {
                            Chategory::Profile => {}
                            Chategory::MyRooms => {}
                        },
                    }
                }
                Some(dialog) => {
                    if let Some(event) = dialog.handle_event(key)?.take() {
                        match event {
                            DialogEvent::Ok(_) => {
                                self.new_room_dialog = None;
                            }
                            DialogEvent::Cancel => {
                                self.new_room_dialog = None;
                            }
                            DialogEvent::Insert => return Ok(Some(Action::Insert)),
                            DialogEvent::Normal => return Ok(Some(Action::Normal)),
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

            if let Some(dialog) = self.new_room_dialog.as_ref() {
                let center = Layout::horizontal([
                    Constraint::Fill(1),
                    Constraint::Percentage(60),
                    Constraint::Fill(1),
                ])
                .split(
                    Layout::vertical([
                        Constraint::Fill(1),
                        Constraint::Percentage(60),
                        Constraint::Fill(1),
                    ])
                    .split(area)[1],
                )[1];
                dialog.render(center, buf);
            }
        }

        Ok(())
    }
}
