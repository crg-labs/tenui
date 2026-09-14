use std::rc::Rc;

use tenui_reactive::{Invalidation, InvalidationSink, ReactiveTextNode, Signal, TwoLaneBus};

#[test]
fn test_adaptive_text_grapheme_width_promotion() {
    let bus = Rc::new(TwoLaneBus::new());
    let node = ReactiveTextNode::new(42, "CPU: 04%".to_string(), Rc::clone(&bus));

    assert_eq!(node.width(), 8);
    assert!(bus.is_clean());

    // Invariant visual width change: "CPU: 04%" -> "CPU: 12%"
    node.update_text("CPU: 12%".to_string());
    assert_eq!(node.width(), 8);

    // Must be strictly in Paint Lane: Layout Lane must NOT be dirty
    assert!(!bus.has_layout_dirty(), "Layout must not be dirty for invariant width");
    assert!(bus.has_paint_dirty(), "Paint lane must be dirty");

    let paint_dirty = bus.drain_paint_dirty();
    assert_eq!(paint_dirty, vec![42]);
    assert!(bus.is_clean());

    // Variant visual width change: "OK" (width 2) -> "FAILED" (width 6)
    let node2 = ReactiveTextNode::new(99, "OK".to_string(), Rc::clone(&bus));
    assert_eq!(node2.width(), 2);

    node2.update_text("FAILED".to_string());
    assert_eq!(node2.width(), 6);

    // Must be promoted to Layout Lane
    assert!(bus.has_layout_dirty(), "Layout lane must be dirty for width change");
    assert!(bus.has_paint_dirty(), "Paint lane must be dirty for width change");

    assert_eq!(bus.drain_layout_dirty(), vec![99]);
    assert_eq!(bus.drain_paint_dirty(), vec![99]);
    assert!(bus.is_clean());
}

#[test]
fn test_signal_invalidation_binding() {
    let bus = Rc::new(TwoLaneBus::new());
    let sig = Signal::new(100);

    sig.bind_invalidation(7, Invalidation::PAINT, Rc::clone(&bus) as Rc<dyn InvalidationSink>);

    sig.set(100); // Same value, should not notify
    assert!(bus.is_clean());

    sig.set(101); // Changed value, should mark paint dirty
    assert!(!bus.has_layout_dirty());
    assert!(bus.has_paint_dirty());
    assert_eq!(bus.drain_paint_dirty(), vec![7]);
}

#[test]
fn test_transactional_signal_batching() {
    use std::cell::RefCell;

    use tenui_reactive::{ReactiveContext, batch};

    let bus = Rc::new(TwoLaneBus::new());
    let sig_a = Signal::new(10);
    let sig_b = Signal::new(20);
    let sig_c = Signal::new(30);

    sig_a.bind_invalidation(1, Invalidation::PAINT, Rc::clone(&bus) as Rc<dyn InvalidationSink>);
    sig_b.bind_invalidation(2, Invalidation::LAYOUT, Rc::clone(&bus) as Rc<dyn InvalidationSink>);
    sig_c.bind_invalidation(3, Invalidation::PAINT, Rc::clone(&bus) as Rc<dyn InvalidationSink>);

    let notify_count = Rc::new(RefCell::new(0));
    let nc = Rc::clone(&notify_count);
    sig_a.subscribe(move || {
        *nc.borrow_mut() += 1;
    });

    let cx = ReactiveContext::new();

    // Execute atomic batch via cx.batch
    cx.batch(|| {
        sig_a.set(11);
        sig_b.set(21);
        sig_c.set(31);

        // During batch execution, mutations are queued and not yet dispatched
        assert!(bus.is_clean());
        assert_eq!(*notify_count.borrow(), 0);
    });

    // Also verify standalone batch function
    batch(|| {
        sig_a.set(12);
    });

    // After batch completion, all invalidations and notifications are flushed together
    assert!(bus.has_layout_dirty());
    assert!(bus.has_paint_dirty());
    assert_eq!(*notify_count.borrow(), 2);

    let layout_dirty = bus.drain_layout_dirty();
    assert_eq!(layout_dirty, vec![2]);

    let mut paint_dirty = bus.drain_paint_dirty();
    paint_dirty.sort();
    assert_eq!(paint_dirty, vec![1, 2, 3]);
}
