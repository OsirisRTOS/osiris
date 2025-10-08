use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
    Frame,
};

use crate::{
    error::Result,
    option::ConfigOption,
    state::ConfigState,
    types::{ConfigNodelike, ConfigType, ConfigValue},
    ui::{
        types::{ModalCmd, Modallike},
        widget::{
            button::{Button, ButtonState, BUTTON_GREEN, BUTTON_RED},
            dropdown::{Dropdown, DropdownState},
        },
        ConfigUI,
    },
};

#[derive(Clone)]
pub struct EditorModal {
    /// The key path of the option being edited
    id: usize,
    /// Current input buffer
    input_buffer: String,
    /// The type constraints for validation
    config_type: ConfigType,
    /// Error message if validation fails
    error_message: Option<String>,
    selected_button: usize, // 0 = Yes, 1 = No
    dropdown_state: Option<DropdownState>,
}
impl EditorModal {
    pub fn new(node: &ConfigOption, state: &ConfigState) -> Option<Self> {
        let default = node.default_value();
        let value = state.value(&node.id()).unwrap_or(&default);

        let mut dropdown_state = None;

        // Initialize dropdown state if we have allowed values
        if let ConfigType::String(Some(allowed_values), _) = &node.typ {
            let mut ds = DropdownState::new(allowed_values.len());
            // Find current value index in allowed values
            if let ConfigValue::String(current_str) = value {
                if let Some(index) = allowed_values.iter().position(|v| current_str == v) {
                    ds.select(index);
                }
            }
            dropdown_state = Some(ds);
        }

        Some(Self {
            id: node.id(),
            input_buffer: value.to_string(),
            config_type: node.typ.clone(),
            error_message: None,
            selected_button: 0,
            dropdown_state,
        })
    }

    fn validate_and_save_value(&mut self, state: &mut ConfigState) -> Result<()> {
        let validation_result = self.validate_input()?;
        state.update_value(&self.id, validation_result)
    }

    fn validate_input(&self) -> Result<ConfigValue> {
        match &self.config_type {
            ConfigType::String(allowed_values, _) => {
                if let Some(allowed) = allowed_values {
                    if allowed.contains(&self.input_buffer) {
                        Ok(ConfigValue::String(self.input_buffer.clone()))
                    } else {
                        Err(anyhow!("Value must be one of: {}", allowed.join(", ")).into())
                    }
                } else {
                    Ok(ConfigValue::String(self.input_buffer.clone()))
                }
            }
            ConfigType::Integer(range, _) => match self.input_buffer.parse::<i64>() {
                Ok(value) => {
                    if value >= range.start && value <= range.end {
                        Ok(ConfigValue::Integer(value))
                    } else {
                        Err(
                            anyhow!("Value must be between {} and {}", range.start, range.end)
                                .into(),
                        )
                    }
                }
                Err(_) => Err(anyhow!("Invalid integer value").into()),
            },
            ConfigType::Float(range, _) => match self.input_buffer.parse::<f64>() {
                Ok(value) => {
                    if value >= range.start && value <= range.end {
                        Ok(ConfigValue::Float(value))
                    } else {
                        Err(anyhow!(
                            "Value must be between {:.2} and {:.2}",
                            range.start,
                            range.end
                        )
                        .into())
                    }
                }
                Err(_) => Err(anyhow!("Invalid float value").into()),
            },
            _ => Err(anyhow!("Unsupported config type").into()),
        }
    }

    pub fn edit(opt: &ConfigOption, state: &mut ConfigState) -> Result<Option<EditorModal>> {
        let id = opt.id();

        if !state.enabled(&id) {
            return Ok(None);
        }

        match state.value(&id) {
            Some(ConfigValue::Boolean(current)) => state
                .update_value(&id, ConfigValue::Boolean(!current))
                .map(|_| None),
            Some(_) => EditorModal::new(opt, state)
                .ok_or(anyhow!("Failed to create editor modal").into())
                .map(Some),
            None => Err(anyhow!("Selected option has no value").into()),
        }
    }

    fn draw_input_box(f: &mut Frame, area: Rect) {
        let input_box = Block::default().borders(Borders::NONE).bg(Color::Black);

        f.render_widget(input_box, area);
    }

    fn draw_content(&self, f: &mut Frame, area: Rect) {
        // Check if we should use dropdown
        if let (ConfigType::String(Some(allowed_values), _), Some(dropdown_state)) =
            (&self.config_type, &self.dropdown_state)
        {
            let dropdown = Dropdown::new(allowed_values);

            let mut state = dropdown_state.clone();
            f.render_stateful_widget(dropdown, area, &mut state);
        } else {
            // Use regular text input
            let content = vec![Line::from(vec![
                Span::styled(
                    self.input_buffer.clone(),
                    Style::default().fg(Color::Cyan).bg(Color::Black),
                ),
                Span::styled(
                    "▌",
                    Style::default()
                        .fg(Color::Cyan)
                        .bg(Color::Black)
                        .slow_blink(),
                ),
            ])];

            let input_text = Paragraph::new(content).wrap(Wrap { trim: true });
            f.render_widget(input_text, area);
        }
    }
}

impl Modallike for EditorModal {
    fn handle_key_event(&mut self, key: KeyEvent, state: &mut ConfigState) -> Result<ModalCmd> {
        match key.code {
            KeyCode::Esc => Ok(ModalCmd::Close),
            KeyCode::Enter => {
                if self.selected_button == 1 {
                    return Ok(ModalCmd::Close);
                }

                // Update input buffer from dropdown selection if applicable
                if let (ConfigType::String(Some(allowed_values), _), Some(dropdown_state)) =
                    (&self.config_type, &self.dropdown_state)
                {
                    self.input_buffer = allowed_values[dropdown_state.selected()].to_string();
                }

                if self.validate_and_save_value(state).is_err() {
                    self.error_message = Some("Invalid value".to_string());
                    return Ok(ModalCmd::Nothing);
                }

                Ok(ModalCmd::Close)
            }
            KeyCode::Up => {
                if let Some(ref mut dropdown_state) = self.dropdown_state {
                    dropdown_state.previous();
                }
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Down => {
                if let Some(ref mut dropdown_state) = self.dropdown_state {
                    dropdown_state.next();
                }
                Ok(ModalCmd::Nothing)
            }
            KeyCode::Backspace => {
                // Only allow backspace for non-dropdown inputs
                if self.dropdown_state.is_none() {
                    self.input_buffer.pop();
                    self.error_message = None;
                }
                Ok(ModalCmd::Nothing)
            }
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
            KeyCode::Char(c) => {
                // Only allow character input for non-dropdown inputs
                if self.dropdown_state.is_none() {
                    self.input_buffer.push(c);
                    self.error_message = None;
                }
                Ok(ModalCmd::Nothing)
            }
            _ => Ok(ModalCmd::Nothing),
        }
    }

    fn draw(&self, f: &mut Frame) {
        let dropdown = match &self.config_type {
            ConfigType::String(Some(_), _) => true,
            _ => false,
        };
        let area = ConfigUI::centered_rect_sized(40, 6 + dropdown as u16 * 2, f.area());

        let block = Block::default()
            .borders(Borders::ALL)
            .border_style(Style::default().fg(Color::Yellow))
            .title(" Edit ")
            .title_style(
                Style::default()
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            );

        f.render_widget(Clear, area);
        f.render_widget(block, area);

        // Everything inside the borders.
        let area = area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        let layout_vert = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(1), // Description
                Constraint::Fill(1),   // The input line
                Constraint::Length(1), // Space
                Constraint::Length(1), // Buttons
            ])
            .split(area);

        let input_area = layout_vert[1].inner(Margin::new(0, 0));
        if !dropdown {
            Self::draw_input_box(f, input_area);
        }
        //self.draw_cursor(f, input_area);
        self.draw_content(f, input_area);

        let mut description = Vec::new();

        // Add type-specific help
        match &self.config_type {
            ConfigType::String(Some(_), _) => {
                description.push(Line::from("Select one of the following values:"));
            }
            ConfigType::String(None, _) => {
                description.push(Line::from("Enter a string value:"));
            }
            ConfigType::Integer(range, _) => {
                description.push(Line::from(format!(
                    "Enter an integer between: {} to {}",
                    range.start, range.end
                )));
            }
            ConfigType::Float(range, _) => {
                description.push(Line::from(format!(
                    "Enter a float between: {:.2} to {:.2}",
                    range.start, range.end
                )));
            }
            _ => {}
        }

        let description_paragraph = Paragraph::new(description)
            .block(Block::default().borders(Borders::NONE))
            .wrap(Wrap { trim: true });

        f.render_widget(description_paragraph, layout_vert[0]);

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
            .split(layout_vert[3]);

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
