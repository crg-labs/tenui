use std::{
    cell::{Cell, RefCell},
    rc::Rc,
};

use unicode_width::UnicodeWidthStr;

use crate::invalidation::{Invalidation, InvalidationSink};

/// A text node that performs Adaptive Text Grapheme Width Promotion.
///
/// Invariant visual width mutations remain strictly confined to the Paint Lane.
/// Changes that alter character cell column count promote the mutation to the Layout Lane.
pub struct ReactiveTextNode<S> {
    pub node_id: u64,
    pub content: RefCell<String>,
    pub visual_width: Cell<usize>,
    pub sink: Rc<S>,
}

impl<S: InvalidationSink> ReactiveTextNode<S> {
    pub fn new(node_id: u64, initial: String, sink: Rc<S>) -> Self {
        let visual_width = UnicodeWidthStr::width(initial.as_str());
        Self {
            node_id,
            content: RefCell::new(initial),
            visual_width: Cell::new(visual_width),
            sink,
        }
    }

    pub fn text(&self) -> String {
        self.content.borrow().clone()
    }

    pub fn width(&self) -> usize {
        self.visual_width.get()
    }

    /// Updates text content and routes invalidation to the appropriate lane.
    pub fn update_text(&self, next: String) {
        let next_width = UnicodeWidthStr::width(next.as_str());
        let prev_width = self.visual_width.get();

        *self.content.borrow_mut() = next;

        if next_width == prev_width {
            // Invariant width: strictly Paint Lane (0 Taffy reflow overhead)
            self.sink.mark_dirty(self.node_id, Invalidation::PAINT);
        } else {
            // Variant width: promote to Layout Lane (Subtree reflow required)
            self.visual_width.set(next_width);
            self.sink.mark_dirty(self.node_id, Invalidation::BOTH);
        }
    }
}
