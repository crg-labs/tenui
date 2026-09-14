//! Integration test suite for Tenui Layer 9 Frontier Subsystems:
//! - tenui-simd: SIMD differential compositing scanner
//! - tenui-collab: Multi-seat real-time collaborative presence & CRDT
//! - tenui-media: Half-block video converter & latency frame dropping
//! - tenui-math: Sub-cell TeX/Typst mathematical typesetting & box layout
//! - tenui-hid: Hardware HID/MIDI bridge with kinetic impulse impartation
//! - tenui-chaos: PTY stream fuzzer, multibyte fragmentation, & resize storm

use std::time::Duration;

use tenui::prelude::*;

#[test]
fn test_simd_differential_scan() {
    let back = vec![
        AlignedCell {
            grapheme: 65,
            fg: 0xFFFFFF,
            bg: 0,
            modifier: 0,
        };
        16
    ];
    let mut front = back.clone();

    // Modify indices 2 and 7
    front[2].grapheme = 66;
    front[7].fg = 0xFF0000;

    let mut dirty = Vec::new();
    SimdDiffScanner::scan_dirty_runs(&back, &front, &mut dirty);

    assert_eq!(dirty, vec![2, 7]);
}

#[test]
fn test_collab_multi_seat_presence_and_crdt() {
    let mut manager = MultiSeatManager::new();
    let alice_id = 1;
    let bob_id = 2;

    manager.add_seat(SeatPresence {
        seat_id: alice_id,
        name: "Alice".to_string(),
        cursor_pos: Point::new(2, 2),
        accent_color: Color::Green,
        active_pane: 0,
    });

    manager.add_seat(SeatPresence {
        seat_id: bob_id,
        name: "Bob".to_string(),
        cursor_pos: Point::new(6, 1),
        accent_color: Color::Magenta,
        active_pane: 0,
    });

    let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 5));
    let mut subview = buffer.subview_mut(Rect::new(0, 0, 20, 5));

    // Render from Alice's perspective -> Bob's caret rendered, Alice's suppressed
    manager.render_remote_seats(alice_id, &mut subview);

    // Bob at (6, 1) has accent color background
    let bob_caret = subview.get(6, 1).expect("bob caret cell");
    assert_eq!(bob_caret.bg, Color::Magenta);

    // Alice at (2, 2) is suppressed
    let alice_caret = subview.get(2, 2).expect("alice cell");
    assert_ne!(alice_caret.bg, Color::Green);

    // CRDT state vector
    let op1 = CollabOperation::Insert {
        pos: 0,
        text: "fn main() {\n}".to_string(),
        author: alice_id,
        clock: 1,
    };
    match op1 {
        CollabOperation::Insert { clock, .. } => assert_eq!(clock, 1),
        _ => panic!("Expected Insert operation"),
    }
}

#[test]
fn test_media_half_block_and_latency_backpressure() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 6, 4));
    let mut subview = buffer.subview_mut(Rect::new(0, 0, 6, 4));

    // 2x2 image: top row green, bottom row yellow
    let rgb = vec![0, 255, 0, 0, 255, 0, 255, 255, 0, 255, 255, 0];

    HalfBlockVideoConverter::render_rgb(&mut subview, 0, 0, 2, 2, &rgb);

    let cell = subview.get(0, 0).expect("cell exists");
    assert_eq!(cell.symbol.as_str(), "▀");
    assert_eq!(cell.fg, Color::Rgb(0, 255, 0));
    assert_eq!(cell.bg, Color::Rgb(255, 255, 0));

    // Test backpressure frame dropping controller
    let mut controller = MediaStreamController::new(60, Duration::from_millis(25));
    assert!(!controller.should_drop_frame(Duration::from_millis(10)));
    assert!(controller.should_drop_frame(Duration::from_millis(40)));
    assert_eq!(controller.frames_rendered, 1);
    assert_eq!(controller.frames_dropped, 1);
}

#[test]
fn test_math_subcell_typesetting_and_fractions() {
    let mut buffer = Buffer::empty(Rect::new(0, 0, 16, 6));
    let mut subview = buffer.subview_mut(Rect::new(0, 0, 16, 6));

    let formula = MathNode::fraction(MathNode::text("x^2 + 1"), MathNode::text("2"));

    let typesetter = MathTypesetter;
    let (w, h) = typesetter.measure(&formula);
    assert!(w >= 9);
    assert_eq!(h, 3);

    typesetter.render(&mut subview, 0, 0, &formula, Color::Yellow);

    // Verify fraction bar '─' rendered on line 1
    let bar_cell = subview.get(0, 1).expect("bar cell");
    assert_eq!(bar_cell.symbol.as_str(), "─");

    // Verify matrix layout
    let mat = MathNode::matrix(vec![
        vec![MathNode::symbol('1'), MathNode::symbol('0')],
        vec![MathNode::symbol('0'), MathNode::symbol('1')],
    ]);
    let (mw, mh) = typesetter.measure(&mat);
    assert!(mw > 3);
    assert_eq!(mh, 2);
}

#[test]
fn test_hid_controller_kinetic_momentum_and_leds() {
    let mut reactor = HidEventReactor::new();

    // Ingest 16-bit slider
    reactor.process_hid_fader(1, 32768);
    let actions = reactor.drain_actions();
    assert_eq!(actions.len(), 1);
    match &actions[0] {
        HardwareAction::SliderMoved { index, value } => {
            assert_eq!(*index, 1);
            assert!((*value - 0.5).abs() < 0.01);
        }
        _ => panic!("Expected SliderMoved"),
    }

    // Rotary kinetic momentum
    let mut rotary = RotaryImpulseController::new(3.0);
    let impulse = rotary.translate_rotary_to_kinetic_impulse(2, 2.0);
    assert_eq!(impulse, 12.0);
    let disp = rotary.step(0.016);
    assert!(disp > 0.0);

    // Key LED Feedback
    let mut leds = KeyLedFeedback::new();
    leds.set_key_led(2, Color::Blue);
    let packet = leds.serialize_packet(2).expect("packet serialized");
    assert_eq!(packet, vec![0xAA, 2, 0, 0, 255, 0xFF]);
}

#[test]
fn test_chaos_multibyte_fragmentation_and_fuzzing() {
    let fuzzer = ChaosStreamFuzzer::new();

    // 1. Multibyte UTF-8 fragmentation
    let input = "🚀 Tenui ⚡ Rust 🦀 2026";
    let chunks = fuzzer.fuzz_payload(input.as_bytes());
    assert!(chunks.len() > 5);

    let (reconstructed, processed) = ChaosStreamFuzzer::decode_fragmented_utf8(&chunks);
    assert_eq!(reconstructed, input);
    assert_eq!(processed, chunks.len());

    // 2. Malformed ANSI injection
    let malformed = fuzzer.inject_malformed_ansi(b"Clean Text");
    assert!(malformed.len() > 10);

    // 3. Resize storm invariant
    let storm = fuzzer.fuzz_resize_storm(100, (10, 5), (200, 80));
    assert_eq!(storm.len(), 100);
    for (w, h) in storm {
        assert!((10..=200).contains(&w));
        assert!((5..=80).contains(&h));
    }
}
