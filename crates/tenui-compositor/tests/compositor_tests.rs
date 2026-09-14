use std::{cell::Cell, rc::Rc};

use tenui_compositor::{
    BrailleCanvas, Compositor, Direction, FocusNode, HalfBlockCanvas, QuadrantCanvas, SpatialNav, cut_rect,
};
use tenui_core::{Buffer, Cell as TerminalCell, Color, Rect};

#[test]
fn test_coordinate_cut_rect_decomposition() {
    let subject = Rect::new(0, 0, 10, 10); // Area = 100
    let cutter = Rect::new(2, 2, 4, 4); // Area = 16, overlap = 16

    let fragments = cut_rect(subject, cutter);
    assert_eq!(fragments.len(), 4);

    let total_area: u32 = fragments.iter().map(|r| r.area()).sum();
    assert_eq!(total_area, 100 - 16);

    // Verify non-overlapping
    for i in 0..fragments.len() {
        for j in (i + 1)..fragments.len() {
            assert!(
                !fragments[i].intersects(&fragments[j]),
                "Fragments {} and {} must not intersect",
                i,
                j
            );
        }
    }
}

#[test]
fn test_compositor_occlusion_culling_and_fragment_blitting() {
    let mut compositor = Compositor::new();
    let lower_ran = Rc::new(Cell::new(false));
    let lower_ran_clone = Rc::clone(&lower_ran);

    // Layer 1: Background at (0, 0, 8, 8) with z_index 0
    compositor.add_layer(Rect::new(0, 0, 8, 8), 0, move |subview| {
        lower_ran_clone.set(true);
        let mut cell = TerminalCell::from_char('A');
        cell.set_fg(Color::Red);
        subview.fill(cell);
    });

    // Layer 2: Modal at (2, 2, 4, 4) with z_index 10
    compositor.add_layer(Rect::new(2, 2, 4, 4), 10, |subview| {
        let mut cell = TerminalCell::from_char('B');
        cell.set_fg(Color::Blue);
        subview.fill(cell);
    });

    let mut buf = Buffer::new(8, 8);
    compositor.composite(&mut buf);

    assert!(lower_ran.get(), "Lower layer ran");

    // Outside modal: should be 'A'
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "A");
    assert_eq!(buf.get(7, 7).unwrap().symbol.as_str(), "A");

    // Inside modal: should be 'B'
    assert_eq!(buf.get(2, 2).unwrap().symbol.as_str(), "B");
    assert_eq!(buf.get(5, 5).unwrap().symbol.as_str(), "B");
}

#[test]
fn test_compositor_completely_obscured_layer_dropped() {
    let mut compositor = Compositor::new();
    let hidden_ran = Rc::new(Cell::new(false));
    let hidden_ran_clone = Rc::clone(&hidden_ran);

    // Layer 1 at (2, 2, 4, 4), z_index 0
    compositor.add_layer(Rect::new(2, 2, 4, 4), 0, move |_| {
        hidden_ran_clone.set(true);
    });

    // Layer 2 covering (0, 0, 10, 10), z_index 5
    compositor.add_layer(Rect::new(0, 0, 10, 10), 5, |subview| {
        subview.fill(TerminalCell::from_char('X'));
    });

    let mut buf = Buffer::new(10, 10);
    compositor.composite(&mut buf);

    // Layer 1 is completely obscured by Layer 2, so its paint must be dropped completely
    assert!(!hidden_ran.get(), "Completely obscured layer paint must be dropped");
}

#[test]
fn test_quadrant_subpixel_canvas() {
    let mut canvas = QuadrantCanvas::new(2, 2);
    // Subpixels in cell (0, 0): set upper-left (0, 0) and lower-right (1, 1)
    canvas.set_pixel(0, 0);
    canvas.set_pixel(1, 1);

    let mut buf = Buffer::new(2, 2);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);
    canvas.render_to_subview(&mut subview, Color::Yellow, Color::Reset);

    // In 2x2 quadrant: upper-left + lower-right is '▚' (U+259A)
    let cell = buf.get(0, 0).unwrap();
    assert_eq!(cell.symbol.as_str(), "▚");
}

#[test]
fn test_aspect_ratio_weighted_spatial_navigation() {
    let nav = SpatialNav::new(); // k_aspect = 2.0

    // Node 1 at (0, 0)
    let n1 = FocusNode::new(1, Rect::new(0, 0, 4, 1));
    // Node 2 horizontally right at (10, 0)
    let n2 = FocusNode::new(2, Rect::new(10, 0, 4, 1));
    // Node 3 diagonally down-right at (10, 3).
    // Because cells are tall (k=2.0), dy=3 is effectively 6 units, so n2 is closer!
    let n3 = FocusNode::new(3, Rect::new(10, 3, 4, 1));

    let nodes = vec![n1, n2, n3];

    let next = nav.navigate(1, Direction::Right, &nodes, None);
    assert_eq!(next, Some(2));
}

#[test]
fn test_focus_scope_barrier() {
    let nav = SpatialNav::new();

    let n1 = FocusNode::new(1, Rect::new(0, 0, 4, 1)).with_scope(10);
    let n2 = FocusNode::new(2, Rect::new(5, 0, 4, 1)).with_scope(10);
    let n3 = FocusNode::new(3, Rect::new(10, 0, 4, 1)); // Background node (no scope)

    let nodes = vec![n1, n2, n3];

    // Navigating inside modal scope 10 cannot leak to background node 3
    let next = nav.navigate(2, Direction::Right, &nodes, Some(10));
    assert_eq!(next, None, "Must not escape modal scope barrier");
}

#[test]
fn test_halfblock_subpixel_rendering() {
    let mut canvas = HalfBlockCanvas::new(2, 1);
    canvas.set_pixel(0, 0, Color::Red);
    canvas.set_pixel(0, 1, Color::Blue);

    let mut buf = Buffer::new(2, 1);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);
    canvas.render_to_subview(&mut subview);

    let cell = buf.get(0, 0).unwrap();
    assert_eq!(cell.symbol.as_str(), "▀");
    assert_eq!(cell.fg, Color::Red);
    assert_eq!(cell.bg, Color::Blue);
}

#[test]
fn test_braille_bresenham_line() {
    let mut canvas = BrailleCanvas::new(2, 2);
    canvas.draw_line(0, 0, 0, 3);

    let mut buf = Buffer::new(2, 2);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);
    canvas.render_to_subview(&mut subview, Color::Green, Color::Reset);

    let cell = buf.get(0, 0).unwrap();
    let expected = char::from_u32(0x2800 + 0x47).unwrap();
    assert_eq!(cell.symbol.as_str(), &expected.to_string());
}

#[test]
fn test_mirrored_spatial_navigation_rtl() {
    let nodes = vec![
        FocusNode::new(1, Rect::new(0, 0, 5, 1)),
        FocusNode::new(2, Rect::new(10, 0, 5, 1)),
    ];

    let ltr_nav = SpatialNav::new();
    assert_eq!(ltr_nav.navigate(1, Direction::Right, &nodes, None), Some(2));
    assert_eq!(ltr_nav.navigate(2, Direction::Left, &nodes, None), Some(1));

    // With RTL enabled, horizontal navigation direction mirrors across y-axis
    let rtl_nav = SpatialNav::new().with_rtl(true);
    assert_eq!(rtl_nav.navigate(1, Direction::Left, &nodes, None), Some(2));
    assert_eq!(rtl_nav.navigate(2, Direction::Right, &nodes, None), Some(1));
}

#[test]
fn test_layer_compositor_dismiss_modal_invalidation() {
    use tenui_compositor::{LayerCompositor, ModalHandle};

    let mut comp = LayerCompositor::new();
    comp.register_node(1, Rect::new(0, 0, 40, 20)); // Background panel
    comp.register_node(2, Rect::new(5, 5, 20, 10)); // Underlying widget 1
    comp.register_node(3, Rect::new(50, 50, 10, 10)); // Disjoint widget

    comp.set_active_modal(ModalHandle::new(100, Rect::new(10, 5, 30, 8)));

    let mut dirtied = Vec::new();
    comp.dismiss_modal(&mut |id| dirtied.push(id));

    assert!(dirtied.contains(&1), "Intersecting node 1 must be dirtied");
    assert!(dirtied.contains(&2), "Intersecting node 2 must be dirtied");
    assert!(!dirtied.contains(&3), "Disjoint node 3 must not be dirtied");
}

#[test]
fn test_spatial_navigation_split_pane() {
    // Left pane has an item at (0, 5, 20, 2), right pane has an item at (25, 4, 20, 2)
    let nodes = vec![
        FocusNode::new(1, Rect::new(0, 5, 20, 2)),
        FocusNode::new(2, Rect::new(25, 4, 20, 2)),
    ];

    let nav = SpatialNav::new();
    // Navigating Right from node 1 should seamlessly reach node 2 across split boundary
    assert_eq!(nav.navigate(1, Direction::Right, &nodes, None), Some(2));
    // Navigating Left from node 2 should return to node 1
    assert_eq!(nav.navigate(2, Direction::Left, &nodes, None), Some(1));
}
