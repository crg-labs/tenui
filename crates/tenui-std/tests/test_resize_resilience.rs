use tenui_core::{Color, Modifier};
use tenui_layout::{Style, StyleExt};
use tenui_std::harness::TenuiTestHarness;

#[test]
fn test_hyprland_resize_jitter_50_cycles() {
    let mut harness = TenuiTestHarness::headless(80, 24);

    let test_geometries = [
        (120, 35),
        (80, 24),
        (160, 50),
        (95, 30),
        (80, 24),
        (140, 45),
        (105, 28),
        (160, 50),
        (75, 20),
        (130, 40),
    ];

    for cycle in 0..50 {
        let (w, h) = test_geometries[cycle % test_geometries.len()];

        harness.clear_emitted_bytes();
        harness.resize(w, h);

        // Verify screen purge sequence was emitted on resize
        let resize_bytes = harness.emitted_bytes();
        let resize_str = String::from_utf8_lossy(resize_bytes);
        assert!(
            resize_str.contains("\x1b[2J\x1b[3J\x1b[H"),
            "Cycle {}: screen purge escapes missing on resize to {}x{}",
            cycle,
            w,
            h
        );

        harness.clear_emitted_bytes();

        let render_layout = |h: &mut TenuiTestHarness| {
            h.draw_layout_with_bg(Color::Rgb(30, 30, 46), |ui| {
                ui.column(Style::default(), |col| {
                    col.leaf(Style::default().height_cells(1.0), |canvas| {
                        canvas.set_string(
                            1,
                            0,
                            "TERMINUS JITTER HARNESS",
                            Color::Rgb(137, 180, 250),
                            Color::Reset,
                            Modifier::BOLD,
                        );
                    });
                    col.container(Style::default().grow(1.0).flex_row(), |body| {
                        body.leaf(Style::default().grow(1.0), |canvas| {
                            canvas.set_string(
                                1,
                                1,
                                "Active Viewport Pane Left",
                                Color::Rgb(166, 227, 161),
                                Color::Reset,
                                Modifier::empty(),
                            );
                        });
                        body.leaf(Style::default().grow(1.0), |canvas| {
                            canvas.set_string(
                                1,
                                1,
                                "Active Viewport Pane Right",
                                Color::Rgb(249, 226, 175),
                                Color::Reset,
                                Modifier::empty(),
                            );
                        });
                    });
                });
            });
        };

        // Frame 1: Full draw after resize (front was poisoned)
        render_layout(&mut harness);

        // Frame 2: Static idle frame
        harness.clear_emitted_bytes();
        render_layout(&mut harness);

        // 1. Post-resize front matches back exactly on idle frame
        assert_eq!(
            harness.terminal().front(),
            harness.terminal().back(),
            "Cycle {}: front buffer diverged from back buffer after draw",
            cycle
        );

        // 2. Buffer dimensions match resized geometry
        assert_eq!(harness.terminal().front().width, w);
        assert_eq!(harness.terminal().front().height, h);

        // 3. Static idle frame diff is strictly 16 bytes (synchronized brackets only, zero cell mutations)
        assert_eq!(
            harness.total_bytes_emitted(),
            16,
            "Cycle {}: idle frame emitted extraneous cell diff",
            cycle
        );

        let emitted = harness.emitted_bytes();
        let emitted_str = String::from_utf8_lossy(emitted);

        assert!(
            emitted_str.starts_with("\x1b[?2026h"),
            "Cycle {}: missing synchronized output start bracket",
            cycle
        );
        assert!(
            emitted_str.ends_with("\x1b[?2026l"),
            "Cycle {}: missing synchronized output end bracket",
            cycle
        );

        // Verify ANSI escape sequences are closed properly (terminating character in '@'..='~')
        let mut in_escape = false;
        let mut in_csi = false;
        for &b in emitted {
            if !in_escape {
                if b == 0x1b {
                    in_escape = true;
                    in_csi = false;
                }
            } else if !in_csi {
                if b == b'[' {
                    in_csi = true;
                } else {
                    in_escape = false;
                }
            } else if (0x40..=0x7e).contains(&b) {
                // CSI terminator reached
                in_escape = false;
                in_csi = false;
            }
        }
        assert!(
            !in_escape,
            "Cycle {}: unclosed escape sequence in emitted stream",
            cycle
        );

        // 4. Assert text rendered within active bounds
        harness.assert_text(1, 0, "TERMINUS JITTER HARNESS");
    }

    // Capture deterministic golden snapshot of final state (80x24)
    harness.assert_golden_snapshot("suite_1_resize_resilience");
}
