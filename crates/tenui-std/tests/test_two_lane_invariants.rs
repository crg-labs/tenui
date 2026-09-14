use std::{cell::RefCell, rc::Rc};

use tenui_core::{Buffer, Color, Modifier};
use tenui_layout::Style;
use tenui_reactive::{Invalidation, InvalidationSink, ReactiveTextNode, ReactiveTree, TwoLaneBus};
use tenui_std::harness::TenuiTestHarness;

#[test]
fn test_two_lane_pipeline_paint_and_layout_invariants() {
    let bus = Rc::new(TwoLaneBus::new());
    let mut tree = ReactiveTree::new(bus.clone());
    let mut buffer = Buffer::new(80, 24);

    // Shared label state wrapped in ReactiveTextNode
    let label_node = Rc::new(ReactiveTextNode::new(2, "Ready".to_string(), bus.clone()));

    // Cosmetic style state (e.g. border color)
    let border_color = Rc::new(RefCell::new(Color::Blue));

    let child_style = Style {
        size: tenui_layout::Size {
            width: tenui_layout::Dimension::length(40.0),
            height: tenui_layout::Dimension::length(2.0),
        },
        ..Default::default()
    };

    // Root container (Node 1)
    tree.insert(1, None, Style::default(), |_| {});

    // Child text label (Node 2)
    let label_clone = label_node.clone();
    tree.insert(2, Some(1), child_style.clone(), move |canvas| {
        canvas.set_string(0, 0, &label_clone.text(), Color::White, Color::Reset, Modifier::empty());
    });

    // Child border panel (Node 3)
    let border_clone = border_color.clone();
    tree.insert(3, Some(1), child_style, move |canvas| {
        canvas.set_string(0, 1, "━━━━", *border_clone.borrow(), Color::Reset, Modifier::empty());
    });

    // Initial Frame: Both layout and paint are resolved
    tree.render_frame(&mut buffer);
    assert_eq!(tree.layout_invocations, 1, "Initial frame requires layout resolution");
    assert!(tree.paint_invocations >= 1, "Initial frame requires paint dispatch");

    let initial_layouts = tree.layout_invocations;
    let initial_paints = tree.paint_invocations;

    // Step 1: Same-Width Text Mutation ("Ready" -> "Error", width 5 -> 5)
    // Adaptive Text Grapheme Width Promotion keeps invariant width in Paint Lane
    label_node.update_text("Error".to_string());

    assert!(
        !bus.has_layout_dirty(),
        "Same-width mutation must NOT mark layout dirty"
    );
    assert!(bus.has_paint_dirty(), "Same-width mutation MUST mark paint dirty");

    tree.render_frame(&mut buffer);

    assert_eq!(
        tree.layout_invocations, initial_layouts,
        "Layout invocations must not increment for same-width text mutation (Paint Lane bypass)"
    );
    assert!(
        tree.paint_invocations > initial_paints,
        "Paint invocations must increment for updated text"
    );

    let step1_layouts = tree.layout_invocations;
    let step1_paints = tree.paint_invocations;

    // Step 2: Cosmetic / Style Mutation (Border Color Blue -> Red)
    *border_color.borrow_mut() = Color::Red;
    bus.mark_dirty(3, Invalidation::PAINT);

    assert!(
        !bus.has_layout_dirty(),
        "Cosmetic style mutation must NOT mark layout dirty"
    );
    assert!(bus.has_paint_dirty(), "Cosmetic style mutation MUST mark paint dirty");

    tree.render_frame(&mut buffer);

    assert_eq!(
        tree.layout_invocations, step1_layouts,
        "Layout invocations must not increment for cosmetic style change"
    );
    assert!(
        tree.paint_invocations > step1_paints,
        "Paint invocations must increment for repainted border"
    );

    let step2_layouts = tree.layout_invocations;
    let step2_paints = tree.paint_invocations;

    // Step 3: Variant-Width Text Mutation ("Error" -> "Long Status Warning", 5 -> 19)
    // Changing visual width promotes mutation to Layout Lane
    label_node.update_text("Long Status Warning".to_string());

    assert!(bus.has_layout_dirty(), "Variant-width mutation MUST mark layout dirty");
    assert!(bus.has_paint_dirty(), "Variant-width mutation MUST mark paint dirty");

    tree.render_frame(&mut buffer);

    assert_eq!(
        tree.layout_invocations,
        step2_layouts + 1,
        "Layout invocations MUST increment by 1 when text width alters layout geometry"
    );
    assert!(
        tree.paint_invocations > step2_paints,
        "Paint invocations must increment after layout reflow"
    );

    // Step 4: Verification with TenuiTestHarness Golden Snapshot
    let mut harness = TenuiTestHarness::headless(80, 24);
    harness.draw_buffer_with_bg(Color::Rgb(30, 30, 46), |b| {
        b.cells.clone_from(&buffer.cells);
    });

    harness.assert_text(0, 0, "Long Status Warning");
    harness.assert_golden_snapshot("suite_3_two_lane_invariants");
}
