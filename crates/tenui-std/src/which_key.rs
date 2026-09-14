use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// A single discovered keybinding entry in the WhichKey drawer.
#[derive(Clone, Debug, PartialEq)]
pub struct KeyAction {
    pub key: String,
    pub description: String,
}

impl KeyAction {
    pub fn new(key: impl Into<String>, description: impl Into<String>) -> Self {
        Self {
            key: key.into(),
            description: description.into(),
        }
    }
}

/// Contextual hotkey discovery popup drawer mounted at the bottom of the screen.
pub struct WhichKey {
    pub chord_title: String,
    pub actions: Vec<KeyAction>,
    pub is_open: bool,
}

impl WhichKey {
    pub fn new(chord_title: impl Into<String>) -> Self {
        Self {
            chord_title: chord_title.into(),
            actions: Vec::new(),
            is_open: false,
        }
    }

    pub fn add_action(&mut self, key: impl Into<String>, desc: impl Into<String>) {
        self.actions.push(KeyAction::new(key, desc));
    }

    pub fn open(&mut self) {
        self.is_open = true;
    }

    pub fn close(&mut self) {
        self.is_open = false;
    }

    /// Renders the floating WhichKey bottom drawer with top border and columnated actions.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>) {
        if !self.is_open || self.actions.is_empty() {
            return;
        }

        let w = surface.width();
        let h = surface.height();

        // Compute dynamic column width based on longest entry + gap
        let max_entry_len = self
            .actions
            .iter()
            .map(|a| a.key.len() + 2 + 1 + a.description.len())
            .max()
            .unwrap_or(24);
        let col_width = (max_entry_len as u16 + 3).max(28);
        let num_cols = (w.saturating_sub(6) / col_width).max(1);
        let num_rows = (self.actions.len() as u16).div_ceil(num_cols);
        let drawer_h = if h >= 8 && num_rows > 2 {
            (num_rows + 2).min(h)
        } else {
            4u16.min(h)
        };
        let start_y = h.saturating_sub(drawer_h);

        let drawer_bg = Color::Rgb(15, 23, 42); // Deep slate
        let border_color = Color::Rgb(56, 189, 248); // Cyan accent

        // Clear drawer area
        for y in start_y..h {
            for x in 0..w {
                surface.set_char(x, y, ' ', Color::WHITE, drawer_bg, Modifier::empty());
            }
        }

        // Top Border with embedded Chord Header: ───[ Space ]──────────
        for x in 0..2.min(w) {
            surface.set_char(x, start_y, '─', border_color, drawer_bg, Modifier::empty());
        }
        let header = format!("───[ {} ]───", self.chord_title);
        surface.write_str_clipped(2, start_y, &header, border_color, drawer_bg);

        let header_end = 2 + header.len() as u16;
        for x in header_end..w {
            surface.set_char(x, start_y, '─', border_color, drawer_bg, Modifier::empty());
        }

        // Render Columnated Actions across rows inside drawer
        let mut curr_x = 3u16;
        let mut curr_y = start_y + 1;

        for action in self.actions.iter() {
            if curr_x + col_width > w {
                curr_x = 3;
                curr_y += 1;
                if curr_y >= h {
                    break;
                }
            }

            // Key badge: [k]
            let key_str = format!("[{}]", action.key);
            surface.write_str_clipped(curr_x, curr_y, &key_str, Color::YELLOW, drawer_bg);

            // Description: Description text
            let desc_x = curr_x + key_str.len() as u16 + 1;
            surface.write_str_clipped(desc_x, curr_y, &action.description, Color::WHITE, drawer_bg);

            curr_x += col_width;
        }
    }
}
