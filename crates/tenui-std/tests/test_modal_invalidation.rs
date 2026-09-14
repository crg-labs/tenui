use std::rc::Rc;

use tenui_compositor::{LayerCompositor, ModalHandle};
use tenui_core::{Buffer, Color, Modifier, Rect};
use tenui_layout::{BorderStyle, Style, StyleExt};
use tenui_reactive::{Invalidation, ReactiveTree, TwoLaneBus};
use tenui_std::harness::TenuiTestHarness;

#[test]
fn test_modal_mount_dismissal_screen_recovery() {
    let mut harness = TenuiTestHarness::headless(100, 40);

    let bg_color = Color::Rgb(30, 30, 46);
    let surface_color = Color::Rgb(49, 50, 68);

    let render_base_layout = |h: &mut TenuiTestHarness| {
        h.draw_layout_with_bg(bg_color, |ui| {
            ui.column(Style::default(), |col| {
                // Header
                col.leaf(Style::default().height_cells(1.0), |canvas| {
                    canvas.set_string(
                        2,
                        0,
                        "TERMINUS COCKPIT OVERVIEW",
                        Color::White,
                        Color::Reset,
                        Modifier::BOLD,
                    );
                });
                // 3 Column body filling 100x37
                col.container(Style::default().grow(1.0).flex_row(), |body| {
                    body.block(
                        Style::default().grow(1.0),
                        Some("Left Panel"),
                        BorderStyle::ROUNDED,
                        Color::Cyan,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Left Item Row {:02} [Data]", y),
                                        Color::White,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                    body.block(
                        Style::default().grow(1.0),
                        Some("Center Panel (Target)"),
                        BorderStyle::ROUNDED,
                        Color::Yellow,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Center Item Row {:02} [Occluded by modal]", y),
                                        Color::Yellow,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                    body.block(
                        Style::default().grow(1.0),
                        Some("Right Panel"),
                        BorderStyle::ROUNDED,
                        Color::Green,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Right Item Row {:02} [Data]", y),
                                        Color::White,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                });
                // Footer
                col.leaf(Style::default().height_cells(1.0), |canvas| {
                    canvas.set_string(
                        2,
                        0,
                        "Status: Operational | WCAG AA: PASS",
                        Color::Green,
                        Color::Reset,
                        Modifier::empty(),
                    );
                });
            });
        });
    };

    // 1. Initial pre-modal frame
    render_base_layout(&mut harness);
    // Draw twice to populate double buffer cleanly
    render_base_layout(&mut harness);

    let pre_modal_screen = harness.dump_screen();
    let pre_modal_front = harness.terminal().front().clone();

    // Verify center panel has expected pre-modal text
    assert!(pre_modal_screen.contains("Center Item Row 05"));

    // 2. Mount Modal Overlay (40x10 centered: x=30, y=15, w=40, h=10)
    let modal_rect = Rect::new(30, 15, 40, 10);
    let mut modal_input = String::from("find symbol");

    let render_modal_frame = |h: &mut TenuiTestHarness, query: &str| {
        h.draw_buffer_with_bg(bg_color, |back| {
            // Render underlying base layout
            let mut ui = tenui_layout::Ui::new();
            ui.column(Style::default(), |col| {
                col.leaf(Style::default().height_cells(1.0), |canvas| {
                    canvas.set_string(
                        2,
                        0,
                        "TERMINUS COCKPIT OVERVIEW",
                        Color::White,
                        Color::Reset,
                        Modifier::BOLD,
                    );
                });
                col.container(Style::default().grow(1.0).flex_row(), |body| {
                    body.block(
                        Style::default().grow(1.0),
                        Some("Left Panel"),
                        BorderStyle::ROUNDED,
                        Color::Cyan,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Left Item Row {:02} [Data]", y),
                                        Color::White,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                    body.block(
                        Style::default().grow(1.0),
                        Some("Center Panel (Target)"),
                        BorderStyle::ROUNDED,
                        Color::Yellow,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Center Item Row {:02} [Occluded by modal]", y),
                                        Color::Yellow,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                    body.block(
                        Style::default().grow(1.0),
                        Some("Right Panel"),
                        BorderStyle::ROUNDED,
                        Color::Green,
                        surface_color,
                        |b| {
                            b.leaf(Style::default().grow(1.0), |canvas| {
                                for y in 0..canvas.height() {
                                    canvas.set_string(
                                        1,
                                        y,
                                        &format!("Right Item Row {:02} [Data]", y),
                                        Color::White,
                                        Color::Reset,
                                        Modifier::empty(),
                                    );
                                }
                            });
                        },
                    );
                });
                col.leaf(Style::default().height_cells(1.0), |canvas| {
                    canvas.set_string(
                        2,
                        0,
                        "Status: Operational | WCAG AA: PASS",
                        Color::Green,
                        Color::Reset,
                        Modifier::empty(),
                    );
                });
            });
            ui.render_to_buffer(back);

            // Composite Modal onto back subview
            let mut mview = back.subview_mut(modal_rect);
            tenui_layout::draw_border(&mut mview, BorderStyle::DOUBLE, Color::Magenta, Color::Black);
            mview.set_string(2, 0, "COMMAND PALETTE", Color::White, Color::Black, Modifier::BOLD);
            mview.set_string(
                2,
                2,
                &format!("> {}█", query),
                Color::Cyan,
                Color::Black,
                Modifier::empty(),
            );
            mview.set_string(2, 4, "[1] Action One", Color::White, Color::Black, Modifier::empty());
            mview.set_string(2, 5, "[2] Action Two", Color::White, Color::Black, Modifier::empty());
        });
    };

    render_modal_frame(&mut harness, &modal_input);

    // Assert modal is visible and occluding the center panel
    harness.assert_text(32, 15, "COMMAND PALETTE");
    harness.assert_text(32, 17, "> find symbol");

    // Interactive input typing in modal
    modal_input.push_str(" --all");
    render_modal_frame(&mut harness, &modal_input);
    harness.assert_text(32, 17, "> find symbol --all");

    // 3. Dismiss Modal Overlay
    render_base_layout(&mut harness);
    // Draw second idle frame so back and front synchronize
    render_base_layout(&mut harness);

    // 4. Invariant Assertions:
    // Every single cell in the buffer matches pre-modal content and styles exactly!
    assert_eq!(
        harness.terminal().front(),
        &pre_modal_front,
        "Screen after modal dismissal does not match exact pre-modal buffer state!"
    );

    let post_modal_screen = harness.dump_screen();
    assert_eq!(
        post_modal_screen, pre_modal_screen,
        "Post-modal screen dump diverged from pre-modal screen dump!"
    );

    // 5. Test Reactive Invalidation Graph on Modal Dismissal
    let bus = Rc::new(TwoLaneBus::new());
    let mut tree = ReactiveTree::new(bus.clone());

    // Insert 3 nodes:
    // Node 1: Left area Rect(0, 0, 20, 20) — Does NOT intersect modal Rect(30, 15, 40, 10)
    // Node 2: Center area Rect(25, 10, 50, 20) — INTERSECTS modal Rect(30, 15, 40, 10)
    // Node 3: Bottom area Rect(0, 30, 100, 10) — Does NOT intersect modal
    tree.insert(1, None, Style::default(), |_| {});
    tree.insert(2, None, Style::default(), |_| {});
    tree.insert(3, None, Style::default(), |_| {});

    let mut dummy_buf = Buffer::new(100, 40);
    tree.render_frame(&mut dummy_buf);

    // Manually set component bounding boxes
    if let Some(c) = tree.components.get_mut(&1) {
        c.bounding_box = Rect::new(0, 0, 20, 20);
    }
    if let Some(c) = tree.components.get_mut(&2) {
        c.bounding_box = Rect::new(25, 10, 50, 20);
    }
    if let Some(c) = tree.components.get_mut(&3) {
        c.bounding_box = Rect::new(0, 30, 100, 10);
    }

    // Clear dirty flags
    let _ = bus.drain_paint_dirty();
    let _ = bus.drain_layout_dirty();
    assert!(!bus.has_paint_dirty());

    // Dismiss modal and invalidate intersecting nodes
    tree.invalidate_intersecting(&modal_rect, Invalidation::PAINT);
    assert!(
        bus.has_paint_dirty(),
        "Bus must have paint dirty nodes after modal dismissal"
    );

    let dirty_paint = bus.drain_paint_dirty();
    assert_eq!(dirty_paint, vec![2], "Node 2 was marked dirty in the invalidation bus");

    // 6. Test LayerCompositor dismissal hook
    let mut compositor = LayerCompositor::new();
    compositor.register_node(1, Rect::new(0, 0, 20, 20));
    compositor.register_node(2, Rect::new(25, 10, 50, 20));
    compositor.register_node(3, Rect::new(0, 30, 100, 10));

    let modal_handle = ModalHandle::new(101, modal_rect);
    compositor.set_active_modal(modal_handle);

    let mut dismissed_dirty = Vec::new();
    let mut sink = |id: u64| dismissed_dirty.push(id);
    compositor.dismiss_modal(&mut sink);

    assert_eq!(
        dismissed_dirty,
        vec![2],
        "LayerCompositor marked node 2 dirty on dismissal"
    );

    // Save golden snapshot
    harness.assert_golden_snapshot("suite_2_modal_recovery");
}
