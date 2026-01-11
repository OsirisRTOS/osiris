use std::path::PathBuf;

use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::Line,
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};
use toml_edit::DocumentMut;

use crate::{
    error::Result,
    state::ConfigState,
    ui::{
        ConfigUI,
        types::{ModalCmd, Modallike},
        widget::button::{BUTTON_GREEN, BUTTON_RED, Button, ButtonState},
    },
};

#[derive(Clone)]
pub struct SaveConfirmationModal {
    selected_button: usize,
    src_dir: Option<PathBuf>,
}

impl SaveConfirmationModal {
    pub fn new<P: Into<PathBuf>>(src_dir: Option<P>) -> Self {
        Self {
            selected_button: 1,
            src_dir: src_dir.map(Into::into),
        }
    }

    fn save_config(&self, state: &mut ConfigState) -> Result<()> {
        let config_path = if let Some(ref dir) = self.src_dir {
            dir.join(".cargo/config.toml")
        } else {
            PathBuf::from(".cargo/config.toml")
        };

        // Load the config file
        let config_data = std::fs::read_to_string(&config_path)?;

        // Parse the config data
        let mut doc = config_data
            .parse::<DocumentMut>()
            .map_err(|e| anyhow!("Failed to parse config file: {}", e))?;

        state.serialize_into(&mut doc)?;

        // Write back to file
        std::fs::write(config_path, doc.to_string())
            .map_err(|e| anyhow!("Failed to write config file: {}", e))?;

        Ok(())
    }
}

impl Modallike for SaveConfirmationModal {
    fn handle_key_event(&mut self, key: KeyEvent, state: &mut ConfigState) -> Result<ModalCmd> {
        match key.code {
            KeyCode::Left => {
                self.selected_button = 0; // Yes
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Right => {
                self.selected_button = 1; // No
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Tab => {
                self.selected_button = 1 - self.selected_button;
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Enter => {
                if self.selected_button == 0 {
                    self.save_config(state)?;
                }

                Ok(ModalCmd::Close)
            }
            KeyCode::Esc => Ok(ModalCmd::Close),
            _ => Ok(ModalCmd::Nothing),
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = ConfigUI::centered_rect_sized(40, 6, f.area());
        // Clear our modal area
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Save? ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        // Render block around the modal
        f.render_widget(block, area);

        let content = vec![
            Line::from("Are you sure you want to save?"),
            Line::from("config.toml will be overwritten."),
            Line::from(""),
        ];

        let chunks_vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // One line border
                Constraint::Fill(1),   // Space for text
                Constraint::Length(1), // One line buttons
                Constraint::Length(1), // One line border
            ])
            .split(area);

        let text_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // One column border
                Constraint::Fill(1),   // Space for text
                Constraint::Length(1), // One column border
            ])
            .split(chunks_vert[1]);

        let text = Paragraph::new(content).wrap(Wrap { trim: true }).centered();

        // Render the text
        f.render_widget(text, text_chunks[1]);

        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // One column border
                Constraint::Fill(1),   // Left spacing
                Constraint::Length(5), // Yes button
                Constraint::Length(6), // Gap between buttons
                Constraint::Length(5), // No button
                Constraint::Fill(1),   // Right spacing
                Constraint::Length(1), // One column border
            ])
            .split(chunks_vert[2]);

        let mut yes_button = Button::new("✓", BUTTON_GREEN);
        let mut no_button = Button::new("✗", BUTTON_RED);

        if self.selected_button == 0 {
            yes_button.set_state(ButtonState::Selected);
            no_button.set_state(ButtonState::Normal);
        } else {
            no_button.set_state(ButtonState::Selected);
            yes_button.set_state(ButtonState::Normal);
        }

        // Render buttons in their respective areas
        f.render_widget(yes_button, button_chunks[2]);
        f.render_widget(no_button, button_chunks[4]);
    }

    fn footer_text(&self) -> String {
        "←/→: Select • Tab: Toggle • Enter: Confirm • Esc: Cancel".to_string()
    }
}

#[derive(Debug, Clone)]
pub struct ExitConfirmationModal {
    selected_button: usize, // 0 = Yes, 1 = No
}

impl ExitConfirmationModal {
    pub fn new() -> Self {
        Self {
            selected_button: 1, // Default to "No" for safety
        }
    }
}

impl Modallike for ExitConfirmationModal {
    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut ConfigState) -> Result<ModalCmd> {
        match key.code {
            KeyCode::Left => {
                self.selected_button = 0; // Yes
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Right => {
                self.selected_button = 1; // No
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Tab => {
                self.selected_button = 1 - self.selected_button;
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Enter => {
                if self.selected_button == 0 {
                    Ok(ModalCmd::ExitApp)
                } else {
                    Ok(ModalCmd::Close)
                }
            }
            KeyCode::Esc => Ok(ModalCmd::Close),
            _ => Ok(ModalCmd::Nothing),
        }
    }

    fn draw(&mut self, f: &mut Frame) {
        let area = ConfigUI::centered_rect_sized(40, 6, f.area());
        // Clear our modal area
        f.render_widget(Clear, area);

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Exit? ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        // Render block around the modal
        f.render_widget(block, area);

        let content = vec![
            Line::from("Are you sure you want to exit?"),
            Line::from("Any unsaved changes will be lost."),
            Line::from(""),
        ];

        let chunks_vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // One line border
                Constraint::Fill(1),   // Space for text
                Constraint::Length(1), // One line buttons
                Constraint::Length(1), // One line border
            ])
            .split(area);

        let text_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // One column border
                Constraint::Fill(1),   // Space for text
                Constraint::Length(1), // One column border
            ])
            .split(chunks_vert[1]);

        let text = Paragraph::new(content).wrap(Wrap { trim: true }).centered();

        // Render the text
        f.render_widget(text, text_chunks[1]);

        let button_chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([
                Constraint::Length(1), // One column border
                Constraint::Fill(1),   // Left spacing
                Constraint::Length(5), // Yes button
                Constraint::Length(6), // Gap between buttons
                Constraint::Length(5), // No button
                Constraint::Fill(1),   // Right spacing
                Constraint::Length(1), // One column border
            ])
            .split(chunks_vert[2]);

        let mut yes_button = Button::new("✓", BUTTON_GREEN);
        let mut no_button = Button::new("✗", BUTTON_RED);

        if self.selected_button == 0 {
            yes_button.set_state(ButtonState::Selected);
            no_button.set_state(ButtonState::Normal);
        } else {
            no_button.set_state(ButtonState::Selected);
            yes_button.set_state(ButtonState::Normal);
        }

        // Render buttons in their respective areas
        f.render_widget(yes_button, button_chunks[2]);
        f.render_widget(no_button, button_chunks[4]);
    }

    fn footer_text(&self) -> String {
        "←/→: Select • Tab: Toggle • Enter: Confirm • Esc: Cancel".to_string()
    }
}
