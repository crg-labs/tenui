use tenui_core::{
    Buffer, ColorTier, NetworkBandwidthState, SshFlowController, bidi_reorder, contains_rtl, is_rtl_char,
};
use tenui_std::{SubprocessState, SubprocessView};

#[test]
fn test_ssh_flow_controller_adaptive_degradation() {
    let mut controller = SshFlowController::new();
    assert_eq!(controller.state(), NetworkBandwidthState::Nominal);
    assert_eq!(controller.target_fps(), 60);
    assert!(controller.should_microstep());
    assert!(!controller.should_drop_intermediate_frames());
    assert_eq!(controller.color_tier(), ColorTier::TrueColor);

    // Moderate latency: 45ms -> Degraded
    controller.record_latency(45.0);
    assert_eq!(controller.state(), NetworkBandwidthState::Degraded);
    assert_eq!(controller.target_fps(), 30);
    assert!(!controller.should_microstep());
    assert!(!controller.should_drop_intermediate_frames());
    assert_eq!(controller.color_tier(), ColorTier::Ansi256);

    // High latency: 120ms -> Congested
    controller.record_latency(120.0);
    assert_eq!(controller.state(), NetworkBandwidthState::Congested);
    assert_eq!(controller.target_fps(), 15);
    assert!(!controller.should_microstep());
    assert!(controller.should_drop_intermediate_frames());
    assert_eq!(controller.color_tier(), ColorTier::Ansi16);

    // Network recovers: 10ms -> Nominal
    controller.record_latency(10.0);
    assert_eq!(controller.state(), NetworkBandwidthState::Nominal);
    assert_eq!(controller.target_fps(), 60);
}

#[test]
fn test_unicode_bidi_text_reordering() {
    // Pure ASCII
    let ascii = "Tenui CLI";
    assert!(!contains_rtl(ascii));
    assert_eq!(bidi_reorder(ascii), "Tenui CLI");

    // Hebrew: 'שלום' (Shalom)
    let hebrew = "שלום";
    assert!(contains_rtl(hebrew));
    assert!(is_rtl_char('ש'));
    // Visual order should be reversed for terminal LTR cell placement
    let reversed_expected: String = hebrew.chars().rev().collect();
    assert_eq!(bidi_reorder(hebrew), reversed_expected);

    // Mixed text: "Status: שלום (OK)"
    let mixed = "Status: שלום (OK)";
    let reordered = bidi_reorder(mixed);
    assert!(reordered.starts_with("Status: "));
    assert!(reordered.ends_with(" (OK)"));
    assert!(reordered.contains(&reversed_expected));
}

#[test]
fn test_subprocess_view_crash_recovery() {
    let mut pane = SubprocessView::new("echo", &["test"], 80, 10);

    let mut buf = Buffer::new(80, 10);
    let full = buf.rect();

    // Trigger crash simulation
    pane.trigger_crash("SIGSEGV signal fault");
    assert_eq!(pane.state, SubprocessState::Crashed("SIGSEGV signal fault".to_string()));

    // Render crash banner
    let mut subview = buf.subview_mut(full);
    pane.render(&mut subview);

    // Verify localized crash banner was rendered without crashing supervisor
    let mut full_text = String::new();
    for y in 0..10 {
        for x in 0..80 {
            if let Some(cell) = buf.get(x, y) {
                full_text.push_str(cell.symbol.as_str());
            }
        }
    }
    assert!(full_text.contains("[!] Pane Process Terminated - Press 'r' to Reload"));
    assert!(full_text.contains("SIGSEGV"));

    // Verify reload recovery
    pane.command = "true".to_string();
    pane.args = vec![];
    let res = pane.reload();
    assert!(res.is_ok());
}
