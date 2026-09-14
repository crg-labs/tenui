//! A first-class, direction-aware selection over a linear index space.
//!
//! Selection previously lived only as `tenui-text::Cursor`; promoting it to core lets
//! the mouse dispatcher (drag-select), the compositor, and the text widgets share one type.
//! Indices are opaque `usize`s (byte offsets, cell offsets, or item indices, per caller).

/// An anchored selection: `anchor` is where it started, `head` is the moving end.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Selection {
    pub anchor: usize,
    pub head: usize,
}

impl Selection {
    /// A collapsed (empty) selection at `pos`.
    pub fn point(pos: usize) -> Self {
        Self { anchor: pos, head: pos }
    }

    /// A selection spanning `[anchor, head]` (order-independent for range queries).
    pub fn new(anchor: usize, head: usize) -> Self {
        Self { anchor, head }
    }

    /// The lower bound of the selection.
    pub fn start(&self) -> usize {
        self.anchor.min(self.head)
    }

    /// The upper bound of the selection.
    pub fn end(&self) -> usize {
        self.anchor.max(self.head)
    }

    /// `(start, end)` as an ordered range.
    pub fn range(&self) -> (usize, usize) {
        (self.start(), self.end())
    }

    pub fn is_empty(&self) -> bool {
        self.anchor == self.head
    }

    /// Number of indices covered.
    pub fn len(&self) -> usize {
        self.end() - self.start()
    }

    /// Whether the selection is anchored before the head (forward) vs after (backward).
    pub fn is_forward(&self) -> bool {
        self.head >= self.anchor
    }

    /// Whether `index` falls within `[start, end)`.
    pub fn contains(&self, index: usize) -> bool {
        index >= self.start() && index < self.end()
    }

    /// Moves the head, keeping the anchor (extends the selection).
    pub fn extend_to(&mut self, head: usize) {
        self.head = head;
    }

    /// Collapses the selection to a point at `pos`.
    pub fn collapse_to(&mut self, pos: usize) {
        self.anchor = pos;
        self.head = pos;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_range_and_direction() {
        let fwd = Selection::new(2, 6);
        assert_eq!(fwd.range(), (2, 6));
        assert!(fwd.is_forward());
        assert_eq!(fwd.len(), 4);

        let back = Selection::new(6, 2);
        assert_eq!(back.range(), (2, 6)); // order-independent
        assert!(!back.is_forward());
    }

    #[test]
    fn test_empty_and_contains() {
        let p = Selection::point(3);
        assert!(p.is_empty());
        assert!(!p.contains(3));

        let s = Selection::new(1, 4);
        assert!(s.contains(1));
        assert!(s.contains(3));
        assert!(!s.contains(4)); // half-open
    }

    #[test]
    fn test_extend_and_collapse() {
        let mut s = Selection::point(5);
        s.extend_to(9);
        assert_eq!(s.range(), (5, 9));
        s.collapse_to(2);
        assert!(s.is_empty());
        assert_eq!(s.head, 2);
    }
}
