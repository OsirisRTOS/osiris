//! Button widget module for the configuration UI.
//! This module provides a customizable button widget for the UI framework.
//!
//! This code is adapted from https://github.com/ratatui/ratatui/blob/v0.26.1/examples/custom_widget.rs

// -----------------------------------------------------------------------------------------------------
// License of the original code at https://github.com/ratatui/ratatui/blob/v0.26.1/examples/custom_widget.rs:

// The MIT License (MIT)
//
// Copyright (c) 2016-2022 Florian Dehau
// Copyright (c) 2023 The Ratatui Developers
//
// Permission is hereby granted, free of charge, to any person obtaining a copy
// of this software and associated documentation files (the "Software"), to deal
// in the Software without restriction, including without limitation the rights
// to use, copy, modify, merge, publish, distribute, sublicense, and/or sell
// copies of the Software, and to permit persons to whom the Software is
// furnished to do so, subject to the following conditions:
//
// The above copyright notice and this permission notice shall be included in all
// copies or substantial portions of the Software.
//
// THE SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND, EXPRESS OR
// IMPLIED, INCLUDING BUT NOT LIMITED TO THE WARRANTIES OF MERCHANTABILITY,
// FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT. IN NO EVENT SHALL THE
// AUTHORS OR COPYRIGHT HOLDERS BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER
// LIABILITY, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING FROM,
// OUT OF OR IN CONNECTION WITH THE SOFTWARE OR THE USE OR OTHER DEALINGS IN THE
// SOFTWARE.
// -----------------------------------------------------------------------------------------------------

use ratatui::prelude::*;

/// A custom widget that renders a button with a label, theme and state.
///
/// The button can be customized with different themes and responds to state changes
/// by adjusting its visual appearance. The label is automatically centered within
/// the button's area during rendering.
///
/// # Examples
///
/// ```
/// use ratatui::prelude::*;
///
/// let mut button = Button::new("Click Me", BUTTON_GREEN);
/// button.set_state(ButtonState::Selected);
/// ```
#[derive(Debug, Clone)]
pub struct Button<'a> {
    /// The text label displayed on the button
    label: Line<'a>,
    /// The color theme used for rendering the button
    theme: ButtonTheme,
    /// The current visual state of the button
    state: ButtonState,
}

/// Represents the different visual states a button can be in.
///
/// The state affects how the button is rendered, particularly
/// which colors from the theme are used.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ButtonState {
    /// Default state - button is not selected or highlighted
    Normal,
    /// Selected state - button is currently focused or selected
    Selected,
}

/// Defines the color scheme for a button.
///
/// A button theme consists of three colors that are used in different
/// combinations depending on the button's current state.
#[derive(Debug, Clone, Copy)]
pub struct ButtonTheme {
    /// Color used for the button text
    text: Color,
    /// Background color used in normal state
    background: Color,
    /// Background color used in selected/highlighted state
    highlight: Color,
}

/// Predefined red color theme for buttons.
pub const BUTTON_RED: ButtonTheme = ButtonTheme {
    text: Color::Rgb(48, 16, 16),
    background: Color::Rgb(144, 48, 48),
    highlight: Color::Rgb(192, 64, 64),
};

/// Predefined green color theme for buttons.
pub const BUTTON_GREEN: ButtonTheme = ButtonTheme {
    text: Color::Rgb(16, 48, 16),
    background: Color::Rgb(48, 144, 48),
    highlight: Color::Rgb(64, 192, 64),
};

impl<'a> Button<'a> {
    /// Creates a new button with the specified label and theme.
    ///
    /// The button is initially created in the `Normal` state.
    ///
    /// # Arguments
    ///
    /// * `label` - The text to display on the button (can be any type that converts to `Line`)
    /// * `theme` - The color theme to use for the button
    ///
    /// # Returns
    ///
    /// A new `Button` instance with the specified label and theme
    pub fn new<T: Into<Line<'a>>>(label: T, theme: ButtonTheme) -> Button<'a> {
        Button {
            label: label.into(),
            theme,
            state: ButtonState::Normal,
        }
    }

    /// Updates the visual state of the button.
    ///
    /// This affects how the button is rendered - selected buttons
    /// use the highlight color instead of the normal background color.
    ///
    /// # Arguments
    ///
    /// * `state` - The new state for the button
    pub fn set_state(&mut self, state: ButtonState) {
        self.state = state;
    }
}

impl<'a> Widget for Button<'a> {
    /// Renders the button widget to the specified area in the buffer.
    ///
    /// The button fills the entire provided area with its background color
    /// and centers the label text within that area.
    ///
    /// # Arguments
    ///
    /// * `area` - The rectangular area where the button should be rendered
    /// * `buf` - The buffer to render into
    fn render(self, area: Rect, buf: &mut Buffer) {
        // Get the appropriate colors based on current state
        let (background, text) = self.colors();

        // Fill the entire button area with the background and text colors
        buf.set_style(area, Style::new().bg(background).fg(text));

        // Calculate centered position for the label
        // Horizontal centering: subtract label width from area width and divide by 2
        let label_x = area.x + (area.width.saturating_sub(self.label.width() as u16)) / 2;

        // Vertical centering: subtract 1 (single line height) from area height and divide by 2
        let label_y = area.y + (area.height.saturating_sub(1)) / 2;

        // Render the label at the calculated centered position
        buf.set_line(label_x, label_y, &self.label, area.width);
    }
}

impl Button<'_> {
    /// Determines the appropriate colors to use based on the button's current state.
    ///
    /// Returns a tuple of (background_color, text_color) that should be used
    /// for rendering the button.
    ///
    /// # Returns
    ///
    /// A tuple containing:
    /// - Background color (theme.background for Normal, theme.highlight for Selected)
    /// - Text color (always theme.text)
    fn colors(&self) -> (Color, Color) {
        let theme = self.theme;
        match self.state {
            ButtonState::Normal => (theme.background, theme.text),
            ButtonState::Selected => (theme.highlight, theme.text),
        }
    }
}
