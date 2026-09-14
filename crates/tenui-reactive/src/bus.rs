use std::{cell::RefCell, collections::HashSet};

use crate::invalidation::{Invalidation, InvalidationSink};

/// Coordinates the Two-Lane Invalidation pipeline.
#[derive(Debug, Default)]
pub struct TwoLaneBus {
    layout_dirty: RefCell<HashSet<u64>>,
    paint_dirty: RefCell<HashSet<u64>>,
}

impl TwoLaneBus {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn has_layout_dirty(&self) -> bool {
        !self.layout_dirty.borrow().is_empty()
    }

    pub fn has_paint_dirty(&self) -> bool {
        !self.paint_dirty.borrow().is_empty()
    }

    pub fn is_clean(&self) -> bool {
        !self.has_layout_dirty() && !self.has_paint_dirty()
    }

    pub fn drain_layout_dirty(&self) -> Vec<u64> {
        let mut set = self.layout_dirty.borrow_mut();
        let list: Vec<u64> = set.drain().collect();
        list
    }

    pub fn drain_paint_dirty(&self) -> Vec<u64> {
        let mut set = self.paint_dirty.borrow_mut();
        let list: Vec<u64> = set.drain().collect();
        list
    }

    pub fn clear(&self) {
        self.layout_dirty.borrow_mut().clear();
        self.paint_dirty.borrow_mut().clear();
    }
}

impl InvalidationSink for TwoLaneBus {
    fn mark_dirty(&self, node_id: u64, flags: Invalidation) {
        if flags.contains(Invalidation::LAYOUT) {
            self.layout_dirty.borrow_mut().insert(node_id);
            self.paint_dirty.borrow_mut().insert(node_id);
        } else if flags.contains(Invalidation::PAINT) {
            self.paint_dirty.borrow_mut().insert(node_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_bus_starts_clean() {
        let bus = TwoLaneBus::new();
        assert!(bus.is_clean());
        assert!(!bus.has_layout_dirty());
        assert!(!bus.has_paint_dirty());
    }

    #[test]
    fn paint_only_goes_to_paint_set() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::PAINT);
        assert!(!bus.has_layout_dirty());
        assert!(bus.has_paint_dirty());
    }

    #[test]
    fn layout_goes_to_both_sets() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::LAYOUT);
        assert!(bus.has_layout_dirty());
        assert!(bus.has_paint_dirty());
    }

    #[test]
    fn drain_layout_empties_set() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::LAYOUT);
        let drained = bus.drain_layout_dirty();
        assert_eq!(drained, vec![1]);
        assert!(!bus.has_layout_dirty());
        assert!(bus.has_paint_dirty(), "paint not drained by drain_layout");
    }

    #[test]
    fn drain_paint_empties_set() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::PAINT);
        let drained = bus.drain_paint_dirty();
        assert_eq!(drained, vec![1]);
        assert!(!bus.has_paint_dirty());
    }

    #[test]
    fn clear_resets_both() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::LAYOUT);
        bus.mark_dirty(2, Invalidation::PAINT);
        bus.clear();
        assert!(bus.is_clean());
    }

    #[test]
    fn dedup_same_node() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(5, Invalidation::PAINT);
        bus.mark_dirty(5, Invalidation::PAINT);
        let drained = bus.drain_paint_dirty();
        assert_eq!(drained.len(), 1);
        assert_eq!(drained[0], 5);
    }

    #[test]
    fn multiple_nodes_tracked() {
        let bus = TwoLaneBus::new();
        bus.mark_dirty(1, Invalidation::PAINT);
        bus.mark_dirty(2, Invalidation::LAYOUT);
        bus.mark_dirty(3, Invalidation::PAINT);
        let mut paint = bus.drain_paint_dirty();
        paint.sort();
        assert_eq!(paint, vec![1, 2, 3]);
        let layout = bus.drain_layout_dirty();
        assert_eq!(layout, vec![2]);
    }
}
