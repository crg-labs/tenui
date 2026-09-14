//! `TextBuffer` — the shared, grapheme-aware editing core.
//!
//! The Architecture Spec specifies "grapheme-aware TextInput/TextArea/RopeEditor" backed by
//! the multi-cursor + branching-undo topology (`tenui-text`). `TextBuffer` is that shared
//! core: text + a selection cursor + a [`BranchingUndoTree`], with insert/delete/selection,
//! clipboard copy/cut/paste, and undo/redo — so widgets are thin views over one editor
//! instead of each re-implementing insert/delete.
//!
//! Storage is a `String` and edits are byte-offset splices. Cursor motion is by UTF-8
//! codepoint boundary.
// ponytail: String storage → O(n) splice per edit; the API is rope-ready, so a chunked
// rope backend can slot in behind it if multi-megabyte editing ever matters.

use tenui_core::clipboard::{Clipboard, ClipboardTarget};

use crate::{
    clipboard_ops,
    multi_cursor::{BranchingUndoTree, Cursor, TextEditDelta},
};

/// A single-selection text editor model: content, cursor, and branching undo history.
#[derive(Debug, Clone)]
pub struct TextBuffer {
    text: String,
    cursor: Cursor,
    undo: BranchingUndoTree,
}

impl Default for TextBuffer {
    fn default() -> Self {
        Self::new()
    }
}

impl TextBuffer {
    pub fn new() -> Self {
        Self::from_text("")
    }

    pub fn from_text(s: &str) -> Self {
        Self {
            text: s.to_string(),
            cursor: Cursor::point(0),
            undo: BranchingUndoTree::new(vec![Cursor::point(0)]),
        }
    }

    pub fn text(&self) -> &str {
        &self.text
    }

    pub fn cursor(&self) -> Cursor {
        self.cursor
    }

    /// The selection's byte range (empty when there is no selection).
    pub fn selection(&self) -> (usize, usize) {
        (self.snap(self.cursor.start()), self.snap(self.cursor.end()))
    }

    pub fn has_selection(&self) -> bool {
        self.cursor.is_selection()
    }

    // boundary helpers

    pub fn snap(&self, i: usize) -> usize {
        let mut i = i.min(self.text.len());
        while i > 0 && !self.text.is_char_boundary(i) {
            i -= 1;
        }
        i
    }

    fn prev_boundary(&self, i: usize) -> usize {
        let i = self.snap(i);
        if i == 0 {
            return 0;
        }
        let mut j = i - 1;
        while j > 0 && !self.text.is_char_boundary(j) {
            j -= 1;
        }
        j
    }

    fn next_boundary(&self, i: usize) -> usize {
        let mut j = self.snap(i);
        if j >= self.text.len() {
            return self.text.len();
        }
        j += 1;
        while j < self.text.len() && !self.text.is_char_boundary(j) {
            j += 1;
        }
        j
    }

    // cursor motion

    pub fn set_head(&mut self, head: usize, extend: bool) {
        if extend {
            self.cursor.head = head;
        } else {
            self.cursor = Cursor::point(head);
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        let h = self.prev_boundary(self.cursor.head);
        self.set_head(h, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        let h = self.next_boundary(self.cursor.head);
        self.set_head(h, extend);
    }

    pub fn move_home(&mut self, extend: bool) {
        let h = self.line_start(self.cursor.head);
        self.set_head(h, extend);
    }

    pub fn move_end(&mut self, extend: bool) {
        let h = self.line_end(self.cursor.head);
        self.set_head(h, extend);
    }

    pub fn move_up(&mut self, extend: bool) {
        let (row, col) = self.line_col(self.cursor.head);
        if row > 0 {
            let h = self.offset_at(row - 1, col);
            self.set_head(h, extend);
        }
    }

    pub fn move_down(&mut self, extend: bool) {
        let (row, col) = self.line_col(self.cursor.head);
        if row + 1 < self.line_count() {
            let h = self.offset_at(row + 1, col);
            self.set_head(h, extend);
        }
    }

    pub fn select_all(&mut self) {
        self.cursor = Cursor {
            anchor: 0,
            head: self.text.len(),
        };
    }

    // editing

    /// Replaces `range` with `inserted`, records a single-delta undo step, and leaves the
    /// cursor collapsed just after the inserted text.
    fn apply(&mut self, range: (usize, usize), inserted: &str) {
        let (s, e) = (self.snap(range.0), self.snap(range.1).max(self.snap(range.0)));
        let replaced = self.text[s..e].to_string();
        self.text.replace_range(s..e, inserted);
        self.cursor = Cursor::point(s + inserted.len());
        self.undo.commit_edit(
            vec![TextEditDelta {
                range: (s, e),
                replaced_text: replaced,
                inserted_text: inserted.to_string(),
            }],
            vec![self.cursor],
        );
    }

    /// Inserts `s`, replacing the current selection if any.
    pub fn insert(&mut self, s: &str) {
        let sel = self.selection();
        self.apply(sel, s);
    }

    pub fn insert_char(&mut self, ch: char) {
        let mut buf = [0u8; 4];
        self.insert(ch.encode_utf8(&mut buf));
    }

    /// Deletes the selection, or the codepoint before the cursor.
    pub fn backspace(&mut self) {
        let (a, b) = self.selection();
        if a != b {
            self.apply((a, b), "");
        } else if a > 0 {
            let prev = self.prev_boundary(a);
            self.apply((prev, a), "");
        }
    }

    /// Deletes the selection, or the codepoint after the cursor.
    pub fn delete_forward(&mut self) {
        let (a, b) = self.selection();
        if a != b {
            self.apply((a, b), "");
        } else {
            let n = self.next_boundary(b);
            if n > b {
                self.apply((b, n), "");
            }
        }
    }

    // clipboard

    /// Copies the selection to `clip`, returning the OSC 52 escape to emit.
    pub fn copy(&self, clip: &mut Clipboard) -> String {
        clipboard_ops::copy_selection(clip, &self.text, &self.cursor)
    }

    /// Cuts the selection into `clip`, applying the deletion. Returns the escape to emit.
    pub fn cut(&mut self, clip: &mut Clipboard) -> String {
        if let Some((delta, escape)) = clipboard_ops::cut_selection(clip, &self.text, &self.cursor) {
            self.apply(delta.range, "");
            escape
        } else {
            String::new()
        }
    }

    /// Pastes `clip`'s register at the cursor (replacing any selection).
    pub fn paste(&mut self, clip: &Clipboard) {
        let delta = clipboard_ops::paste(clip, &self.text, &self.cursor);
        self.apply(delta.range, &delta.inserted_text);
    }

    // undo / redo

    /// Reverts the last edit. Returns whether anything was undone.
    pub fn undo(&mut self) -> bool {
        let restored = self.undo.undo().map(|(d, c)| (d.to_vec(), c.to_vec()));
        if let Some((deltas, cursors)) = restored {
            // Invert each delta in reverse: text currently has `inserted_text` at
            // `[s, s+inserted.len())`; put `replaced_text` back.
            for d in deltas.iter().rev() {
                let s = self.snap(d.range.0);
                let end = (s + d.inserted_text.len()).min(self.text.len());
                self.text.replace_range(s..end, &d.replaced_text);
            }
            self.cursor = cursors.first().copied().unwrap_or(Cursor::point(0));
            self.clamp_cursor();
            true
        } else {
            false
        }
    }

    /// Re-applies a previously undone edit along child branch `branch_index`.
    pub fn redo(&mut self, branch_index: usize) -> bool {
        let restored = self.undo.redo(branch_index).map(|(d, c)| (d.to_vec(), c.to_vec()));
        if let Some((deltas, cursors)) = restored {
            for d in &deltas {
                let s = self.snap(d.range.0);
                let e = self.snap(d.range.1).max(s);
                self.text.replace_range(s..e, &d.inserted_text);
            }
            self.cursor = cursors.first().copied().unwrap_or(Cursor::point(0));
            self.clamp_cursor();
            true
        } else {
            false
        }
    }

    fn clamp_cursor(&mut self) {
        self.cursor.anchor = self.snap(self.cursor.anchor);
        self.cursor.head = self.snap(self.cursor.head);
    }

    // line mapping (for rendering / vertical motion)

    pub fn line_count(&self) -> usize {
        self.text.bytes().filter(|&b| b == b'\n').count() + 1
    }

    /// Byte offset of the start of the line containing byte `pos`.
    fn line_start(&self, pos: usize) -> usize {
        let pos = self.snap(pos);
        self.text[..pos].rfind('\n').map(|i| i + 1).unwrap_or(0)
    }

    /// Byte offset of the end of the line containing byte `pos` (before the newline).
    fn line_end(&self, pos: usize) -> usize {
        let pos = self.snap(pos);
        self.text[pos..].find('\n').map(|i| pos + i).unwrap_or(self.text.len())
    }

    /// (row, column-in-codepoints) of byte offset `pos`.
    pub fn line_col(&self, pos: usize) -> (usize, usize) {
        let pos = self.snap(pos);
        let row = self.text[..pos].bytes().filter(|&b| b == b'\n').count();
        let start = self.line_start(pos);
        let col = self.text[start..pos].chars().count();
        (row, col)
    }

    /// Byte offset of (row, column-in-codepoints), clamped to the line's length.
    fn offset_at(&self, row: usize, col: usize) -> usize {
        let mut start = 0usize;
        for (i, line) in self.text.split('\n').enumerate() {
            if i == row {
                let mut off = start;
                for (c, ch) in line.chars().enumerate() {
                    if c == col {
                        return off;
                    }
                    off += ch.len_utf8();
                }
                return start + line.len();
            }
            start += line.len() + 1; // + '\n'
        }
        self.text.len()
    }

    /// Convenience: (row, col) of the caret head.
    pub fn cursor_line_col(&self) -> (usize, usize) {
        self.line_col(self.cursor.head)
    }

    // search

    /// All non-overlapping match ranges of `needle` in the buffer (see [`crate::search`]).
    pub fn find(&self, needle: &str, case_sensitive: bool) -> Vec<(usize, usize)> {
        crate::search::find_all(&self.text, needle, case_sensitive)
    }

    /// Selects the byte range `[start, end)` (used to jump to a search match).
    pub fn select_range(&mut self, start: usize, end: usize) {
        self.cursor = Cursor {
            anchor: self.snap(start),
            head: self.snap(end),
        };
    }
}

impl ClipboardTarget for TextBuffer {
    fn clipboard_copy(&self, clip: &mut Clipboard) -> String {
        self.copy(clip)
    }
    fn clipboard_cut(&mut self, clip: &mut Clipboard) -> String {
        self.cut(clip)
    }
    fn clipboard_paste(&mut self, clip: &Clipboard) {
        self.paste(clip)
    }
    fn clipboard_select_all(&mut self) {
        self.select_all()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_insert_and_backspace() {
        let mut b = TextBuffer::new();
        b.insert("hello");
        assert_eq!(b.text(), "hello");
        b.backspace();
        assert_eq!(b.text(), "hell");
        assert_eq!(b.cursor_line_col(), (0, 4));
    }

    #[test]
    fn test_selection_replace_and_delete() {
        let mut b = TextBuffer::from_text("hello world");
        b.set_head(0, false);
        b.move_right(true); // select "h"
        b.move_right(true); // select "he"
        assert!(b.has_selection());
        b.insert("HE"); // replace selection
        assert_eq!(b.text(), "HEllo world");
    }

    #[test]
    fn test_clipboard_copy_cut_paste() {
        let mut clip = Clipboard::new();
        let mut b = TextBuffer::from_text("hello world");
        // Select "hello"
        b.set_head(0, false);
        for _ in 0..5 {
            b.move_right(true);
        }
        let esc = b.copy(&mut clip);
        assert!(esc.starts_with("\x1b]52;c;"));
        assert_eq!(clip.register(), "hello");

        // Cut the same selection.
        b.cut(&mut clip);
        assert_eq!(b.text(), " world");

        // Paste it back at start.
        b.set_head(0, false);
        b.paste(&clip);
        assert_eq!(b.text(), "hello world");
    }

    #[test]
    fn test_undo_redo() {
        let mut b = TextBuffer::new();
        b.insert("foo");
        b.insert("bar");
        assert_eq!(b.text(), "foobar");
        assert!(b.undo());
        assert_eq!(b.text(), "foo");
        assert!(b.undo());
        assert_eq!(b.text(), "");
        assert!(b.redo(0));
        assert_eq!(b.text(), "foo");
    }

    #[test]
    fn test_vertical_motion_and_line_col() {
        let mut b = TextBuffer::from_text("abc\ndefg\nhi");
        b.set_head(0, false);
        b.move_down(false);
        assert_eq!(b.cursor_line_col(), (1, 0));
        b.move_end(false);
        assert_eq!(b.cursor_line_col(), (1, 4)); // end of "defg"
        b.move_down(false); // col clamps to "hi" length (2)
        assert_eq!(b.cursor_line_col(), (2, 2));
    }

    #[test]
    fn test_multibyte_backspace_no_panic() {
        let mut b = TextBuffer::from_text("héllo"); // 'é' is 2 bytes
        b.move_end(false);
        for _ in 0..5 {
            b.backspace();
        }
        assert_eq!(b.text(), "");
    }
}
