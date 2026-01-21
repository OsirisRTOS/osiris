use anyhow::anyhow;
use crossterm::{
    event::{self, DisableMouseCapture, EnableMouseCapture, Event, KeyEventKind},
    execute,
    terminal::{
        EnterAlternateScreen, LeaveAlternateScreen, disable_raw_mode, enable_raw_mode, size,
    },
};
use ratatui::{
    Frame, Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout, Rect},
};
use std::{io, path::Path};

use crate::{
    category::ConfigCategory,
    error::Result,
    state::ConfigState,
    types::ConfigNode,
    ui::{
        base::BaseUI,
        types::{Modal, ModalCmd, Modallike},
    },
};

mod base;
mod confirm;
mod editor;
mod help;
mod types;
mod widget;

pub struct ConfigUI<'a> {
    base: BaseUI<'a>,
    /// The currently open modal
    modal: Option<Modal>,
    /// Global configuration state
    state: ConfigState<'a>,
}

#[derive(Clone)]
struct NavigationLevel<'a> {
    category: &'a ConfigCategory,
    selected_index: usize,
}

impl<'a> ConfigUI<'a> {
    pub fn new(root: &'a ConfigNode, state: ConfigState<'a>, src_dir: &Path) -> Result<Self> {
        Ok(Self {
            base: BaseUI::new(root, src_dir, &state)?,
            state,
            modal: None,
        })
    }

    pub fn run(&mut self) -> Result<()> {
        // Check terminal size before setting up
        self.check_terminal_size()?;
        Self::install_panic_hook();

        // Setup terminal
        enable_raw_mode()?;
        let mut stdout = io::stdout();
        execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
        let backend = CrosstermBackend::new(stdout);
        let mut terminal = Terminal::new(backend)?;

        let result = self.run_app(&mut terminal);

        // Restore terminal
        Self::reset_terminal()?;
        terminal.show_cursor()?;

        result
    }

    fn install_panic_hook() {
        let hook = std::panic::take_hook();

        std::panic::set_hook(Box::new(move |info| {
            Self::reset_terminal().unwrap_or_else(|e| eprintln!("Failed to reset terminal: {}", e));
            hook(info);
        }));
    }

    fn reset_terminal() -> Result<()> {
        disable_raw_mode()?;

        execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture,)?;
        Ok(())
    }

    /// Check if terminal meets minimum size requirements
    fn check_terminal_size(&self) -> Result<()> {
        const MIN_WIDTH: u16 = 80;
        const MIN_HEIGHT: u16 = 24;

        let (width, height) = size()?;

        if width < MIN_WIDTH || height < MIN_HEIGHT {
            return Err(anyhow!(
                "Terminal too small: {}x{} (minimum required: {}x{})",
                width,
                height,
                MIN_WIDTH,
                MIN_HEIGHT
            )
            .into());
        }

        Ok(())
    }

    fn run_app(&mut self, terminal: &mut Terminal<CrosstermBackend<io::Stdout>>) -> Result<()> {
        'app: loop {
            terminal.draw(|f| self.draw(f))?;

            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    let cmd = match &mut self.modal {
                        Some(modal) => modal.handle_key_event(key, &mut self.state)?,
                        None => self.base.handle_key_event(key, &mut self.state)?,
                    };

                    match cmd {
                        ModalCmd::Close => {
                            self.modal = None;
                            self.base.refresh_current_items(&self.state);
                        }
                        ModalCmd::Swap(new_modal) => {
                            self.modal = Some(new_modal);
                            self.base.refresh_current_items(&self.state);
                        }
                        ModalCmd::Nothing => {}
                        ModalCmd::ExitApp => {
                            break 'app;
                        }
                    }
                }
            }
        }
        Ok(())
    }

    fn draw(&mut self, f: &mut Frame) {
        match &mut self.modal {
            Some(modal) => {
                self.base.draw(f, Some(&modal.footer_text()), &self.state);
                modal.draw(f);
            }
            None => {
                self.base.draw(f, None, &self.state);
            }
        }
    }

    fn centered_rect(percent_x: u16, percent_y: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - percent_y) / 2),
                Constraint::Percentage(percent_y),
                Constraint::Percentage((100 - percent_y) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - percent_x) / 2),
                Constraint::Percentage(percent_x),
                Constraint::Percentage((100 - percent_x) / 2),
            ])
            .split(popup_layout[1])[1]
    }

    fn centered_rect_sized(width: u16, height: u16, r: Rect) -> Rect {
        let popup_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Percentage((100 - height) / 2),
                Constraint::Length(height),
                Constraint::Percentage((100 - height) / 2),
            ])
            .split(r);

        Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Percentage((100 - width) / 2),
                Constraint::Length(width),
                Constraint::Percentage((100 - width) / 2),
            ])
            .split(popup_layout[1])[1]
    }
}

pub fn launch_config_ui(root_node: &ConfigNode, state: ConfigState, src_dir: &Path) -> Result<()> {
    let mut ui = ConfigUI::new(root_node, state, src_dir)?;
    ui.run()
}
