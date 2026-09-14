use tenui_core::{Buffer, Cell, Color, Rect};
use tenui_std::harness::TenuiTestHarness;

#[test]
fn test_canvas_subview_scissor_clipping() {
    let mut buffer = Buffer::new(50, 30);

    // Initialize all parent cells with a sentinel character and style
    let sentinel = Cell::new("·");
    for cell in &mut buffer.cells {
        *cell = sentinel;
        cell.fg = Color::DarkGray;
        cell.bg = Color::Black;
    }

    let scissor_rect = Rect::new(5, 5, 20, 10);

    {
        let mut subview = buffer.subview_mut(scissor_rect);
        assert_eq!(subview.width(), 20);
        assert_eq!(subview.height(), 10);

        // 1. Test clear(bg) strictly within scissor boundary
        subview.clear(Color::Rgb(40, 40, 60));

        // 2. Out-of-bounds writes (must be safely clipped without panic or memory corruption)
        let cell_x = Cell::new("X");
        subview.set_cell_clipped(25, 5, cell_x);
        subview.set_cell_clipped(5, 15, cell_x);
        subview.set_cell_clipped(20, 10, cell_x);
        subview.set_cell_clipped(200, 200, cell_x);

        // Valid boundary edge write
        let mut cell_corner = Cell::new("Z");
        cell_corner.fg = Color::Red;
        subview.set_cell_clipped(19, 9, cell_corner);

        // 3. String writes with partial and full overflow
        // Overflow row 0: 50 'A's written at x=0 -> exactly 20 fit, 30 clipped
        subview.write_str_clipped(0, 0, &"A".repeat(50), Color::White, Color::Reset);

        // Partial overflow row 2: "HelloWorld" written at x=15 -> 5 chars ("Hello") fit, "World" clipped
        subview.write_str_clipped(15, 2, "HelloWorld", Color::Yellow, Color::Reset);

        // Out-of-bounds row string write: y=12 >= height 10
        subview.write_str_clipped(0, 12, "GhostText", Color::Red, Color::Reset);
    }

    // Asserts on Parent Buffer:

    // 1. Outside scissor bounds MUST remain completely untouched sentinel cells
    for y in 0..buffer.height {
        for x in 0..buffer.width {
            let inside =
                x >= scissor_rect.x && x < scissor_rect.right() && y >= scissor_rect.y && y < scissor_rect.bottom();

            let cell = buffer.get(x, y).expect("cell exists");
            if !inside {
                assert_eq!(
                    cell.symbol.as_str(),
                    "·",
                    "Cell outside scissor at ({}, {}) was corrupted: {:?}",
                    x,
                    y,
                    cell
                );
                assert_eq!(cell.fg, Color::DarkGray);
                assert_eq!(cell.bg, Color::Black);
            }
        }
    }

    // 2. Inside scissor bounds verification:
    // Row 0 of subview (y=5 in parent): exactly 20 'A's
    for x in 0..20 {
        let cell = buffer.get(scissor_rect.x + x, scissor_rect.y).unwrap();
        assert_eq!(cell.symbol.as_str(), "A", "Expected 'A' at subview x={}", x);
    }
    // Column immediately to right of subview (x=25, y=5) must be untouched sentinel
    assert_eq!(buffer.get(25, 5).unwrap().symbol.as_str(), "·");

    // Row 2 of subview (y=7 in parent): "Hello" from x=15..20
    for (i, expected_char) in "Hello".chars().enumerate() {
        let cell = buffer.get(scissor_rect.x + 15 + i as u16, scissor_rect.y + 2).unwrap();
        assert_eq!(cell.symbol.as_str(), &expected_char.to_string());
    }
    // Column immediately to right (x=25, y=7) must be untouched sentinel ("World" was clipped)
    assert_eq!(buffer.get(25, 7).unwrap().symbol.as_str(), "·");

    // Corner cell at local (19, 9) -> parent (24, 14)
    let corner = buffer.get(24, 14).unwrap();
    assert_eq!(corner.symbol.as_str(), "Z");
    assert_eq!(corner.fg, Color::Red);

    // Adjacent cells right and below corner must remain untouched
    assert_eq!(buffer.get(25, 14).unwrap().symbol.as_str(), "·");
    assert_eq!(buffer.get(24, 15).unwrap().symbol.as_str(), "·");

    // Golden Snapshot Verification via TenuiTestHarness
    let mut harness = TenuiTestHarness::headless(50, 30);
    harness.draw_buffer(|b| {
        b.cells.clone_from(&buffer.cells);
    });

    harness.assert_symbol(5, 5, "A");
    harness.assert_symbol(24, 5, "A");
    assert_eq!(harness.get_text_at(5, 5, 20), "A".repeat(20));
    assert_eq!(harness.get_text_at(20, 7, 5), "Hello");
    harness.assert_symbol(24, 14, "Z");

    harness.assert_golden_snapshot("suite_4_scissor_clipping");
}
