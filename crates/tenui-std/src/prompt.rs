use tenui_core::{
    CanvasSubviewMut, Color, Modifier,
    clipboard::{Clipboard, ClipboardTarget},
};

/// A single suggestion candidate in the ReplInput autocomplete popup.
#[derive(Clone, Debug, PartialEq)]
pub struct SuggestionItem {
    pub label: String,
    pub detail: Option<String>,
}

impl SuggestionItem {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            detail: None,
        }
    }

    pub fn with_detail(mut self, detail: impl Into<String>) -> Self {
        self.detail = Some(detail.into());
        self
    }
}

/// Advanced REPL input widget with caret-anchored autocomplete popup and ghost text.
#[derive(Clone, Debug)]
pub struct ReplInput {
    pub text: String,
    pub cursor_pos: usize,
    pub ghost_text: Option<String>,
    pub suggestions: Vec<SuggestionItem>,
    pub selected_suggestion: Option<usize>,
    pub is_focused: bool,
    pub popup_visible: bool,
}

impl ReplInput {
    pub fn new() -> Self {
        Self {
            text: String::new(),
            cursor_pos: 0,
            ghost_text: None,
            suggestions: Vec::new(),
            selected_suggestion: None,
            is_focused: true,
            popup_visible: false,
        }
    }

    pub fn set_text(&mut self, text: impl Into<String>) {
        self.text = text.into();
        self.cursor_pos = self.text.chars().count();
    }

    pub fn set_suggestions(&mut self, items: Vec<SuggestionItem>) {
        self.suggestions = items;
        self.selected_suggestion = if self.suggestions.is_empty() { None } else { Some(0) };
        self.popup_visible = !self.suggestions.is_empty();
    }

    pub fn select_next(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        let next = match self.selected_suggestion {
            Some(idx) => (idx + 1) % self.suggestions.len(),
            None => 0,
        };
        self.selected_suggestion = Some(next);
    }

    pub fn select_prev(&mut self) {
        if self.suggestions.is_empty() {
            return;
        }
        let prev = match self.selected_suggestion {
            Some(0) | None => self.suggestions.len().saturating_sub(1),
            Some(idx) => idx - 1,
        };
        self.selected_suggestion = Some(prev);
    }

    pub fn complete_current(&mut self) -> bool {
        if let Some(idx) = self.selected_suggestion
            && let Some(item) = self.suggestions.get(idx)
        {
            self.text = item.label.clone();
            self.cursor_pos = self.text.chars().count();
            self.popup_visible = false;
            self.ghost_text = None;
            return true;
        }
        false
    }

    /// Renders the input line at row 0 along with ghost text and the caret-anchored popup.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color, prompt_prefix: &str) {
        self.render_at(surface, 0, fg, bg, prompt_prefix);
    }

    /// Renders the input line at a specific row (e.g. bottom of container) with dynamic popup flip.
    pub fn render_at(
        &self,
        surface: &mut CanvasSubviewMut<'_>,
        input_y: u16,
        fg: Color,
        bg: Color,
        prompt_prefix: &str,
    ) {
        let w = surface.width();
        let h = surface.height();
        if w < 5 || h < 1 || input_y >= h {
            return;
        }

        // 1. Render Prompt Prefix & User Text
        let prefix_len = prompt_prefix.chars().count() as u16;
        surface.write_str_clipped(0, input_y, prompt_prefix, Color::CYAN, bg);

        surface.write_str_clipped(prefix_len, input_y, &self.text, fg, bg);

        let caret_col = prefix_len + self.cursor_pos as u16;

        // 2. Render Ghost-Text Autosuggestion (if present and caret is at end of input)
        if let Some(ghost) = &self.ghost_text
            && self.cursor_pos == self.text.chars().count()
            && ghost.starts_with(&self.text)
        {
            let suffix = &ghost[self.text.len()..];
            let muted_color = fg.dim(0.55);
            surface.write_str_clipped(caret_col, input_y, suffix, muted_color, bg);
        }

        // 3. Render Hardware Cursor Block
        if self.is_focused && caret_col < w {
            let cursor_ch = self.text.chars().nth(self.cursor_pos).unwrap_or(' ');
            surface.set_char(
                caret_col,
                input_y,
                cursor_ch,
                Color::BLACK,
                Color::WHITE,
                Modifier::empty(),
            );
        }

        // 4. Render Caret-Anchored Autocomplete Popup
        if self.popup_visible && !self.suggestions.is_empty() && h > 2 {
            let popup_w = 36u16.min(w.saturating_sub(caret_col.min(w.saturating_sub(10))));
            let popup_h = (self.suggestions.len() as u16 + 2).min(8).min(h.saturating_sub(1));

            // Dynamic boundary check: if input is near the bottom, flip popup above the line!
            let (popup_x, popup_y) = if input_y + popup_h >= h {
                (
                    caret_col.min(w.saturating_sub(popup_w)),
                    input_y.saturating_sub(popup_h),
                )
            } else {
                (caret_col.min(w.saturating_sub(popup_w)), input_y + 1)
            };

            let popup_bg = Color::Rgb(15, 23, 42); // Deep slate
            let popup_border = Color::Rgb(56, 189, 248); // Cyan accent

            // Draw popup box
            for py in 0..popup_h {
                for px in 0..popup_w {
                    let rx = popup_x + px;
                    let ry = popup_y + py;
                    if rx < w && ry < h {
                        surface.set_char(rx, ry, ' ', Color::WHITE, popup_bg, Modifier::empty());
                    }
                }
            }

            // Draw items
            let max_visible = (popup_h.saturating_sub(2)) as usize;
            for (i, item) in self.suggestions.iter().take(max_visible).enumerate() {
                let row_y = popup_y + 1 + i as u16;
                let is_selected = self.selected_suggestion == Some(i);

                let item_bg = if is_selected {
                    Color::Rgb(30, 58, 138) // Elevated blue
                } else {
                    popup_bg
                };

                let item_fg = if is_selected {
                    Color::WHITE
                } else {
                    Color::Rgb(226, 232, 240)
                };

                // Clear row background
                for px in 1..(popup_w - 1) {
                    let rx = popup_x + px;
                    if rx < w && row_y < h {
                        surface.set_char(rx, row_y, ' ', item_fg, item_bg, Modifier::empty());
                    }
                }

                // Render item label & detail
                let bullet = if is_selected { "⏺ " } else { "  " };
                surface.write_str_clipped(popup_x + 1, row_y, bullet, popup_border, item_bg);
                surface.write_str_clipped(popup_x + 3, row_y, &item.label, item_fg, item_bg);

                if let Some(detail) = &item.detail {
                    let detail_x = popup_x + popup_w.saturating_sub(detail.len() as u16 + 2);
                    surface.write_str_clipped(detail_x, row_y, detail, Color::GRAY, item_bg);
                }
            }

            // Popup borders
            for px in 0..popup_w {
                surface.set_char(popup_x + px, popup_y, '─', popup_border, popup_bg, Modifier::empty());
                surface.set_char(
                    popup_x + px,
                    popup_y + popup_h - 1,
                    '─',
                    popup_border,
                    popup_bg,
                    Modifier::empty(),
                );
            }
            for py in 0..popup_h {
                surface.set_char(popup_x, popup_y + py, '│', popup_border, popup_bg, Modifier::empty());
                surface.set_char(
                    popup_x + popup_w - 1,
                    popup_y + py,
                    '│',
                    popup_border,
                    popup_bg,
                    Modifier::empty(),
                );
            }
            surface.set_char(popup_x, popup_y, '┌', popup_border, popup_bg, Modifier::empty());
            surface.set_char(
                popup_x + popup_w - 1,
                popup_y,
                '┐',
                popup_border,
                popup_bg,
                Modifier::empty(),
            );
            surface.set_char(
                popup_x,
                popup_y + popup_h - 1,
                '└',
                popup_border,
                popup_bg,
                Modifier::empty(),
            );
            surface.set_char(
                popup_x + popup_w - 1,
                popup_y + popup_h - 1,
                '┘',
                popup_border,
                popup_bg,
                Modifier::empty(),
            );
        }
    }
}

impl Default for ReplInput {
    fn default() -> Self {
        Self::new()
    }
}

impl ClipboardTarget for ReplInput {
    /// The REPL line has no selection model, so copy/cut act on the whole line.
    fn clipboard_copy(&self, clip: &mut Clipboard) -> String {
        clip.copy(&self.text)
    }

    fn clipboard_cut(&mut self, clip: &mut Clipboard) -> String {
        let escape = clip.copy(&self.text);
        self.text.clear();
        self.cursor_pos = 0;
        escape
    }

    fn clipboard_paste(&mut self, clip: &Clipboard) {
        let ins = clip.paste().to_string();
        let bpos = self
            .text
            .char_indices()
            .nth(self.cursor_pos)
            .map(|(i, _)| i)
            .unwrap_or(self.text.len());
        self.text.insert_str(bpos, &ins);
        self.cursor_pos += ins.chars().count();
    }
}
