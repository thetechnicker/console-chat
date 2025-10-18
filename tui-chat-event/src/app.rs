use crate::event::{AppEvent, Event, EventHandler, WidgetEvent};
use crate::widgets;
use crate::widgets::Widget;
use ratatui::DefaultTerminal;
use ratatui::crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

/// Application.
#[derive(Debug)]
pub struct App {
    pub running: bool,
    pub events: EventHandler,

    pub tab_index: usize,
    pub max_tab: usize,

    pub input: widgets::InputWidget,

    pub last_event: Option<AppEvent>,
}

impl Default for App {
    fn default() -> Self {
        Self {
            running: true,
            events: EventHandler::new(),
            tab_index: 0,
            max_tab: 2, // Num Selectable Elements + 1
            input: widgets::InputWidget::default(),
            last_event: None,
        }
    }
}

impl App {
    /// Constructs a new instance of [`App`].
    pub fn new() -> Self {
        Self::default()
    }

    /// Run the application's main loop.
    pub async fn run(mut self, mut terminal: DefaultTerminal) -> color_eyre::Result<()> {
        while self.running {
            terminal.draw(|frame| {
                frame.render_widget(&self, frame.area());
            })?;

            match self.events.next().await? {
                Event::Tick => self.tick(),
                Event::Crossterm(event) => match event {
                    ratatui::crossterm::event::Event::Key(key_event) => {
                        self.handle_key_events(key_event)?
                    }
                    _ => {}
                },
                Event::App(app_event) => {
                    self.last_event = Some(app_event.clone());
                    match app_event {
                        AppEvent::Quit => self.quit(),
                        AppEvent::KeyEvent(key_event) => {
                            match key_event.code {
                                KeyCode::Tab if key_event.kind == KeyEventKind::Press => {
                                    self.send_current_widget_event(WidgetEvent::NoFocus);
                                    self.tab_index = (self.tab_index + 1) % self.max_tab;
                                    self.send_current_widget_event(WidgetEvent::Focus);
                                }
                                KeyCode::BackTab if key_event.kind == KeyEventKind::Press => {
                                    self.send_current_widget_event(WidgetEvent::NoFocus);
                                    self.tab_index = (self.tab_index - 1) % self.max_tab;
                                    self.send_current_widget_event(WidgetEvent::Focus);
                                }
                                KeyCode::Esc => {
                                    self.send_all_widgets_event(WidgetEvent::NoFocus);
                                    self.tab_index = 0;
                                }

                                _ => {}
                            }
                            self.send_current_widget_event(WidgetEvent::KeyEvent(key_event));
                        }
                        AppEvent::WidgetEvent(w_event) => self.send_current_widget_event(w_event),
                        _ => {}
                    };
                }
            }
        }
        Ok(())
    }
    pub fn send_current_widget_event(&mut self, event: WidgetEvent) {
        if let Some(elem) = self.current_widget() {
            elem.handle_event(event)
        }
    }
    pub fn send_all_widgets_event(&mut self, event: WidgetEvent) {
        for i in 0..self.max_tab {
            if let Some(elem) = self.widget_at(i) {
                elem.handle_event(event.clone());
            }
        }
    }

    pub fn widget_at(&mut self, index: usize) -> Option<&mut dyn Widget> {
        match index {
            1 => Some(&mut self.input as &mut dyn Widget),
            _ => None,
        }
    }
    pub fn current_widget(&mut self) -> Option<&mut dyn Widget> {
        match self.tab_index {
            1 => Some(&mut self.input as &mut dyn Widget),
            _ => None,
        }
    }

    /// Handles the key events and updates the state of [`App`].
    pub fn handle_key_events(&mut self, key_event: KeyEvent) -> color_eyre::Result<()> {
        match key_event.code {
            KeyCode::Char('q') if self.tab_index == 0 => self.events.send(AppEvent::Quit),
            KeyCode::Char('c' | 'C') if key_event.modifiers == KeyModifiers::CONTROL => {
                self.events.send(AppEvent::Quit)
            }
            // Other handlers you could add here.
            _ => {
                self.events.send(AppEvent::KeyEvent(key_event));
            }
        }
        Ok(())
    }

    /// Handles the tick event of the terminal.
    ///
    /// The tick event is where you can update the state of your application with any logic that
    /// needs to be updated at a fixed frame rate. E.g. polling a server, updating an animation.
    pub fn tick(&mut self) {}

    /// Set running to false to quit the application.
    pub fn quit(&mut self) {
        self.running = false;
    }
}
