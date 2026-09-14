//! Click-and-drag text selection across a rendered document.
//!
//! Terminal text selection is fundamentally over the *rendered* grid, not the source: what the
//! user drags across is wrapped, prefixed, box-drawn output. [`DocumentSelection`] models a
//! selection as an anchor/active pair of `(row, column)` points in that rendered grid, where
//! `row` indexes the document's visual rows and `column` is a display column. Give it the
//! rendered rows (as [`SelectableRow`]s) and it will test membership per cell
//! ([`contains`](DocumentSelection::contains)), paint the highlight
//! ([`highlight`](DocumentSelection::highlight)), and extract the selected text with correct
//! multi-row and wide-character handling ([`extract`](DocumentSelection::extract)).
//!
//! It is widget-agnostic: [`MarkdownView`](crate::doc::MarkdownView) and
//! [`ChatThread`](crate::chat_thread::ChatThread) both drive one, and any widget that can
//! report its rendered rows can too.

use tenui_core::{CanvasSubviewMut, Color, Modifier};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// One rendered visual row that can take part in selection: the display column its text begins
/// at, and the row's text. Implemented for `(u16, String)` and `(u16, &str)` so widgets can
/// hand over rows without a wrapper type.
pub trait SelectableRow {
    /// The display column where this row's text starts (0 for a left-flush row; higher when the
    /// content is inset behind a prefix, gutter, or bubble border).
    fn x_offset(&self) -> u16;
    /// The row's rendered text (without the inset — `x_offset` accounts for that).
    fn row_text(&self) -> &str;
}

impl SelectableRow for (u16, String) {
    fn x_offset(&self) -> u16 {
        self.0
    }
    fn row_text(&self) -> &str {
        &self.1
    }
}

impl SelectableRow for (u16, &str) {
    fn x_offset(&self) -> u16 {
        self.0
    }
    fn row_text(&self) -> &str {
        self.1
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord)]
struct Point {
    row: usize,
    col: u16,
}

/// A click-and-drag selection over a document's rendered visual rows. Coordinates are
/// `(visual row index, display column)`; columns are inclusive on both ends.
#[derive(Clone, Debug, Default)]
pub struct DocumentSelection {
    anchor: Point,
    active: Point,
    dragging: bool,
    present: bool,
}

impl DocumentSelection {
    /// An empty selection.
    pub fn new() -> Self {
        Self::default()
    }

    /// Begins a drag at `(row, col)` (mouse press): sets both ends to this point and enters the
    /// dragging state. A selection that is begun but never [`extend`](Self::extend)ed stays
    /// empty.
    pub fn begin(&mut self, row: usize, col: u16) {
        let p = Point { row, col };
        self.anchor = p;
        self.active = p;
        self.dragging = true;
        self.present = true;
    }

    /// Moves the active end to `(row, col)` while dragging (mouse move). No-op if not dragging.
    pub fn extend(&mut self, row: usize, col: u16) {
        if self.dragging {
            self.active = Point { row, col };
        }
    }

    /// Ends the drag (mouse release). The selection remains for extraction/highlight until
    /// [`clear`](Self::clear)ed.
    pub fn finish(&mut self) {
        self.dragging = false;
    }

    /// Discards the selection entirely.
    pub fn clear(&mut self) {
        *self = Self::default();
    }

    /// Whether a drag is currently in progress.
    pub fn is_dragging(&self) -> bool {
        self.dragging
    }

    /// Whether there is nothing selected (never started, or start == end).
    pub fn is_empty(&self) -> bool {
        !self.present || self.anchor == self.active
    }

    /// The selection as sorted `((start_row, start_col), (end_row, end_col))`, or `None` when
    /// empty. `start <= end` in reading order.
    pub fn normalized(&self) -> Option<((usize, u16), (usize, u16))> {
        if self.is_empty() {
            return None;
        }
        let (a, b) = if self.anchor <= self.active {
            (self.anchor, self.active)
        } else {
            (self.active, self.anchor)
        };
        Some(((a.row, a.col), (b.row, b.col)))
    }

    /// Whether visual row `row` has any selected cell.
    pub fn contains_row(&self, row: usize) -> bool {
        match self.normalized() {
            Some(((sr, _), (er, _))) => row >= sr && row <= er,
            None => false,
        }
    }

    /// Whether the cell at `(row, col)` is selected (columns inclusive).
    pub fn contains(&self, row: usize, col: u16) -> bool {
        let Some(((sr, sc), (er, ec))) = self.normalized() else {
            return false;
        };
        if row < sr || row > er {
            return false;
        }
        if sr == er {
            col >= sc && col <= ec
        } else if row == sr {
            col >= sc
        } else if row == er {
            col <= ec
        } else {
            true
        }
    }

    /// Extracts the selected text from the document's `rows` (indexed by visual row), joining
    /// rows with `\n`. Handles the partial first/last rows, full interior rows, and each row's
    /// `x_offset` inset; wide characters are sliced by display column.
    pub fn extract<R: SelectableRow>(&self, rows: &[R]) -> String {
        let Some(((sr, sc), (er, ec))) = self.normalized() else {
            return String::new();
        };
        let mut lines: Vec<String> = Vec::new();
        for r in sr..=er {
            let Some(row) = rows.get(r) else { continue };
            let text = row.row_text();
            let off = row.x_offset();
            if text.is_empty() {
                if sr != er {
                    lines.push(String::new());
                }
                continue;
            }
            let (start_col, end_col) = if sr == er {
                (sc, ec)
            } else if r == sr {
                (sc, u16::MAX)
            } else if r == er {
                (0, ec)
            } else {
                (0, u16::MAX)
            };
            let rel_start = start_col.saturating_sub(off) as usize;
            let rel_end = if end_col == u16::MAX {
                usize::MAX
            } else {
                end_col.saturating_sub(off) as usize
            };
            lines.push(slice_by_cols(text, rel_start, rel_end));
        }
        lines.join("\n").trim_end().to_string()
    }

    /// Paints the selection highlight onto `surface`: for each on-screen row, inverts the
    /// selected cells to `primary` background / `on_primary` foreground. `first_row` is the
    /// document visual-row index drawn at screen `y = 0` (the scroll offset), and `rows` are the
    /// full document rows (so `rows[first_row + y]` is the row on screen line `y`).
    pub fn highlight<R: SelectableRow>(
        &self,
        surface: &mut CanvasSubviewMut<'_>,
        first_row: usize,
        rows: &[R],
        primary: Color,
        on_primary: Color,
    ) {
        if self.is_empty() {
            return;
        }
        for y in 0..surface.height() {
            let doc_row = first_row + y as usize;
            if !self.contains_row(doc_row) {
                continue;
            }
            let Some(row) = rows.get(doc_row) else { break };
            let x0 = row.x_offset();
            let width = UnicodeWidthStr::width(row.row_text()) as u16;
            for col in x0..x0.saturating_add(width) {
                if self.contains(doc_row, col)
                    && let Some(cell) = surface.get_mut(col, y)
                {
                    cell.bg = primary;
                    cell.fg = on_primary;
                    cell.modifier.insert(Modifier::BOLD);
                }
            }
        }
    }
}

/// Returns the substring of `text` spanning display columns `[start_col, end_col]` (inclusive).
/// A wide glyph is included when any of its cells fall within the range. Returns empty when
/// `start_col > end_col`.
pub fn slice_by_cols(text: &str, start_col: usize, end_col: usize) -> String {
    if start_col > end_col {
        return String::new();
    }
    let mut col = 0usize;
    let mut out = String::new();
    for ch in text.chars() {
        let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
        // Include the glyph if its cell span [col, col+cw) overlaps [start_col, end_col].
        if col + cw > start_col && col <= end_col {
            out.push(ch);
        }
        col += cw;
        if col > end_col {
            break;
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rows() -> Vec<(u16, String)> {
        vec![
            (0, "hello world".to_string()),
            (0, "second line".to_string()),
            (0, "third line here".to_string()),
        ]
    }

    #[test]
    fn empty_until_extended() {
        let mut s = DocumentSelection::new();
        assert!(s.is_empty());
        s.begin(0, 2);
        assert!(s.is_empty(), "a bare click selects nothing");
        s.extend(0, 6);
        assert!(!s.is_empty());
        assert!(s.is_dragging());
        s.finish();
        assert!(!s.is_dragging());
    }

    #[test]
    fn single_row_extract_and_contains() {
        let mut s = DocumentSelection::new();
        s.begin(0, 0);
        s.extend(0, 4); // "hello"
        assert_eq!(s.extract(&rows()), "hello");
        assert!(s.contains(0, 3));
        assert!(!s.contains(0, 5));
        assert!(!s.contains(1, 0));
    }

    #[test]
    fn reversed_drag_normalizes() {
        let mut s = DocumentSelection::new();
        s.begin(0, 6); // drag right-to-left
        s.extend(0, 0);
        assert_eq!(s.extract(&rows()), "hello w");
        assert_eq!(s.normalized().unwrap().0, (0, 0));
    }

    #[test]
    fn multi_row_extract_spans_partial_ends() {
        let mut s = DocumentSelection::new();
        s.begin(0, 6); // from "world"
        s.extend(2, 4); // to "third"
        // row0: cols 6.. -> "world"; row1 full -> "second line"; row2: cols ..=4 -> "third"
        assert_eq!(s.extract(&rows()), "world\nsecond line\nthird");
        assert!(s.contains_row(1));
        assert!(s.contains(1, 0));
        assert!(s.contains(1, 99), "interior rows select to the end");
    }

    #[test]
    fn x_offset_inset_is_honored() {
        // A bubble line whose text starts at column 4.
        let rows = vec![(4u16, "inset text".to_string())];
        let mut s = DocumentSelection::new();
        s.begin(0, 4);
        s.extend(0, 8); // "inset"
        assert_eq!(s.extract(&rows), "inset");
    }

    #[test]
    fn slice_by_cols_wide_chars() {
        // "a世b": 'a'=1 col (0), '世'=2 cols (1..3), 'b'=1 col (3).
        assert_eq!(slice_by_cols("a世b", 0, 0), "a");
        assert_eq!(slice_by_cols("a世b", 1, 2), "世");
        assert_eq!(slice_by_cols("a世b", 0, 3), "a世b");
        assert_eq!(slice_by_cols("abc", 5, 2), "");
    }

    #[test]
    fn highlight_inverts_selected_cells() {
        use tenui_core::{Buffer, Rect};
        let mut s = DocumentSelection::new();
        s.begin(0, 0);
        s.extend(0, 4); // "hello"
        let rs = rows();
        let mut buf = Buffer::new(11, 3);
        {
            let mut sv = buf.subview_mut(Rect::new(0, 0, 11, 3));
            // Draw the row text first so cells exist.
            sv.write_str_clipped(0, 0, "hello world", Color::White, Color::Reset);
            s.highlight(&mut sv, 0, &rs, Color::Blue, Color::Black);
        }
        assert_eq!(buf.get(0, 0).unwrap().bg, Color::Blue);
        assert_eq!(buf.get(4, 0).unwrap().bg, Color::Blue);
        assert_ne!(buf.get(6, 0).unwrap().bg, Color::Blue, "past selection end");
    }
}
