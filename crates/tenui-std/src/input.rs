use tenui_core::{
    CanvasSubviewMut, Color, Modifier,
    clipboard::{Clipboard, ClipboardTarget},
};

/// A single-line text input field with cursor management and selection/clipboard support.
#[derive(Debug, Clone, Default)]
pub struct TextInput {
    pub content: String,
    pub cursor: usize,
    /// Selection anchor (char index). `None` means no active selection.
    pub selection_anchor: Option<usize>,
}

impl TextInput {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_content(content: &str) -> Self {
        let cursor = content.chars().count();
        Self {
            content: content.to_string(),
            cursor,
            selection_anchor: None,
        }
    }

    /// Clears the field and any selection.
    pub fn clear(&mut self) {
        self.content.clear();
        self.cursor = 0;
        self.selection_anchor = None;
    }

    /// The selection as a `(start, end)` char-index range, or `None` if collapsed/absent.
    pub fn selection(&self) -> Option<(usize, usize)> {
        self.selection_anchor.and_then(|a| {
            let (s, e) = (a.min(self.cursor), a.max(self.cursor));
            if s == e { None } else { Some((s, e)) }
        })
    }

    /// Begins (or continues) a selection anchored at the current cursor.
    pub fn anchor_selection(&mut self) {
        if self.selection_anchor.is_none() {
            self.selection_anchor = Some(self.cursor);
        }
    }

    pub fn clear_selection(&mut self) {
        self.selection_anchor = None;
    }

    /// Byte offset of char index `c` within `content`.
    fn byte_of(&self, c: usize) -> usize {
        self.content
            .char_indices()
            .nth(c)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len())
    }

    pub fn insert(&mut self, ch: char) {
        let byte_pos = self
            .content
            .char_indices()
            .nth(self.cursor)
            .map(|(i, _)| i)
            .unwrap_or(self.content.len());
        self.content.insert(byte_pos, ch);
        self.cursor += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        for ch in s.chars() {
            self.insert(ch);
        }
    }

    pub fn delete_backward(&mut self) {
        if self.cursor > 0 {
            self.cursor -= 1;
            let byte_pos = self
                .content
                .char_indices()
                .nth(self.cursor)
                .map(|(i, _)| i)
                .unwrap_or(self.content.len());
            self.content.remove(byte_pos);
        }
    }

    pub fn move_left(&mut self) {
        self.cursor = self.cursor.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        let total = self.content.chars().count();
        self.cursor = (self.cursor + 1).min(total);
    }

    pub fn move_home(&mut self) {
        self.cursor = 0;
    }

    pub fn move_end(&mut self) {
        self.cursor = self.content.chars().count();
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        subview.set_string(0, 0, &self.content, fg, bg, Modifier::empty());

        // Invert cell under cursor to draw active caret
        let cx = self.cursor as u16;
        if cx < subview.width() {
            let ch = self.content.chars().nth(self.cursor).unwrap_or(' ');
            subview.set_char(cx, 0, ch, bg, fg, Modifier::REVERSE);
        }
    }
}

impl ClipboardTarget for TextInput {
    fn clipboard_copy(&self, clip: &mut Clipboard) -> String {
        match self.selection() {
            Some((s, e)) => {
                let text: String = self.content.chars().skip(s).take(e - s).collect();
                clip.copy(&text)
            }
            None => String::new(),
        }
    }

    fn clipboard_cut(&mut self, clip: &mut Clipboard) -> String {
        if let Some((s, e)) = self.selection() {
            let text: String = self.content.chars().skip(s).take(e - s).collect();
            let (bs, be) = (self.byte_of(s), self.byte_of(e));
            self.content.replace_range(bs..be, "");
            self.cursor = s;
            self.selection_anchor = None;
            clip.copy(&text)
        } else {
            String::new()
        }
    }

    fn clipboard_paste(&mut self, clip: &Clipboard) {
        if let Some((s, e)) = self.selection() {
            let (bs, be) = (self.byte_of(s), self.byte_of(e));
            self.content.replace_range(bs..be, "");
            self.cursor = s;
            self.selection_anchor = None;
        }
        let ins = clip.paste().to_string();
        let bpos = self.byte_of(self.cursor);
        self.content.insert_str(bpos, &ins);
        self.cursor += ins.chars().count();
    }

    fn clipboard_select_all(&mut self) {
        self.selection_anchor = Some(0);
        self.cursor = self.content.chars().count();
    }
}

/// A multi-line rope-based editor buffer.
#[derive(Debug, Clone)]
pub struct RopeEditor {
    pub lines: Vec<String>,
    pub cursor_line: usize,
    pub cursor_col: usize,
}

impl Default for RopeEditor {
    fn default() -> Self {
        Self::new()
    }
}

impl RopeEditor {
    pub fn new() -> Self {
        Self {
            lines: vec![String::new()],
            cursor_line: 0,
            cursor_col: 0,
        }
    }

    pub fn insert_char(&mut self, ch: char) {
        if ch == '\n' {
            let current_line = &self.lines[self.cursor_line];
            let byte_pos = current_line
                .char_indices()
                .nth(self.cursor_col)
                .map(|(i, _)| i)
                .unwrap_or(current_line.len());

            let remainder = current_line[byte_pos..].to_string();
            self.lines[self.cursor_line].truncate(byte_pos);

            self.cursor_line += 1;
            self.cursor_col = 0;
            self.lines.insert(self.cursor_line, remainder);
        } else {
            let current_line = &mut self.lines[self.cursor_line];
            let byte_pos = current_line
                .char_indices()
                .nth(self.cursor_col)
                .map(|(i, _)| i)
                .unwrap_or(current_line.len());
            current_line.insert(byte_pos, ch);
            self.cursor_col += 1;
        }
    }

    pub fn delete_backward(&mut self) {
        if self.cursor_col > 0 {
            self.cursor_col -= 1;
            let current_line = &mut self.lines[self.cursor_line];
            let byte_pos = current_line
                .char_indices()
                .nth(self.cursor_col)
                .map(|(i, _)| i)
                .unwrap_or(current_line.len());
            current_line.remove(byte_pos);
        } else if self.cursor_line > 0 {
            // Merge with previous line
            let current_line = self.lines.remove(self.cursor_line);
            self.cursor_line -= 1;
            self.cursor_col = self.lines[self.cursor_line].chars().count();
            self.lines[self.cursor_line].push_str(&current_line);
        }
    }

    /// Renders the editor text with foreground `fg` on background `bg`. Pass theme colors so the
    /// editor stays legible on light themes (a hardcoded white foreground renders white-on-white
    /// on Catppuccin Latte and similar). The caret inverts `fg`/`bg`.
    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let max_lines = subview.height() as usize;
        for (i, line) in self.lines.iter().take(max_lines).enumerate() {
            subview.set_string(0, i as u16, line, fg, bg, Modifier::empty());
        }

        // Draw cursor caret (inverted).
        let cy = self.cursor_line as u16;
        let cx = self.cursor_col as u16;
        if cy < subview.height() && cx < subview.width() {
            let line = &self.lines[self.cursor_line];
            let ch = line.chars().nth(self.cursor_col).unwrap_or(' ');
            subview.set_char(cx, cy, ch, bg, fg, Modifier::REVERSE);
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::clipboard::{ClipboardCommand, apply_clipboard_command};

    use super::*;

    #[test]
    fn test_text_input_clipboard_roundtrip() {
        let mut clip = Clipboard::new();
        let mut input = TextInput::with_content("hello world");

        // Select the trailing "world" (chars 6..11) and cut via the unified command path.
        input.cursor = 6;
        input.anchor_selection();
        input.cursor = 11;
        let esc = apply_clipboard_command(&mut input, ClipboardCommand::Cut, &mut clip);
        assert!(esc.unwrap().starts_with("\x1b]52;c;"));
        assert_eq!(input.content, "hello ");
        assert_eq!(clip.register(), "world");

        // Paste it back at the caret.
        apply_clipboard_command(&mut input, ClipboardCommand::Paste, &mut clip);
        assert_eq!(input.content, "hello world");
        assert_eq!(input.cursor, 11);
    }

    #[test]
    fn test_text_input_select_all_then_copy() {
        let mut clip = Clipboard::new();
        let mut input = TextInput::with_content("abc");
        apply_clipboard_command(&mut input, ClipboardCommand::SelectAll, &mut clip);
        apply_clipboard_command(&mut input, ClipboardCommand::Copy, &mut clip);
        assert_eq!(clip.register(), "abc");
    }
}
