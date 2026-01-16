//! Base UI module for the configuration interface.
//!
//! This module provides the core UI functionality for navigating and displaying
//! configuration options in a hierarchical tree structure. It implements a
//! terminal-based user interface using ratatui with support for:
//!
//! - Hierarchical navigation through configuration categories
//! - List-based display of configuration options and categories
//!
//! The UI is organized into three main sections:
//! - Header: Shows breadcrumb navigation
//! - Main content: Split between item list and details panel
//! - Footer: Shows available keyboard shortcuts

use std::path::{Path, PathBuf};

use anyhow::anyhow;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::{
    Frame,
    layout::{Constraint, Direction, Layout, Margin, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{
        Block, Borders, List, ListItem, ListState, Paragraph, Scrollbar, ScrollbarOrientation,
        ScrollbarState, Wrap,
    },
};

use crate::{
    category::ConfigCategory,
    error::Result,
    option::ConfigOption,
    state::ConfigState,
    types::{Attribute, ConfigNode, ConfigNodelike, ConfigValue},
    ui::{
        NavigationLevel,
        confirm::{ExitConfirmationModal, SaveConfirmationModal},
        editor::EditorModal,
        help::HelpModal,
        types::{Modal, ModalCmd},
    },
};

/// The main UI component for the configuration interface.
///
/// This struct manages the entire user interface state, including navigation,
/// display of configuration items, and integration with various modal dialogs.
/// It maintains a navigation stack to support hierarchical browsing of
/// configuration categories.
#[derive(Clone)]
pub struct BaseUI<'a> {
    /// Stack of navigation levels for hierarchical browsing.
    /// Each level represents a category and maintains its own selection state.
    nav_stack: Vec<NavigationLevel<'a>>,

    /// Current items being displayed in the main list.
    /// These are references to child nodes of the current category.
    current_items: Vec<&'a ConfigNode>,

    /// State for the main list widget, tracking selection and scroll position.
    list_state: ListState,

    /// State for the scrollbar widget.
    scrollbar_state: ScrollbarState,

    /// Breadcrumb path showing the current navigation location.
    /// Each string represents a category name in the navigation hierarchy.
    breadcrumb: Vec<String>,

    /// Source directory path for configuration files.
    src_dir: PathBuf,
}

impl<'a> BaseUI<'a> {
    /// Creates a new BaseUI instance initialized with the root configuration node.
    ///
    /// The root node must be a category, as options cannot contain child items.
    /// The UI is automatically navigated to the root category upon creation.
    ///
    /// # Arguments
    ///
    /// * `root` - The root configuration node (must be a Category)
    /// * `src_dir` - Path to the source directory for configuration files
    ///
    /// # Returns
    ///
    /// A new `BaseUI` instance
    ///
    /// # Errors
    ///
    /// Returns an error if the root node is not a category
    pub fn new(root: &'a ConfigNode, src_dir: &Path, state: &ConfigState) -> Result<Self> {
        // Ensure root is a category, not an option
        let root = match root {
            ConfigNode::Category(cat) => cat,
            ConfigNode::Option(_) => {
                return Err(anyhow!("Root node must be a category").into());
            }
        };

        let mut new_struct = Self {
            nav_stack: Vec::new(),
            current_items: Vec::new(),
            list_state: ListState::default(),
            scrollbar_state: ScrollbarState::default(),
            breadcrumb: Vec::new(),
            src_dir: src_dir.to_path_buf(),
        };

        // Initialize by navigating to the root category
        new_struct.go_next(root, state);
        Ok(new_struct)
    }

    /// Moves the list selection up or down by the specified direction.
    ///
    /// The selection is clamped to valid indices within the current items list.
    ///
    /// # Arguments
    ///
    /// * `direction` - Positive values move down, negative values move up
    fn move_selection(&mut self, direction: i32) {
        let current = self.list_state.selected().unwrap_or(0);
        let max = self.current_items.len().saturating_sub(1);

        let new_index = if direction > 0 {
            // Move down: increment but don't exceed maximum
            (current + 1).min(max)
        } else {
            // Move up: decrement but don't go below zero
            current.saturating_sub(1)
        };

        self.list_state.select(Some(new_index));
    }

    /// Handles selection/activation of the currently selected item.
    ///
    /// For categories, this navigates deeper into the hierarchy.
    /// For options, this opens an editor modal if the option is enabled.
    /// Disabled items are ignored.
    ///
    /// # Arguments
    ///
    /// * `state` - Current configuration state for checking enabled status
    ///
    /// # Returns
    ///
    /// An optional Modal if an editor should be opened, None otherwise
    ///
    /// # Errors
    ///
    /// Returns an error if navigation fails
    fn click_selection(&mut self, state: &mut ConfigState) -> Result<Option<Modal>> {
        if let Some(selected) = self.list_state.selected() {
            if let Some(item) = self.current_items.get(selected) {
                // Skip disabled items
                if !state.enabled(&item.id()) {
                    return Ok(None);
                }

                match item {
                    // Navigate into category
                    ConfigNode::Category(cat) => {
                        self.go_next(cat, state);
                    }
                    // Open editor for option
                    ConfigNode::Option(opt) => {
                        return Ok(EditorModal::edit(opt, state)?.map(Modal::Editor));
                    }
                }
            }
        }
        Ok(None)
    }

    fn children_nodes(category: &'a ConfigCategory, state: &ConfigState) -> Vec<&'a ConfigNode> {
        category
            .children
            .iter()
            .filter(|item| !item.has_attribute(&Attribute::Hidden))
            .filter(|item| {
                if item.has_attribute(&Attribute::NoHiddenPreview) {
                    return item.visible(state);
                }
                true
            })
            .fold(Vec::new(), |mut acc, item| {
                if let ConfigNode::Category(cat) = item {
                    if item.has_attribute(&Attribute::Skip) {
                        acc.extend(Self::children_nodes(&cat, state));
                        return acc;
                    }
                }

                acc.push(item);
                acc
            })
    }

    /// Navigates back to the previous level in the hierarchy.
    ///
    /// This pops the current navigation level and restores the previous one,
    /// including its selection state. Cannot go back from the root level.
    ///
    /// # Errors
    ///
    /// Returns an error if restoring the previous level fails
    fn go_back(&mut self, state: &ConfigState) -> Result<()> {
        // Don't go back from root level
        if self.nav_stack.len() > 1 {
            // Remove current level from stack and breadcrumb
            self.nav_stack.pop();
            self.breadcrumb.pop();

            // Restore previous level state
            if let Some(prev_level) = self.nav_stack.last().cloned() {
                // Restore the items and selection from the previous level
                self.current_items = Self::children_nodes(prev_level.category, state);
                self.list_state.select(Some(prev_level.selected_index));
            }
        }
        Ok(())
    }

    /// Navigates forward into a category, adding it to the navigation stack.
    ///
    /// This saves the current selection state, updates the current items to
    /// the category's children, and adds the category to the breadcrumb path.
    ///
    /// # Arguments
    ///
    /// * `category` - The category to navigate into
    pub fn go_next(&mut self, category: &'a ConfigCategory, state: &ConfigState) {
        // Save current selection state before navigating
        if let Some(current_level) = self.nav_stack.last_mut() {
            current_level.selected_index = self.list_state.selected().unwrap_or(0);
        }

        // Update current items to category's children
        self.current_items = Self::children_nodes(category, state);

        // Add to breadcrumb navigation
        self.breadcrumb.push(category.name.clone());

        // Create and push new navigation level
        let nav_level = NavigationLevel {
            category,
            selected_index: 0,
        };
        self.nav_stack.push(nav_level);

        // Reset selection to first item
        self.list_state.select(Some(0));
    }

    /// Handles keyboard input events and returns appropriate modal commands.
    ///
    /// This method processes all keyboard interactions including navigation,
    /// selection, and modal activation. Key bindings include:
    /// - q/Esc: Exit confirmation
    /// - Ctrl+S: Save confirmation
    /// - h: Help modal
    /// - Up/Down: Navigate list
    /// - Enter/Right/Space: Select/activate item
    /// - Left/Backspace: Go back
    ///
    /// # Arguments
    ///
    /// * `key` - The key event to process
    /// * `state` - Mutable reference to configuration state
    ///
    /// # Returns
    ///
    /// A `ModalCmd` indicating what action should be taken
    ///
    /// # Errors
    ///
    /// Returns an error if any navigation or modal creation fails
    pub fn handle_key_event(&mut self, key: KeyEvent, state: &mut ConfigState) -> Result<ModalCmd> {
        match key.code {
            // Exit application with confirmation
            KeyCode::Char('q') | KeyCode::Esc => Ok(ModalCmd::Swap(Modal::ExitConfirmation(
                ExitConfirmationModal::new(),
            ))),

            // Save configuration with confirmation
            KeyCode::Char('s') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                Ok(ModalCmd::Swap(Modal::SaveConfirmation(
                    SaveConfirmationModal::new(Some(self.src_dir.clone())),
                )))
            }

            // Show help modal
            KeyCode::Char('h') => Ok(ModalCmd::Swap(Modal::Help(HelpModal::new()))),

            // Navigate down in list
            KeyCode::Down => {
                self.move_selection(1);
                Ok(ModalCmd::Nothing)
            }

            // Navigate up in list
            KeyCode::Up => {
                self.move_selection(-1);
                Ok(ModalCmd::Nothing)
            }

            // Select/activate current item
            KeyCode::Enter | KeyCode::Right | KeyCode::Char(' ') => self
                .click_selection(state)
                .map(|modal| modal.map_or(ModalCmd::Nothing, ModalCmd::Swap)),

            // Go back to previous level
            KeyCode::Left | KeyCode::Backspace => {
                self.go_back(state)?;
                Ok(ModalCmd::Nothing)
            }

            // Ignore other keys
            _ => Ok(ModalCmd::Nothing),
        }
    }

    /// Renders the entire UI to the terminal frame.
    ///
    /// The UI is laid out in three main sections:
    /// - Header (3 lines): Breadcrumb navigation
    /// - Main content (remaining space): Item list and details panel
    /// - Footer (3 lines): Keyboard shortcuts
    ///
    /// # Arguments
    ///
    /// * `f` - The terminal frame to render into
    /// * `footer_text` - Optional custom footer text (uses default if None)
    /// * `state` - Current configuration state for display purposes
    pub fn draw(&mut self, f: &mut Frame, footer_text: Option<&str>, state: &ConfigState) {
        // Create main layout with header, content, and footer
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([
                Constraint::Length(3), // Header area
                Constraint::Min(0),    // Main content area (flexible)
                Constraint::Length(3), // Footer area
            ])
            .split(f.area());

        // Render each section
        self.draw_header(f, chunks[0]);
        self.draw_main_content(f, chunks[1], state);
        self.draw_footer(f, chunks[2], footer_text.unwrap_or(&self.footer_text()));
    }

    /// Renders the header section with breadcrumb navigation.
    ///
    /// The header displays the current navigation path as a breadcrumb trail,
    /// showing how deep the user is in the configuration hierarchy.
    ///
    /// # Arguments
    ///
    /// * `f` - The terminal frame to render into
    /// * `area` - The rectangular area for the header
    fn draw_header(&self, f: &mut Frame, area: Rect) {
        let breadcrumb_text = self.breadcrumb.join(" › ");
        let header = Paragraph::new(breadcrumb_text)
            .style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue))
                    .title(" Config ")
                    .title_style(
                        Style::default()
                            .fg(Color::White)
                            .add_modifier(Modifier::BOLD),
                    ),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(header, area);
    }

    /// Renders the main content area with item list and details panel.
    ///
    /// The main content is split horizontally:
    /// - Left side: Scrollable list of configuration items
    /// - Right side: Details panel for selected item
    ///
    /// The list includes a scrollbar and highlights the selected item.
    ///
    /// # Arguments
    ///
    /// * `f` - The terminal frame to render into
    /// * `area` - The rectangular area for the main content
    /// * `state` - Current configuration state for item rendering
    fn draw_main_content(&mut self, f: &mut Frame, area: Rect, state: &ConfigState) {
        let selected = self.list_state.selected();
        let items_count = self.current_items.len();

        // Split main content area horizontally
        let chunks = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(70), Constraint::Percentage(30)])
            .split(area);

        let list_area = chunks[0];
        let details_area = chunks[1];

        // Convert config nodes to list items for display
        let items: Vec<ListItem> = self
            .current_items
            .iter()
            .enumerate()
            .map(|(_, item)| Self::to_list_item(item, state))
            .collect();

        // Create and render the main list widget
        let list = List::new(items)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue))
                    .title(format!(" {items_count} items "))
                    .title_style(Style::default().fg(Color::White)),
            )
            .highlight_style(
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("❯ ");

        f.render_stateful_widget(list, list_area, &mut self.list_state);

        // Configure and render scrollbar for the list
        let scrollbar = Scrollbar::default()
            .orientation(ScrollbarOrientation::VerticalRight)
            .begin_symbol(Some("↑"))
            .end_symbol(Some("↓"));

        // Update scrollbar state based on current content and selection
        self.scrollbar_state = self.scrollbar_state.content_length(items_count);

        if let Some(selected_idx) = selected {
            self.scrollbar_state = self.scrollbar_state.position(selected_idx);
        }

        // Render scrollbar in the inner area of the list (avoiding borders)
        let scrollbar_area = list_area.inner(Margin {
            vertical: 1,
            horizontal: 0,
        });

        f.render_stateful_widget(scrollbar, scrollbar_area, &mut self.scrollbar_state);

        // Render details panel for the selected item
        if let Some(selected_idx) = selected {
            if let Some(item) = self.current_items.get(selected_idx) {
                Self::draw_details_panel_in_area(f, details_area, item);
            }
        }
    }

    /// Renders the details panel for a specific configuration item.
    ///
    /// The details panel shows comprehensive information about the selected item:
    /// - Item name as the panel title
    /// - Description section
    /// - Type information section
    ///
    /// # Arguments
    ///
    /// * `f` - The terminal frame to render into
    /// * `area` - The rectangular area for the details panel
    /// * `item` - The configuration item to show details for
    fn draw_details_panel_in_area(f: &mut Frame, area: Rect, item: &ConfigNode) {
        // Create inner area with margins for better visual appearance
        let details_area = area.inner(Margin {
            vertical: 1,
            horizontal: 2,
        });

        // Skip rendering if area is too small
        if details_area.height < 3 {
            return;
        }

        // Extract information based on item type
        let (title, description, details) = match item {
            ConfigNode::Category(ConfigCategory {
                name, description, ..
            }) => (
                name.clone(),
                description
                    .clone()
                    .unwrap_or_else(|| "No description".to_string()),
                "Category".to_string(),
            ),
            ConfigNode::Option(ConfigOption {
                name,
                description,
                typ,
                ..
            }) => (
                name.clone(),
                description
                    .clone()
                    .unwrap_or_else(|| "No description".to_string()),
                format!("{typ}\n",),
            ),
        };

        // Create formatted text with sections
        let detail_text = vec![
            Line::from(""),
            Line::from(vec![Span::styled(
                "Description",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(description),
            Line::from(""),
            Line::from(vec![Span::styled(
                "Type",
                Style::default()
                    .fg(Color::Green)
                    .add_modifier(Modifier::BOLD),
            )]),
            Line::from(details),
        ];

        // Create and render the details paragraph widget
        let details_paragraph = Paragraph::new(detail_text)
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Yellow))
                    .title(format!(" {title} "))
                    .title_style(Style::default().fg(Color::White)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(details_paragraph, details_area);
    }

    /// Renders the footer section with keyboard shortcuts.
    ///
    /// The footer displays available keyboard commands to help users
    /// understand how to interact with the interface.
    ///
    /// # Arguments
    ///
    /// * `f` - The terminal frame to render into
    /// * `area` - The rectangular area for the footer
    /// * `footer_text` - The text to display in the footer
    fn draw_footer(&self, f: &mut Frame, area: Rect, footer_text: &str) {
        let footer = Paragraph::new(footer_text)
            .style(Style::default().fg(Color::Gray))
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Blue)),
            )
            .wrap(Wrap { trim: true });

        f.render_widget(footer, area);
    }

    /// Returns the default footer text with keyboard shortcuts.
    ///
    /// This provides a comprehensive list of available keyboard commands
    /// for navigating and interacting with the configuration interface.
    ///
    /// # Returns
    ///
    /// A string containing formatted keyboard shortcut descriptions
    fn footer_text(&self) -> String {
        "↑/↓: Navigate • Enter/→/Space: Open/Edit • ←/Backspace: Back • Ctrl+S: Save • h: Help • q/Esc: Quit".to_owned()
    }

    /// Refreshes the current items list from the active navigation level.
    ///
    /// This is useful when the configuration state has changed and the
    /// display needs to be updated to reflect current values and visibility.
    pub fn refresh_current_items(&mut self, state: &ConfigState) {
        if let Some(current_level) = self.nav_stack.last().cloned() {
            self.current_items = Self::children_nodes(current_level.category, state);
        }
    }

    /// Converts a configuration node into a formatted list item for display.
    ///
    /// This method handles the visual representation of both categories and options,
    /// including appropriate icons, colors, and value displays. The appearance
    /// changes based on the item's visibility and current configuration state.
    ///
    /// # Arguments
    ///
    /// * `item` - The configuration node to convert
    /// * `state` - Current configuration state for value lookup
    ///
    /// # Returns
    ///
    /// A `ListItem`
    fn to_list_item<'res>(item: &ConfigNode, state: &ConfigState) -> ListItem<'res> {
        let visible = item.visible(state);

        match item {
            // Handle category items
            ConfigNode::Category(cat) => {
                let icon = "📁";

                // Choose colors based on visibility
                let (icon_color, text_color, text_modifier) = if visible {
                    (Color::Blue, Color::White, Modifier::BOLD)
                } else {
                    (Color::DarkGray, Color::DarkGray, Modifier::empty())
                };

                let line = Line::from(vec![
                    Span::styled(format!("{icon} "), Style::default().fg(icon_color)),
                    Span::styled(
                        cat.name.clone(),
                        Style::default().fg(text_color).add_modifier(text_modifier),
                    ),
                ]);
                ListItem::new(line)
            }

            // Handle option items
            ConfigNode::Option(opt) => {
                // Choose base colors based on visibility
                let (text_color, text_modifier) = if visible {
                    (Color::White, Modifier::empty())
                } else {
                    (Color::DarkGray, Modifier::DIM)
                };

                let value_color = if visible {
                    Color::Cyan
                } else {
                    Color::DarkGray
                };

                // Format the line based on the option's current value
                let line = match state.value(&opt.id()) {
                    Some(ConfigValue::Boolean(value)) => {
                        let status = if *value { "✓" } else { "✗" };
                        let mut status_color = if *value { Color::Green } else { Color::Red };

                        if !visible {
                            status_color = Color::DarkGray;
                        }

                        Line::from(vec![
                            Span::styled(
                                opt.name.clone(),
                                Style::default().fg(text_color).add_modifier(text_modifier),
                            ),
                            Span::styled(format!(" {status}"), Style::default().fg(status_color)),
                        ])
                    }

                    Some(ConfigValue::String(string_value)) => {
                        let display_value = if string_value.is_empty() {
                            "\"\"".to_string()
                        } else {
                            format!("\"{}\"", string_value)
                        };

                        Line::from(vec![
                            Span::styled(
                                opt.name.clone(),
                                Style::default().fg(text_color).add_modifier(text_modifier),
                            ),
                            Span::styled(" = ", Style::default().fg(Color::Gray)),
                            Span::styled(display_value, Style::default().fg(value_color)),
                        ])
                    }

                    Some(ConfigValue::Integer(int_value)) => Line::from(vec![
                        Span::styled(
                            opt.name.clone(),
                            Style::default().fg(text_color).add_modifier(text_modifier),
                        ),
                        Span::styled(" = ", Style::default().fg(Color::Gray)),
                        Span::styled(int_value.to_string(), Style::default().fg(value_color)),
                    ]),

                    // Float values: show with 2 decimal places
                    Some(ConfigValue::Float(float_value)) => Line::from(vec![
                        Span::styled(
                            opt.name.clone(),
                            Style::default().fg(text_color).add_modifier(text_modifier),
                        ),
                        Span::styled(" = ", Style::default().fg(Color::Gray)),
                        Span::styled(
                            format!("{:.2}", float_value),
                            Style::default().fg(value_color),
                        ),
                    ]),

                    // Fallback for mismatched types or missing values
                    _ => Line::from(vec![
                        Span::styled(
                            opt.name.clone(),
                            Style::default().fg(text_color).add_modifier(text_modifier),
                        ),
                        Span::styled(" = ", Style::default().fg(Color::Gray)),
                        Span::styled(format!("{}", opt.typ), Style::default().fg(value_color)),
                    ]),
                };

                ListItem::new(line)
            }
        }
    }
}
