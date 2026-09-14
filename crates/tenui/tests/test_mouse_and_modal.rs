use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};

use tenui_core::{
    Buffer, Color, MouseButton, MouseDispatcher, MouseEvent, MouseTargetEntry, NodeId, Point, Rect, SgrMouseParser,
};
use tenui_std::Modal;

#[test]
fn test_mouse_click_dispatches_strictly_to_top_z_index_node() {
    let mut dispatcher = MouseDispatcher::new();
    let bottom_clicked = Arc::new(AtomicBool::new(false));
    let top_clicked = Arc::new(AtomicBool::new(false));

    let b_flag = bottom_clicked.clone();
    dispatcher.register_target(
        MouseTargetEntry::new(NodeId::new(1), Rect::new(10, 5, 20, 10))
            .with_z_index(0)
            .with_on_click(move |_| b_flag.store(true, Ordering::SeqCst)),
    );

    let t_flag = top_clicked.clone();
    dispatcher.register_target(
        MouseTargetEntry::new(NodeId::new(2), Rect::new(15, 5, 20, 10))
            .with_z_index(10) // Higher Z-Index overlaps column 15..30
            .with_on_click(move |_| t_flag.store(true, Ordering::SeqCst)),
    );

    // Click inside the overlapping region (x = 18, y = 7)
    let pt = Point::new(18, 7);
    dispatcher.dispatch(MouseEvent::Down {
        button: MouseButton::Left,
        position: pt,
    });
    dispatcher.dispatch(MouseEvent::Up {
        button: MouseButton::Left,
        position: pt,
    });

    // INVARIANT: Only the top layer must receive the click
    assert!(
        !bottom_clicked.load(Ordering::SeqCst),
        "Obscured bottom node received mouse click"
    );
    assert!(
        top_clicked.load(Ordering::SeqCst),
        "Top z-index node failed to intercept mouse click"
    );
}

#[test]
fn test_modal_auto_width_preserves_full_scene_text() {
    let mut buf = Buffer::new(100, 30);

    let modal = Modal::new("OPTICAL ACRYLIC LENS", 50, 16, 999)
        .with_transparent(true)
        .with_auto_width(0.85);

    let bounds = modal.bounds(buf.rect());
    // Auto-width bounded by 85% of 100 cols expands modal width to 85 columns
    assert!(
        bounds.width >= 70,
        "Modal auto-width must expand to accommodate wide content, got {}",
        bounds.width
    );

    let mut subview = buf.subview_mut(bounds);
    modal.render(&mut subview, |inner| {
        Modal::render_key_value(
            inner,
            4,
            10,
            "Underlying Scene:  ",
            "Volumetrics & Kinetic Spinners Visible",
            Color::White,
            Color::Green,
        );
    });

    // Extract rendered content row from the buffer (bounds.y + 1 for border, + 10 for y inside client)
    let mut line_str = String::new();
    let y = bounds.y + 1 + 10;
    for x in 0..100 {
        if let Some(cell) = buf.get(x, y) {
            line_str.push_str(cell.symbol.as_str());
        }
    }

    assert!(
        line_str.contains("Spinners Visible"),
        "Modal text truncated; expected 'Visible', found line: {}",
        line_str
    );
    assert!(
        !line_str.contains("Visib│"),
        "Text abuts right border frame without breathing margin: {}",
        line_str
    );
}

#[test]
fn test_sgr_mouse_parser() {
    // 1. Left button down at (20, 10) (1-based -> 19, 9)
    let ev1 = SgrMouseParser::parse_sgr("\x1b[<0;20;10M");
    assert_eq!(
        ev1,
        Some(MouseEvent::Down {
            button: MouseButton::Left,
            position: Point::new(19, 9),
        })
    );

    // 2. Left button release at (20, 10)
    let ev2 = SgrMouseParser::parse_sgr("\x1b[<0;20;10m");
    assert_eq!(
        ev2,
        Some(MouseEvent::Up {
            button: MouseButton::Left,
            position: Point::new(19, 9),
        })
    );

    // 3. Mouse move at (35, 15)
    let ev3 = SgrMouseParser::parse_sgr("\x1b[<32;35;15M");
    assert_eq!(
        ev3,
        Some(MouseEvent::Move {
            position: Point::new(34, 14),
        })
    );

    // 4. Wheel up (64)
    let ev4 = SgrMouseParser::parse_sgr("\x1b[<64;10;10M");
    assert_eq!(
        ev4,
        Some(MouseEvent::Scroll {
            delta_x: 0.0,
            delta_y: -1.0,
            is_pixel: false,
        })
    );
}

#[test]
fn test_click_vs_drag_threshold() {
    let mut dispatcher = MouseDispatcher::new();
    let click_fired = Arc::new(AtomicBool::new(false));
    let drag_fired = Arc::new(AtomicBool::new(false));

    let c_flag = click_fired.clone();
    let d_flag = drag_fired.clone();
    dispatcher.register_target(
        MouseTargetEntry::new(NodeId::new(10), Rect::new(0, 0, 40, 20))
            .with_on_click(move |_| c_flag.store(true, Ordering::SeqCst))
            .with_on_drag(move |_, _| d_flag.store(true, Ordering::SeqCst)),
    );

    // Click: small travel (distance 1.0 <= 1.5)
    dispatcher.dispatch(MouseEvent::Down {
        button: MouseButton::Left,
        position: Point::new(5, 5),
    });
    dispatcher.dispatch(MouseEvent::Move {
        position: Point::new(6, 5),
    });
    dispatcher.dispatch(MouseEvent::Up {
        button: MouseButton::Left,
        position: Point::new(6, 5),
    });

    assert!(
        click_fired.load(Ordering::SeqCst),
        "Click should synthesize when travel <= 1.5 cells"
    );
    assert!(
        !drag_fired.load(Ordering::SeqCst),
        "Drag should not synthesize when travel <= 1.5 cells"
    );

    // Reset and test Drag (distance > 1.5)
    click_fired.store(false, Ordering::SeqCst);
    drag_fired.store(false, Ordering::SeqCst);

    dispatcher.dispatch(MouseEvent::Down {
        button: MouseButton::Left,
        position: Point::new(5, 5),
    });
    dispatcher.dispatch(MouseEvent::Move {
        position: Point::new(15, 5),
    });
    assert!(
        drag_fired.load(Ordering::SeqCst),
        "Drag should synthesize when movement exceeds 1.5 cells"
    );
    dispatcher.dispatch(MouseEvent::Up {
        button: MouseButton::Left,
        position: Point::new(15, 5),
    });
    assert!(
        !click_fired.load(Ordering::SeqCst),
        "Click should not synthesize when travel > 1.5 cells"
    );
}

#[test]
fn test_modal_barrier_and_outside_click() {
    let mut dispatcher = MouseDispatcher::new();
    let outside_clicked = Arc::new(AtomicBool::new(false));
    let underlying_clicked = Arc::new(AtomicBool::new(false));

    // Underlying target at (5, 5)
    let u_flag = underlying_clicked.clone();
    dispatcher.register_target(
        MouseTargetEntry::new(NodeId::new(1), Rect::new(0, 0, 30, 20))
            .with_on_click(move |_| u_flag.store(true, Ordering::SeqCst)),
    );

    // Active modal at (40, 5, 40, 20)
    dispatcher.set_modal_barrier(NodeId::new(99), Rect::new(40, 5, 40, 20));
    let o_flag = outside_clicked.clone();
    dispatcher.set_on_outside_click(move |_| o_flag.store(true, Ordering::SeqCst));

    // Click outside modal at (10, 10)
    dispatcher.dispatch(MouseEvent::Down {
        button: MouseButton::Left,
        position: Point::new(10, 10),
    });
    dispatcher.dispatch(MouseEvent::Up {
        button: MouseButton::Left,
        position: Point::new(10, 10),
    });

    assert!(
        outside_clicked.load(Ordering::SeqCst),
        "Outside click callback should fire when clicking outside modal barrier"
    );
    assert!(
        !underlying_clicked.load(Ordering::SeqCst),
        "Underlying targets must be suppressed when clicking outside active modal"
    );
}

#[test]
fn test_hover_enter_leave_transition() {
    let mut dispatcher = MouseDispatcher::new();
    let enter_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let leave_count = Arc::new(std::sync::atomic::AtomicUsize::new(0));

    let e_flag = enter_count.clone();
    let l_flag = leave_count.clone();
    dispatcher.register_target(
        MouseTargetEntry::new(NodeId::new(1), Rect::new(10, 5, 10, 5))
            .with_on_enter(move || {
                e_flag.fetch_add(1, Ordering::SeqCst);
            })
            .with_on_leave(move || {
                l_flag.fetch_add(1, Ordering::SeqCst);
            }),
    );

    // Move into target
    dispatcher.dispatch(MouseEvent::Move {
        position: Point::new(12, 6),
    });
    assert_eq!(enter_count.load(Ordering::SeqCst), 1);
    assert_eq!(leave_count.load(Ordering::SeqCst), 0);
    assert!(dispatcher.is_hovered(NodeId::new(1)));

    // Move inside target
    dispatcher.dispatch(MouseEvent::Move {
        position: Point::new(14, 7),
    });
    assert_eq!(enter_count.load(Ordering::SeqCst), 1);
    assert_eq!(leave_count.load(Ordering::SeqCst), 0);

    // Move outside target
    dispatcher.dispatch(MouseEvent::Move {
        position: Point::new(30, 20),
    });
    assert_eq!(enter_count.load(Ordering::SeqCst), 1);
    assert_eq!(leave_count.load(Ordering::SeqCst), 1);
    assert!(!dispatcher.is_hovered(NodeId::new(1)));
}

#[test]
fn test_adaptive_motion_gate() {
    let mut dispatcher = MouseDispatcher::new();
    assert!(
        !dispatcher.needs_any_event_tracking(),
        "Default dispatcher should not require 1003h any-event tracking"
    );

    // Register a target with enter listener
    dispatcher.register_target(MouseTargetEntry::new(NodeId::new(1), Rect::new(0, 0, 10, 10)).with_on_enter(|| {}));
    assert!(
        dispatcher.needs_any_event_tracking(),
        "Dispatcher with hover listeners must promote tracking to any-event"
    );

    dispatcher.clear_targets();
    assert!(
        !dispatcher.needs_any_event_tracking(),
        "Clearing hover targets must restore low-overhead mode"
    );
}
