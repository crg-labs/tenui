use tenui_core::{
    CursorStyle, DevToolsProtocol, FrameTelemetry, InlineViewport, TerminalDriver, TimeTravelRingBuffer,
    VirtualTerminalDriver, format_cursor_sync, format_osc8_hyperlink, is_light_theme, osc11_query,
    parse_osc11_response, relative_luminance,
};

#[test]
fn test_osc11_ambient_palette_interrogation() {
    assert_eq!(osc11_query(), "\x1b]11;?\x07");

    // Dark background response: #1e1e1e
    let dark_resp = "\x1b]11;rgb:1e1e/1e1e/1e1e\x07";
    let (r, g, b) = parse_osc11_response(dark_resp).expect("valid osc11 response");
    assert_eq!(r, 0x1e);
    assert_eq!(g, 0x1e);
    assert_eq!(b, 0x1e);
    assert!(!is_light_theme(r, g, b));
    assert!(relative_luminance(r, g, b) < 0.2);

    // Light background response: #f5f5f5
    let light_resp = "\x1b]11;rgb:f5f5/f5f5/f5f5\x1b\\";
    let (lr, lg, lb) = parse_osc11_response(light_resp).expect("valid osc11 response");
    assert_eq!(lr, 0xf5);
    assert_eq!(lg, 0xf5);
    assert_eq!(lb, 0xf5);
    assert!(is_light_theme(lr, lg, lb));
    assert!(relative_luminance(lr, lg, lb) > 0.8);
}

#[test]
fn test_osc8_hyperlinks_and_ime_cursor_sync() {
    let link = format_osc8_hyperlink("https://tenui-tui.org", "Tenui Website");
    assert!(link.starts_with("\x1b]8;;https://tenui-tui.org\x1b\\"));
    assert!(link.ends_with("\x1b]8;;\x1b\\"));
    assert!(link.contains("Tenui Website"));

    let cursor_sync = format_cursor_sync(10, 5, CursorStyle::BlinkingBar);
    // Style 5 (blinking bar), row 6, col 11 (1-based), cursor visible (?25h)
    assert_eq!(cursor_sync, "\x1b[5 q\x1b[6;11H\x1b[?25h");
}

#[test]
fn test_inline_viewport_lifecycle() {
    let mut viewport = InlineViewport::new(5);
    assert_eq!(viewport.height(), 5);

    let mut out = Vec::new();
    viewport.begin(&mut out).unwrap();
    let text_start = String::from_utf8(out.clone()).unwrap();
    assert!(text_start.contains("\x1b[?25l")); // cursor hidden
    assert!(text_start.contains("\x1b[5A")); // moves up 5 lines

    viewport.render_frame(&mut out, b"FRAME CONTENT").unwrap();
    assert!(out.ends_with(b"FRAME CONTENT"));

    viewport.finish(&mut out).unwrap();
    let text_end = String::from_utf8(out).unwrap();
    assert!(text_end.contains("\x1b[5B\r\n")); // cursor moved down below region
    assert!(text_end.contains("\x1b[?25h")); // cursor restored
}

#[test]
fn test_virtual_terminal_driver() {
    let mut driver = VirtualTerminalDriver::new(80, 24);
    assert_eq!(driver.terminal_size(), (80, 24));

    driver.write_diff(b"HELLO VIRTUAL").unwrap();
    driver.flush().unwrap();
    assert_eq!(driver.output_sink, b"HELLO VIRTUAL");

    assert!(driver.poll_event(None).unwrap().is_none());

    driver.push_event(tenui_core::InputEvent::Resize(100, 30));
    let evt = driver.poll_event(None).unwrap().expect("event available");
    assert_eq!(evt, tenui_core::InputEvent::Resize(100, 30));
}

#[test]
fn test_devtools_ring_buffer_and_protocol() {
    let mut ring = TimeTravelRingBuffer::new(3);
    assert!(ring.is_empty());

    ring.record("State A");
    ring.record("State B");
    ring.record("State C");
    ring.record("State D"); // Evicts State A

    assert_eq!(ring.len(), 3);
    assert_eq!(ring.step_backward(), Some(&"State D")); // Head of ring
    assert_eq!(ring.step_backward(), Some(&"State C")); // Step back to prior
    assert_eq!(ring.step_forward(), Some(&"State D")); // Step forward back to head

    let telemetry = FrameTelemetry {
        frame_index: 42,
        reflow_duration_us: 120,
        paint_duration_us: 45,
        bytes_emitted: 312,
        layout_invoked: true,
    };
    let json = DevToolsProtocol::format_telemetry_response(1, &telemetry);
    assert!(json.contains("\"jsonrpc\":\"2.0\""));
    assert!(json.contains("\"frame\":42"));
    assert!(json.contains("\"reflow_us\":120"));
    assert!(json.contains("\"paint_us\":45"));
}
