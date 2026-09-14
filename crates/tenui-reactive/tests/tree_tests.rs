use std::{cell::RefCell, rc::Rc};

use taffy::prelude::*;
use tenui_core::{Buffer, Color, Modifier};
use tenui_reactive::{Invalidation, InvalidationSink, ReactiveTree, TwoLaneBus};

#[test]
fn test_reactive_tree_phase1_bypass_on_paint_mutation() {
    let bus = Rc::new(TwoLaneBus::new());
    let mut tree = ReactiveTree::new(Rc::clone(&bus));

    let counter = Rc::new(RefCell::new(0));
    let counter_clone = Rc::clone(&counter);

    // Root container
    tree.insert(1, None, Style::default(), |_| {});

    // Child leaf node
    tree.insert(2, Some(1), Style::default(), move |subview| {
        let val = *counter_clone.borrow();
        subview.set_string(
            0,
            0,
            &format!("Val: {}", val),
            Color::Green,
            Color::Reset,
            Modifier::empty(),
        );
    });

    let mut buf = Buffer::new(20, 5);

    // Frame 1: Initial layout and paint
    tree.render_frame(&mut buf);
    assert_eq!(tree.layout_invocations, 1, "Initial frame must compute layout");

    // Mutate state cosmetically: only mark PAINT lane dirty
    *counter.borrow_mut() = 1;
    bus.mark_dirty(2, Invalidation::PAINT);

    // Frame 2: Render frame with only Paint Lane dirty
    tree.render_frame(&mut buf);

    // Taffy layout MUST BE COMPLETELY BYPASSED (O(0) layout cost)
    assert_eq!(
        tree.layout_invocations, 1,
        "Phase 1 layout must be skipped for cosmetic paint mutations!"
    );

    // Mutate state geometrically: mark LAYOUT lane dirty
    bus.mark_dirty(2, Invalidation::LAYOUT);

    // Frame 3: Render frame with Layout Lane dirty
    tree.render_frame(&mut buf);
    assert_eq!(
        tree.layout_invocations, 2,
        "Phase 1 layout must run when layout is dirty"
    );
}

#[test]
fn test_lca_calculation() {
    let bus = Rc::new(TwoLaneBus::new());
    let mut tree = ReactiveTree::new(Rc::clone(&bus));

    // Hierarchy:
    // Root (1)
    //   ├── Col A (2)
    //   │     ├── Leaf A1 (3)
    //   │     └── Leaf A2 (4)
    //   └── Col B (5)
    tree.insert(1, None, Style::default(), |_| {});
    tree.insert(2, Some(1), Style::default(), |_| {});
    tree.insert(3, Some(2), Style::default(), |_| {});
    tree.insert(4, Some(2), Style::default(), |_| {});
    tree.insert(5, Some(1), Style::default(), |_| {});

    // Sibling nodes 3 and 4 -> LCA should be 2 (Col A)
    let lca = tree.find_lca(&[3, 4]);
    assert_eq!(lca, Some(2));

    // Nodes 3 and 5 in different branches -> LCA should be 1 (Root)
    let lca_cross = tree.find_lca(&[3, 5]);
    assert_eq!(lca_cross, Some(1));
}

#[test]
fn test_invalidate_intersecting() {
    use tenui_core::Rect;

    let bus = Rc::new(TwoLaneBus::new());
    let mut tree = ReactiveTree::new(Rc::clone(&bus));

    tree.insert(1, None, Style::default(), |_| {});
    tree.insert(
        2,
        Some(1),
        Style {
            size: Size {
                width: Dimension::length(10.0),
                height: Dimension::length(5.0),
            },
            ..Default::default()
        },
        |_| {},
    );

    let mut buf = Buffer::new(30, 10);
    tree.render_frame(&mut buf);

    // Clear all dirty sets
    bus.drain_paint_dirty();
    bus.drain_layout_dirty();
    assert!(!bus.has_paint_dirty());

    // Invalidate intersecting rect over node 2 (bounds 0,0 to 10,5)
    let modal_rect = Rect::new(5, 2, 10, 5);
    tree.invalidate_intersecting(&modal_rect, Invalidation::PAINT);

    assert!(bus.has_paint_dirty());
    let dirty_paint = bus.drain_paint_dirty();
    assert!(dirty_paint.contains(&2), "Intersecting node 2 must be marked dirty");

    // Invalidate non-intersecting rect
    let distant_rect = Rect::new(40, 20, 5, 2);
    tree.invalidate_intersecting(&distant_rect, Invalidation::PAINT);
    assert!(!bus.has_paint_dirty(), "Disjoint rect must not mark node dirty");
}
