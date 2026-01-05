use crate::{
    LockErrorExt,
    action::Action,
    cli::Cli,
    components::{
        Component, chat::Chat, editor::Editor, error_display::ErrorDisplay, fps::FpsCounter,
        home::Home, join::Join, login::Login, settings::Settings, sorted_components,
    },
    config::Config,
    error::AppError,
    network,
    tui::{Event, Tui},
};
use color_eyre::Result;
use crossterm::event::KeyEvent;
use ratatui::prelude::Rect;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, RwLock};
use tokio::sync::mpsc;
use tracing::{debug, error, info};

pub struct App {
    config: Arc<RwLock<Config>>,
    args: Cli,
    components: Vec<Box<dyn Component>>,
    should_quit: bool,
    should_suspend: bool,
    mode: Mode,
    last_mode: Option<Mode>,
    last_tick_key_events: Vec<KeyEvent>,
    action_tx: mpsc::UnboundedSender<Action>,
    action_rx: mpsc::UnboundedReceiver<Action>,
}

#[derive(Default, Debug, Copy, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Mode {
    #[default]
    Home,
    Login,
    Join,
    Chat,
    Settings,
    RawSettings,
    Insert, //Special, for when one element should consume all key inputs
    Global, //Special, should never be used, only for key shortcuts that should work everywhere
}

impl App {
    pub fn new(args: Cli) -> Result<Self> {
        let (action_tx, action_rx) = mpsc::unbounded_channel();
        Ok(Self {
            args,
            components: sorted_components(vec![
                Box::new(Home::new()),
                Box::new(Chat::new()),
                Box::new(Join::new()),
                Box::new(Editor::new()),
                Box::new(Settings::new()),
                Box::new(Login::new()),
                Box::new(ErrorDisplay::new()),
                Box::new(FpsCounter::default()),
            ]),
            should_quit: false,
            should_suspend: false,
            config: Config::new_locked()?,
            mode: Mode::Home,
            last_mode: None,
            last_tick_key_events: Vec::new(),
            action_tx,
            action_rx,
        })
    }

    pub async fn run(&mut self) -> Result<()> {
        network::init(self.args.clone(), self.action_tx.clone())
            .await
            .map_err(|e| color_eyre::Report::new(e))?;
        let mut tui = Tui::new()?
            .mouse(true) // uncomment this line to enable mouse support
            .tick_rate(self.args.tick_rate)
            .frame_rate(self.args.frame_rate);
        tui.enter()?;

        for component in self.components.iter_mut() {
            component.register_action_handler(self.action_tx.clone())?;
        }
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
        }
        for component in self.components.iter_mut() {
            component.init(tui.size()?)?;
        }

        let action_tx = self.action_tx.clone();
        loop {
            self.handle_events(&mut tui).await?;
            self.handle_actions(&mut tui).await?;
            if self.should_suspend {
                tui.suspend()?;
                action_tx.send(Action::Resume)?;
                action_tx.send(Action::ClearScreen)?;
                // tui.mouse(true);
                tui.enter()?;
            } else if self.should_quit {
                tui.stop()?;
                break;
            }
        }
        tui.exit()?;
        Ok(())
    }

    fn hide_all(&mut self) {
        for component in self.components.iter_mut() {
            component.hide();
        }
    }

    fn reload_config(&mut self, tui: &mut Tui) -> Result<()> {
        self.config = Config::new_locked()?;
        for component in self.components.iter_mut() {
            component.register_config_handler(self.config.clone())?;
            component.init(tui.size()?)?;
        }
        self.hide_all();
        self.mode_to_screen()?;
        Ok(())
    }

    fn mode_to_screen(&mut self) -> Result<()> {
        match self.mode {
            Mode::Home => self.action_tx.send(Action::OpenHome),
            Mode::Join => self.action_tx.send(Action::OpenJoin),
            Mode::Login => self.action_tx.send(Action::OpenLogin),
            Mode::Chat => self.action_tx.send(Action::OpenChat),
            Mode::Settings => self.action_tx.send(Action::OpenSettings),
            Mode::RawSettings => self.action_tx.send(Action::OpenRawSettings),
            Mode::Insert => {
                self.restore_prev_mode()?;
                if self.mode == Mode::Insert {
                    self.action_tx
                        .send(Action::Error("Restoring Mode failed".into()))?;
                    self.mode = Mode::Home;
                }
                self.mode_to_screen()?;
                Ok(())
            }
            Mode::Global => {
                self.action_tx
                    .send(Action::Error("Reached Illegal state".into()))?;
                self.mode = Mode::Home;
                self.mode_to_screen()?;
                Ok(())
            }
        }?;
        Ok(())
    }

    fn restore_prev_mode(&mut self) -> Result<()> {
        if let Some(mode) = self.last_mode.take() {
            self.mode = mode;
        } else {
            self.action_tx.send(Action::Error(
                "received Normal action but last mode wasn't set.".into(),
            ))?;
            self.set_mode(Mode::Home)?;
        }
        Ok(())
    }

    async fn handle_events(&mut self, tui: &mut Tui) -> Result<()> {
        let Some(event) = tui.next_event().await else {
            return Ok(());
        };
        let action_tx = self.action_tx.clone();
        match event {
            Event::Quit => action_tx.send(Action::Quit)?,
            Event::Tick => action_tx.send(Action::Tick)?,
            Event::Render => action_tx.send(Action::Render)?,
            Event::Resize(x, y) => action_tx.send(Action::Resize(x, y))?,
            Event::Key(key) => self.handle_key_event(key)?,
            _ => {}
        }
        for component in self.components.iter_mut() {
            if let Some(action) = component.handle_events(Some(event.clone()))? {
                action_tx.send(action)?;
            }
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key: KeyEvent) -> Result<()> {
        let conf_arc = self.config.clone();
        let config = conf_arc.read().error().map_err(AppError::Error)?;
        let action_tx = self.action_tx.clone();
        let Some(keymap) = config.keybindings.get(&self.mode) else {
            return Ok(());
        };
        match keymap.get(&vec![key]) {
            Some(action) => {
                info!("Got action: {action:?}");
                action_tx.send(action.clone())?;
                return Ok(());
            }
            _ => {
                // If the key was not handled as a single key action,
                // then consider it for multi-key combinations.
                self.last_tick_key_events.push(key);

                // Check for multi-key combinations
                if let Some(action) = keymap.get(&self.last_tick_key_events) {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                    return Ok(());
                }
            }
        }
        // Global keycommands less priority
        if self.mode != Mode::Insert {
            let Some(keymap) = config.keybindings.get(&Mode::Global) else {
                return Ok(());
            };
            match keymap.get(&vec![key]) {
                Some(action) => {
                    info!("Got action: {action:?}");
                    action_tx.send(action.clone())?;
                }
                _ => {
                    // Check for multi-key combinations
                    if let Some(action) = keymap.get(&self.last_tick_key_events) {
                        info!("Got action: {action:?}");
                        action_tx.send(action.clone())?;
                    }
                }
            }
        }
        Ok(())
    }

    fn set_mode(&mut self, mode: Mode) -> Result<()> {
        if self.mode == Mode::Insert {
            self.restore_prev_mode()?;
        }
        self.hide_all();
        if self.mode != mode && self.mode == Mode::Chat {
            let _ = self.action_tx.send(Action::Leave);
        }
        self.mode = mode;
        Ok(())
    }

    async fn handle_actions(&mut self, tui: &mut Tui) -> Result<()> {
        while let Ok(action) = self.action_rx.try_recv() {
            if action != Action::Tick && action != Action::Render {
                debug!("{action:?}");
            }
            match action.clone() {
                Action::Tick => {
                    self.last_tick_key_events.drain(..);
                }
                Action::Quit => self.should_quit = true,
                Action::Suspend => self.should_suspend = true,
                Action::Resume => self.should_suspend = false,
                Action::ClearScreen => tui.terminal.clear()?,
                Action::Resize(w, h) => self.handle_resize(tui, w, h)?,
                Action::Render => self.render(tui)?,
                Action::ReloadConfig => self.reload_config(tui)?,
                //open
                Action::OpenJoin => self.set_mode(Mode::Join)?,
                Action::OpenSettings => self.set_mode(Mode::Settings)?,
                Action::OpenLogin => self.set_mode(Mode::Login)?,
                Action::OpenHome => self.set_mode(Mode::Home)?,
                Action::OpenChat => self.set_mode(Mode::Chat)?,
                Action::OpenRawSettings => self.set_mode(Mode::RawSettings)?,
                Action::Hide => self.hide_all(),
                Action::Insert => {
                    self.last_mode = Some(self.mode);
                    self.mode = Mode::Insert;
                }
                Action::Normal => self.restore_prev_mode()?,
                Action::Error(e) => error!("{e}"),
                Action::ResetConfig => {
                    {
                        let mut config = self
                            .config
                            .write()
                            .error()
                            .map_err(AppError::Error)?;
                        if config.config.safe_file.exists() {
                            std::fs::remove_file(config.config.safe_file.clone())?;
                        }
                        *config = Config::new()?;
                        debug!("Reloading config");
                    }
                    self.reload_config(tui)?;
                    let config = self.config.read().error().map_err(AppError::Error)?;
                    config.save()?;
                }
                _ => {}
            }
            for component in self.components.iter_mut() {
                if let Some(action) = component.update(action.clone())? {
                    self.action_tx.send(action)?
                }
            }
            match network::handle_actions(action.clone()).await {
                Ok(action) => {
                    if let Some(action) = action {
                        self.action_tx.send(action)?
                    }
                }
                Err(e) => {
                    let _ = self.action_tx.send(Action::Error(e.into()));
                    let _ = self.action_tx.send(Action::OpenHome);
                }
            }
        }
        Ok(())
    }

    fn handle_resize(&mut self, tui: &mut Tui, w: u16, h: u16) -> Result<()> {
        tui.resize(Rect::new(0, 0, w, h))?;
        self.render(tui)?;
        Ok(())
    }

    fn render(&mut self, tui: &mut Tui) -> Result<()> {
        tui.draw(|frame| {
            for component in self.components.iter_mut() {
                if let Err(err) = component.draw(frame, frame.area()) {
                    let _ = self
                        .action_tx
                        .send(Action::Error(format!("Failed to draw: {:?}", err).into()));
                }
            }
        })?;
        Ok(())
    }
}
