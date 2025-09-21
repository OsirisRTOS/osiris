use crossterm::event::{KeyCode, KeyEvent};
use ratatui::{
    Frame,
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Clear, Paragraph, Wrap},
};

use crate::{
    error::Result,
    state::ConfigState,
    ui::{
        ConfigUI,
        types::{ModalCmd, Modallike},
    },
};

#[derive(Debug, Clone)]
pub struct HelpModal {}

impl HelpModal {
    pub fn new() -> Self {
        Self {}
    }
}

impl Modallike for HelpModal {
    fn handle_key_event(&mut self, key: KeyEvent, _state: &mut ConfigState) -> Result<ModalCmd> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('h') => Ok(ModalCmd::Close),
            _ => Ok(ModalCmd::Nothing),
        }
    }

    fn draw(&self, f: &mut Frame) {
        let area = ConfigUI::centered_rect(60, 70, f.area());

        let help_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Bindings:",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from("  ↑               - Move up"),
            Line::from("  ↓               - Move down"),
            Line::from("  Enter/→/Space   - Enter category or edit option"),
            Line::from("  ←/Backspace     - Go back to parent category"),
            Line::from("  h               - Toggle this help"),
            Line::from("  Ctrl+S          - Save current configuration"),
            Line::from("  q/Esc           - Quit application"),
            Line::from(""),
            Line::from(""),
        ];

        let help_paragraph = Paragraph::new(help_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(" Help ")
                    .title_style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(Clear, area);
        f.render_widget(help_paragraph, area);
    }

    fn footer_text(&self) -> String {
        "Press h to hide help".to_owned()
    }
}
