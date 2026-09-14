//! Selection-aware clipboard operations for text buffers.
//!
//! These bridge a [`Cursor`] selection and a [`Clipboard`] into [`TextEditDelta`]s, so
//! copy/cut/paste compose with the existing [`BranchingUndoTree`](crate::BranchingUndoTree):
//! feed the returned delta to `commit_edit` and apply it with [`apply_delta`].
//!
//! Cursor offsets are byte offsets; every offset is snapped to a UTF-8 char boundary
//! before slicing so multi-byte content can never panic.

use tenui_core::clipboard::Clipboard;

use crate::multi_cursor::{Cursor, TextEditDelta};

/// Largest char boundary `<= i`, clamped to `s.len()`.
fn floor_boundary(s: &str, i: usize) -> usize {
    let mut i = i.min(s.len());
    while i > 0 && !s.is_char_boundary(i) {
        i -= 1;
    }
    i
}

/// The selection's byte range within `text`, snapped to char boundaries.
fn selection_range(text: &str, cursor: &Cursor) -> (usize, usize) {
    let start = floor_boundary(text, cursor.start());
    let end = floor_boundary(text, cursor.end()).max(start);
    (start, end)
}

/// Copies the cursor's selection into `clip`'s register and returns the OSC 52 escape to
/// emit to the terminal. Returns an empty string (and touches nothing) when the cursor
/// has no selection.
#[must_use = "emit the returned OSC 52 escape to update the system clipboard"]
pub fn copy_selection(clip: &mut Clipboard, text: &str, cursor: &Cursor) -> String {
    if !cursor.is_selection() {
        return String::new();
    }
    let (start, end) = selection_range(text, cursor);
    clip.copy(&text[start..end])
}

/// Cuts the cursor's selection: copies it into `clip` and returns the [`TextEditDelta`]
/// that removes it, together with the OSC 52 escape to emit. Returns `None` when the
/// cursor has no selection.
#[must_use = "apply the returned delta and emit the escape; cut_selection does not mutate text itself"]
pub fn cut_selection(clip: &mut Clipboard, text: &str, cursor: &Cursor) -> Option<(TextEditDelta, String)> {
    if !cursor.is_selection() {
        return None;
    }
    let (start, end) = selection_range(text, cursor);
    let removed = text[start..end].to_string();
    let escape = clip.copy(&removed);
    let delta = TextEditDelta {
        range: (start, end),
        replaced_text: removed,
        inserted_text: String::new(),
    };
    Some((delta, escape))
}

/// Produces the [`TextEditDelta`] for pasting `clip`'s register at the cursor: it replaces
/// the selection if there is one, otherwise inserts at the cursor head.
#[must_use = "apply the returned TextEditDelta to the buffer; paste itself does not mutate text"]
pub fn paste(clip: &Clipboard, text: &str, cursor: &Cursor) -> TextEditDelta {
    let (start, end) = if cursor.is_selection() {
        selection_range(text, cursor)
    } else {
        let p = floor_boundary(text, cursor.head);
        (p, p)
    };
    let replaced = text[start..end].to_string();
    TextEditDelta {
        range: (start, end),
        replaced_text: replaced,
        inserted_text: clip.paste().to_string(),
    }
}

/// Applies a [`TextEditDelta`] to `text` in place.
///
/// The delta's `range` is assumed to sit on char boundaries (as produced by the helpers
/// above); it is re-snapped defensively so an externally built delta can't panic.
pub fn apply_delta(text: &mut String, delta: &TextEditDelta) {
    let start = floor_boundary(text, delta.range.0);
    let end = floor_boundary(text, delta.range.1).max(start);
    text.replace_range(start..end, &delta.inserted_text);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_copy_selection() {
        let mut clip = Clipboard::new();
        let text = "hello world";
        let cursor = Cursor { anchor: 0, head: 5 };
        let esc = copy_selection(&mut clip, text, &cursor);
        assert_eq!(clip.register(), "hello");
        assert!(esc.starts_with("\x1b]52;c;"));
    }

    #[test]
    fn test_copy_no_selection_is_noop() {
        let mut clip = Clipboard::new();
        clip.set_register("keep");
        let esc = copy_selection(&mut clip, "abc", &Cursor::point(1));
        assert!(esc.is_empty());
        assert_eq!(clip.register(), "keep");
    }

    #[test]
    fn test_cut_then_apply() {
        let mut clip = Clipboard::new();
        let mut text = "hello world".to_string();
        let cursor = Cursor { anchor: 0, head: 6 }; // "hello "
        let (delta, _esc) = cut_selection(&mut clip, &text, &cursor).unwrap();
        assert_eq!(clip.register(), "hello ");
        apply_delta(&mut text, &delta);
        assert_eq!(text, "world");
    }

    #[test]
    fn test_paste_at_cursor() {
        let mut clip = Clipboard::new();
        clip.set_register("XYZ");
        let mut text = "ab".to_string();
        let delta = paste(&clip, &text, &Cursor::point(1));
        apply_delta(&mut text, &delta);
        assert_eq!(text, "aXYZb");
    }

    #[test]
    fn test_paste_replaces_selection() {
        let mut clip = Clipboard::new();
        clip.set_register("Z");
        let mut text = "abcd".to_string();
        let delta = paste(&clip, &text, &Cursor { anchor: 1, head: 3 }); // replace "bc"
        apply_delta(&mut text, &delta);
        assert_eq!(text, "aZd");
    }

    #[test]
    fn test_multibyte_cut_no_panic() {
        let mut clip = Clipboard::new();
        let text = "héllo"; // 'é' spans bytes 1..3
        // Selection end lands inside 'é' — must snap, not panic.
        let cursor = Cursor { anchor: 0, head: 2 };
        let (delta, _) = cut_selection(&mut clip, text, &cursor).unwrap();
        // Snapped to byte 1 -> only "h" is cut.
        assert_eq!(clip.register(), "h");
        assert_eq!(delta.range, (0, 1));
    }
}
