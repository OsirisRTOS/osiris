use ratatui::widgets::{Scrollbar, ScrollbarOrientation, ScrollbarState};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::{Color, Modifier, Style},
    widgets::{List, ListItem, ListState, StatefulWidget},
};

#[derive(Debug, Clone)]
pub struct Dropdown<'a> {
    items: &'a [String],
    style: Style,
    highlight_style: Style,
}

#[derive(Debug, Clone, Default)]
pub struct DropdownState {
    length: usize,
    list_state: ListState,
    vertical_scroll_state: ScrollbarState,
}

impl<'a> Dropdown<'a> {
    pub fn new(items: &'a [String]) -> Self {
        Self {
            items,
            style: Style::default().fg(Color::White),
            highlight_style: Style::default()
                .bg(Color::Blue)
                .fg(Color::White)
                .add_modifier(Modifier::BOLD),
        }
    }
}

impl DropdownState {
    pub fn new(length: usize) -> Self {
        Self {
            length,
            list_state: ListState::default(),
            vertical_scroll_state: ScrollbarState::new(length),
        }
    }

    pub fn select(&mut self, index: usize) {
        self.vertical_scroll_state = self.vertical_scroll_state.position(index);
        self.list_state.select(Some(index));
    }

    pub fn selected(&self) -> usize {
        self.list_state.selected().unwrap_or_default()
    }

    pub fn next(&mut self) {
        if self.length == 0 {
            return;
        }
        let index = match self.list_state.selected() {
            Some(i) => (i + 1) % self.length,
            None => 0,
        };
        self.select(index);
    }

    pub fn previous(&mut self) {
        if self.length == 0 {
            return;
        }
        let index = match self.list_state.selected() {
            Some(i) => {
                if i == 0 {
                    self.length - 1
                } else {
                    i - 1
                }
            }
            None => self.length - 1,
        };
        self.select(index);
    }
}

impl<'a> StatefulWidget for Dropdown<'a> {
    type State = DropdownState;

    fn render(self, area: Rect, buf: &mut Buffer, state: &mut Self::State) {
        let items: Vec<ListItem> = self
            .items
            .iter()
            .map(|item| ListItem::new(item.as_str()))
            .collect();

        let list = List::new(items)
            .style(self.style)
            .highlight_style(self.highlight_style)
            .highlight_symbol("❯ ");

        // Split area to reserve space for scrollbar
        let list_area = Rect {
            x: area.x,
            y: area.y,
            width: area.width.saturating_sub(1),
            height: area.height,
        };

        let scrollbar_area = Rect {
            x: area.x + area.width.saturating_sub(1),
            y: area.y,
            width: 1,
            height: area.height,
        };

        StatefulWidget::render(list, list_area, buf, &mut state.list_state);

        let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));
        StatefulWidget::render(
            scrollbar,
            scrollbar_area,
            buf,
            &mut state.vertical_scroll_state,
        );
    }
}
