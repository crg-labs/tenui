use std::{thread, time::Duration};

use tenui_core::{FrameStatus, InputDemuxer, InputEvent, NonBlockingWriter};

#[test]
fn test_bracketed_paste_transactional_demuxing() {
    let mut demuxer = InputDemuxer::new();
    let raw_payload = b"\x1b[200~Pasted Multi-Line\nData\x1b[201~";

    let events = demuxer.feed_bytes(raw_payload);
    assert_eq!(events.len(), 1);
    match &events[0] {
        InputEvent::Paste(content) => {
            assert_eq!(content, "Pasted Multi-Line\nData");
        }
        _ => panic!("Expected Paste event"),
    }
}

#[test]
fn test_capability_response_demuxing() {
    let mut demuxer = InputDemuxer::new();
    let da1_response = b"\x1b[?1;2c";

    let events = demuxer.feed_bytes(da1_response);
    assert_eq!(events.len(), 1);
    match &events[0] {
        InputEvent::CapabilityResponse(resp) => {
            assert_eq!(resp, "\x1b[?1;2c");
        }
        _ => panic!("Expected CapabilityResponse event"),
    }
}

#[test]
fn test_non_blocking_writer_frame_dropping_under_backpressure() {
    // A slow target that blocks write
    struct SlowWriter;
    impl std::io::Write for SlowWriter {
        fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
            thread::sleep(Duration::from_millis(50));
            Ok(buf.len())
        }
        fn flush(&mut self) -> std::io::Result<()> {
            Ok(())
        }
    }

    // Capacity of 1 frame
    let writer = NonBlockingWriter::custom(SlowWriter, 1);

    // First frame fills queue
    let s1 = writer.try_send_frame(vec![1, 2, 3]);
    assert_eq!(s1, FrameStatus::Queued);

    // Quick burst while worker is sleeping 50ms: must drop and not block
    let mut dropped = 0;
    for _ in 0..10 {
        if writer.try_send_frame(vec![4, 5, 6]) == FrameStatus::DroppedBackpressure {
            dropped += 1;
        }
    }

    assert!(dropped > 0, "Must drop frames when background queue is full");
    assert!(writer.dropped_frames() > 0);
}

#[test]
fn test_sgr_1016_pixel_mouse_and_decoupler() {
    use tenui_core::PixelDecoupler;

    let mut demuxer = InputDemuxer::new();
    // \x1b[<0;542;381M -> Button 0, Pixel X=542, Pixel Y=381, Press
    let sgr_payload = b"\x1b[<0;542;381M";
    let events = demuxer.feed_bytes(sgr_payload);
    assert_eq!(events.len(), 1);

    match &events[0] {
        InputEvent::PixelMouse {
            button,
            pixel_x,
            pixel_y,
            is_release,
        } => {
            assert_eq!(*button, 0);
            assert_eq!(*pixel_x, 542);
            assert_eq!(*pixel_y, 381);
            assert!(!is_release);

            // Test decoupler: 10px wide x 20px tall cells
            let decoupler = PixelDecoupler::new(10.0, 20.0);
            let (col_frac, row_frac) = decoupler.to_cell_coords(*pixel_x, *pixel_y);
            assert!((col_frac - 54.2).abs() < 1e-4);
            assert!((row_frac - 19.05).abs() < 1e-4);
        }
        _ => panic!("Expected PixelMouse event"),
    }

    // Test release: \x1b[<0;542;381m
    let release_payload = b"\x1b[<0;542;381m";
    let rel_events = demuxer.feed_bytes(release_payload);
    assert_eq!(rel_events.len(), 1);
    match &rel_events[0] {
        InputEvent::PixelMouse { is_release, .. } => {
            assert!(is_release);
        }
        _ => panic!("Expected PixelMouse release event"),
    }
}

#[test]
fn test_kitty_key_repeat_and_event_helpers() {
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyEventState, KeyModifiers};

    let press_event = InputEvent::Key(KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Press,
        state: KeyEventState::NONE,
    });
    assert!(press_event.is_key_press());
    assert!(!press_event.is_key_repeat());
    assert!(!press_event.is_key_release());
    assert_eq!(press_event.key_code(), Some(KeyCode::Char('j')));
    assert_eq!(press_event.key_modifiers(), Some(KeyModifiers::NONE));

    let repeat_event = InputEvent::Key(KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Repeat,
        state: KeyEventState::NONE,
    });
    assert!(!repeat_event.is_key_press());
    assert!(repeat_event.is_key_repeat());
    assert!(!repeat_event.is_key_release());

    let release_event = InputEvent::Key(KeyEvent {
        code: KeyCode::Char('j'),
        modifiers: KeyModifiers::NONE,
        kind: KeyEventKind::Release,
        state: KeyEventState::NONE,
    });
    assert!(!release_event.is_key_press());
    assert!(!release_event.is_key_repeat());
    assert!(release_event.is_key_release());
}

#[test]
fn test_const_cell_and_modifier_attributes() {
    use tenui_core::{Cell, Modifier};

    const MOD: Modifier = Modifier::PLAIN.with(Modifier::BOLD).with(Modifier::ITALIC);
    assert!(MOD.has(Modifier::BOLD));
    assert!(MOD.has(Modifier::ITALIC));
    assert!(!MOD.has(Modifier::UNDERLINE));

    const WITHOUT_BOLD: Modifier = MOD.without(Modifier::BOLD);
    assert!(!WITHOUT_BOLD.has(Modifier::BOLD));
    assert!(WITHOUT_BOLD.has(Modifier::ITALIC));

    const BLANK_CELL: Cell = Cell::blank();
    assert_eq!(BLANK_CELL.symbol.as_str(), " ");
    assert_eq!(BLANK_CELL.modifier, Modifier::PLAIN);
    assert_eq!(BLANK_CELL.width, 1);
}
