//! `CollaborativeText` — an editing model backed by the convergent [`TextCrdt`].
//!
//! Bridges the local editing UX (a cursor + selection + clipboard) to the CRDT wire
//! protocol: local edits return [`CrdtOp`]s to broadcast, remote ops integrate
//! convergently. Cursor/selection are tracked in *visible character* indices.

use tenui_core::clipboard::Clipboard;

use crate::crdt::{CrdtOp, TextCrdt};

/// A single-selection collaborative text editor over a [`TextCrdt`].
#[derive(Clone, Debug)]
pub struct CollaborativeText {
    doc: TextCrdt,
    /// Selection anchor and head, in visible-character indices.
    anchor: usize,
    head: usize,
}

impl CollaborativeText {
    /// Creates an empty collaborative document for replica `site` (must be unique).
    pub fn new(site: u32) -> Self {
        Self {
            doc: TextCrdt::new(site),
            anchor: 0,
            head: 0,
        }
    }

    pub fn text(&self) -> String {
        self.doc.text()
    }

    pub fn len(&self) -> usize {
        self.doc.len()
    }

    pub fn is_empty(&self) -> bool {
        self.doc.is_empty()
    }

    pub fn cursor(&self) -> usize {
        self.head
    }

    /// The selection as a sorted `(start, end)` visible-index range.
    pub fn selection(&self) -> (usize, usize) {
        (self.anchor.min(self.head), self.anchor.max(self.head))
    }

    pub fn set_cursor(&mut self, pos: usize) {
        let p = pos.min(self.doc.len());
        self.anchor = p;
        self.head = p;
    }

    fn set_head(&mut self, pos: usize, extend: bool) {
        let p = pos.min(self.doc.len());
        self.head = p;
        if !extend {
            self.anchor = p;
        }
    }

    pub fn move_left(&mut self, extend: bool) {
        let h = self.head.saturating_sub(1);
        self.set_head(h, extend);
    }

    pub fn move_right(&mut self, extend: bool) {
        let h = (self.head + 1).min(self.doc.len());
        self.set_head(h, extend);
    }

    /// Deletes the current selection (if any), returning the ops produced.
    fn delete_selection(&mut self) -> Vec<CrdtOp> {
        let (start, end) = self.selection();
        let mut ops = Vec::new();
        for _ in start..end {
            if let Some(op) = self.doc.local_delete(start) {
                ops.push(op);
            }
        }
        self.set_cursor(start);
        ops
    }

    /// Inserts `s` at the cursor (replacing any selection), returning ops to broadcast.
    #[must_use = "broadcast the returned CrdtOps so peers converge"]
    pub fn insert(&mut self, s: &str) -> Vec<CrdtOp> {
        let mut ops = self.delete_selection();
        let at = self.head;
        ops.extend(self.doc.local_insert_str(at, s));
        self.set_cursor(at + s.chars().count());
        ops
    }

    /// Deletes the selection, or the character before the cursor.
    #[must_use = "broadcast the returned CrdtOps so peers converge"]
    pub fn backspace(&mut self) -> Vec<CrdtOp> {
        if self.selection().0 != self.selection().1 {
            return self.delete_selection();
        }
        if self.head == 0 {
            return Vec::new();
        }
        let at = self.head - 1;
        let ops = self.doc.local_delete(at).into_iter().collect();
        self.set_cursor(at);
        ops
    }

    /// Integrates a remote operation (idempotent, commutative, order-independent).
    pub fn apply_remote(&mut self, op: CrdtOp) {
        self.doc.apply(op);
        // Keep the cursor in range as remote edits change length.
        let len = self.doc.len();
        self.anchor = self.anchor.min(len);
        self.head = self.head.min(len);
    }

    /// The selected substring (visible characters).
    fn selected_text(&self) -> String {
        let (start, end) = self.selection();
        self.text().chars().skip(start).take(end - start).collect()
    }

    /// Copies the selection into `clip`; returns the OSC 52 escape to emit.
    #[must_use = "emit the returned OSC 52 escape to update the system clipboard"]
    pub fn copy(&self, clip: &mut Clipboard) -> String {
        clip.copy(&self.selected_text())
    }

    /// Cuts the selection into `clip`; returns the escape and the CRDT ops to broadcast.
    pub fn cut(&mut self, clip: &mut Clipboard) -> (String, Vec<CrdtOp>) {
        let escape = clip.copy(&self.selected_text());
        let ops = self.delete_selection();
        (escape, ops)
    }

    /// Pastes `clip`'s register at the cursor (replacing any selection).
    #[must_use = "broadcast the returned CrdtOps so peers converge"]
    pub fn paste(&mut self, clip: &Clipboard) -> Vec<CrdtOp> {
        let text = clip.paste().to_string();
        self.insert(&text)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_local_edit_and_clipboard() {
        let mut clip = Clipboard::new();
        let mut ed = CollaborativeText::new(1);
        let _ = ed.insert("hello world");
        assert_eq!(ed.text(), "hello world");

        // Select "world" and cut.
        ed.set_cursor(6);
        for _ in 0..5 {
            ed.move_right(true);
        }
        let (esc, _ops) = ed.cut(&mut clip);
        assert!(esc.starts_with("\x1b]52;c;"));
        assert_eq!(ed.text(), "hello ");
        assert_eq!(clip.register(), "world");

        // Paste it back.
        let _ = ed.paste(&clip);
        assert_eq!(ed.text(), "hello world");
    }

    #[test]
    fn test_two_replicas_converge() {
        let mut clip = Clipboard::new();
        let _ = &mut clip;
        let mut alice = CollaborativeText::new(1);
        let mut bob = CollaborativeText::new(2);

        let base = alice.insert("Hi");
        for op in &base {
            bob.apply_remote(op.clone());
        }
        assert_eq!(bob.text(), "Hi");

        // Concurrent inserts at the same spot, cross-delivered in different orders.
        alice.set_cursor(1);
        bob.set_cursor(1);
        let a_ops = alice.insert("AAA");
        let b_ops = bob.insert("bbb");
        for op in &b_ops {
            alice.apply_remote(op.clone());
        }
        for op in a_ops.iter().rev() {
            bob.apply_remote(op.clone());
        }

        assert_eq!(alice.text(), bob.text(), "replicas must converge");
        assert_eq!(alice.len(), 8); // Hi + AAA + bbb
    }
}
