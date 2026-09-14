use std::io::Cursor;

use tenui_core::{Buffer, Color, Modifier, Rect, SgrCoalescer, Terminal};

#[test]
fn test_buffer_set_string_and_clipping() {
    let mut buf = Buffer::new(10, 5);
    buf.set_string(0, 0, "Hello", Color::Red, Color::Reset, Modifier::BOLD);

    let cell = buf.get(0, 0).expect("cell exists");
    assert_eq!(cell.symbol.as_str(), "H");
    assert_eq!(cell.fg, Color::Red);
    assert_eq!(cell.modifier, Modifier::BOLD);

    let cell_o = buf.get(4, 0).expect("cell exists");
    assert_eq!(cell_o.symbol.as_str(), "o");

    // Test subview hardware clipping
    let clip_rect = Rect::new(2, 2, 4, 2);
    let mut subview = buf.subview_mut(clip_rect);
    assert_eq!(subview.width(), 4);
    assert_eq!(subview.height(), 2);

    subview.set_string(0, 0, "TEST12345", Color::Green, Color::Reset, Modifier::empty());
    // Only "TEST" fits in width 4, "12345" must be safely clipped

    // Verify in underlying buffer
    assert_eq!(buf.get(2, 2).unwrap().symbol.as_str(), "T");
    assert_eq!(buf.get(3, 2).unwrap().symbol.as_str(), "E");
    assert_eq!(buf.get(4, 2).unwrap().symbol.as_str(), "S");
    assert_eq!(buf.get(5, 2).unwrap().symbol.as_str(), "T");
    // Cell at (6, 2) must remain default
    assert_eq!(buf.get(6, 2).unwrap().symbol.as_str(), " ");
}

#[test]
fn test_wide_character_continuation() {
    let mut buf = Buffer::new(10, 2);
    // '🦀' is visual width 2
    buf.set_string(0, 0, "🦀Rust", Color::Reset, Color::Reset, Modifier::empty());

    let c0 = buf.get(0, 0).unwrap();
    assert_eq!(c0.symbol.as_str(), "🦀");
    assert_eq!(c0.width, 2);
    assert!(!c0.is_continuation);

    let c1 = buf.get(1, 0).unwrap();
    assert_eq!(c1.width, 0);
    assert!(c1.is_continuation);

    let c2 = buf.get(2, 0).unwrap();
    assert_eq!(c2.symbol.as_str(), "R");
    assert_eq!(c2.width, 1);
    assert!(!c2.is_continuation);
}

#[test]
fn test_sgr_coalescing_and_sync_output() {
    let front = Buffer::new(10, 2);
    let mut back = Buffer::new(10, 2);

    back.set_string(2, 1, "AB", Color::Rgb(255, 0, 0), Color::Black, Modifier::BOLD);

    let mut coalescer = SgrCoalescer::new();
    let mut output = Vec::new();
    coalescer
        .write_diff(&mut output, &front, &back)
        .expect("write_diff succeeds");

    let text = String::from_utf8(output).expect("valid utf8");
    assert!(text.starts_with("\x1b[?2026h"));
    assert!(text.ends_with("\x1b[?2026l"));
    assert!(text.contains("\x1b[2;3H")); // 1-based cursor jump to y=1, x=2 -> row 2, col 3
    // Coalesced compound SGR: BOLD (1) + RGB fg + black bg, in one escape.
    assert!(text.contains("\x1b[1;38;2;255;0;0;40m"));
    assert!(text.contains("AB"));
}

#[test]
fn test_write_diff_indexed_matches_full_diff() {
    // The indexed (SIMD-fed) diff must emit byte-identical output to the full scan.
    let front = Buffer::new(24, 4);
    let mut back = Buffer::new(24, 4);
    back.set_string(2, 1, "Hello", Color::Rgb(255, 0, 0), Color::Black, Modifier::BOLD);
    back.set_string(10, 2, "Zz", Color::Green, Color::Reset, Modifier::empty());

    // Exact scalar dirty set.
    let mut dirty = Vec::new();
    for i in 0..front.cells.len() {
        if front.cells[i] != back.cells[i] {
            dirty.push(i);
        }
    }

    let mut full = SgrCoalescer::new();
    let mut full_out = Vec::new();
    full.write_diff(&mut full_out, &front, &back).unwrap();

    let mut indexed = SgrCoalescer::new();
    let mut indexed_out = Vec::new();
    indexed
        .write_diff_indexed(&mut indexed_out, &front, &back, &dirty)
        .unwrap();

    assert_eq!(full_out, indexed_out);
}

#[test]
fn test_egc_interning_and_spillover() {
    use tenui_core::{Cell, CompactSymbol};

    // Standard 4-byte emoji fits inline
    let c1 = CompactSymbol::from_str("🦀");
    assert!(!c1.is_spillover());
    assert_eq!(c1.as_str(), "🦀");

    // Composite emoji with ZWJ sequence (> 7 bytes): "👨‍👩‍👧‍👦" (25 bytes)
    let family = "👨‍👩‍👧‍👦";
    assert!(family.len() > 7);
    let c2 = CompactSymbol::from_str(family);
    assert!(c2.is_spillover());
    assert_eq!(c2.as_str(), family);

    let cell = Cell::new(family);
    assert_eq!(cell.as_str(), family);
    assert_eq!(cell.as_grapheme_str(), family);
}

#[test]
fn test_visual_degradation_cascade() {
    use tenui_core::{Cell, TerminalCapabilities};

    let box_cell = Cell::new("│");
    let check_cell = Cell::new("✔");

    let utf8_caps = TerminalCapabilities {
        has_utf8: true,
        ..Default::default()
    };
    let legacy_caps = TerminalCapabilities {
        has_utf8: false,
        ..Default::default()
    };

    // On UTF-8 terminal: preserves glyphs
    assert_eq!(box_cell.resolve_degradation(&utf8_caps).0, '│');
    assert_eq!(check_cell.resolve_degradation(&utf8_caps).0, '✔');

    // On legacy non-UTF8 terminal: degrades to ASCII
    assert_eq!(box_cell.resolve_degradation(&legacy_caps).0, '|');
    assert_eq!(check_cell.resolve_degradation(&legacy_caps).0, 'V');
}

#[test]
fn test_extended_underline_and_csi58_color() {
    use tenui_core::{Buffer, Color, SgrCoalescer, UnderlineStyle};

    let front = Buffer::new(10, 1);
    let mut back = Buffer::new(10, 1);

    let cell = back.get_mut(0, 0).unwrap();
    cell.set_char('U');
    cell.set_underline_style(UnderlineStyle::Curly);
    cell.set_underline_color(Color::Rgb(255, 50, 50));

    let mut coalescer = SgrCoalescer::new();
    let mut output = Vec::new();
    coalescer
        .write_diff(&mut output, &front, &back)
        .expect("write_diff succeeds");

    let text = String::from_utf8(output).expect("valid utf8");
    assert!(text.contains("4:3")); // Curly underline
    assert!(text.contains("58;2;255;50;50")); // CSI 58 extended underline color
}

#[test]
fn test_headless_terminal_diffing() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 20, 5);

    // Frame 1: Draw "Hello"
    term.draw(|ui| {
        ui.set_string(0, 0, "Hello", Color::Reset, Color::Reset, Modifier::empty());
    })
    .unwrap();

    assert_eq!(term.front().get(0, 0).unwrap().symbol.as_str(), "H");
    assert_eq!(term.front().get(4, 0).unwrap().symbol.as_str(), "o");

    // Frame 2: Identical content -> diff should be empty (no cell output except sync brackets)
    let output2 = Vec::new();
    let cursor = Cursor::new(output2);
    let mut term2 = Terminal::with_writer(cursor, 20, 5);

    // Populate term2 front to match Frame 1
    term2
        .draw(|ui| {
            ui.set_string(0, 0, "Hello", Color::Reset, Color::Reset, Modifier::empty());
        })
        .unwrap();

    // Frame 3: Mutate only 1 char
    let output3 = Vec::new();
    let mut term3 = Terminal::with_writer(Cursor::new(output3), 20, 5);
    term3
        .draw(|ui| {
            ui.set_string(0, 0, "Hello", Color::Reset, Color::Reset, Modifier::empty());
        })
        .unwrap();

    // Change 'o' to '!'
    term3
        .draw(|ui| {
            ui.set_string(0, 0, "Hell!", Color::Reset, Color::Reset, Modifier::empty());
        })
        .unwrap();

    assert_eq!(term3.front().get(4, 0).unwrap().symbol.as_str(), "!");
}

#[test]
fn test_terminal_guard_resize_signal_interception() {
    let term = Terminal::with_writer(Cursor::new(Vec::new()), 20, 5);
    // In headless mode without guard, has_resized is false
    assert!(!term.has_resized());
    assert!(!term.take_resized());
}

#[test]
fn test_cell_poison_and_buffer_poison_all() {
    use tenui_core::Cell;

    let mut front = Buffer::new(10, 2);
    let mut back = Buffer::new(10, 2);

    // Populate both buffers with identical spaces
    front.clear();
    back.clear();

    let mut coalescer = SgrCoalescer::new();
    let mut diff_bytes = Vec::new();
    coalescer.write_diff(&mut diff_bytes, &front, &back).unwrap();
    // Since front and back are identical, diff_bytes should only contain sync brackets
    let empty_len = SgrCoalescer::SYNC_START.len() + SgrCoalescer::SYNC_END.len();
    assert_eq!(
        diff_bytes.len(),
        empty_len,
        "Identical buffers should produce no cell diff"
    );

    // Poison front buffer
    front.poison_all();
    assert_eq!(front.get(0, 0).unwrap(), &Cell::POISON);

    // Diff again: every single cell should now be emitted because POISON never equals back
    diff_bytes.clear();
    coalescer.write_diff(&mut diff_bytes, &front, &back).unwrap();
    assert!(
        diff_bytes.len() > empty_len,
        "Poisoned front buffer must force differential emission of all cells"
    );
}

#[test]
fn test_terminal_resize_purge_and_poison() {
    use tenui_core::Cell;

    let cursor = Cursor::new(Vec::new());
    let mut term = Terminal::with_writer(cursor, 20, 5);

    // Initial draw
    term.draw(|ui| {
        ui.set_string(0, 0, "Initial", Color::Reset, Color::Reset, Modifier::empty());
    })
    .unwrap();

    // Clear recorded writer output and reset cursor position
    term.writer_mut().set_position(0);
    term.writer_mut().get_mut().clear();

    // Perform resize
    term.resize(40, 10);
    assert_eq!(term.size(), (40, 10));

    // Verify host terminal screen purge escape sequence \x1b[2J\x1b[3J\x1b[H was written
    let emitted = term.writer().get_ref();
    assert!(
        emitted.starts_with(b"\x1b[2J\x1b[3J\x1b[H"),
        "Resize must emit physical screen clear and scrollback purge"
    );

    // Verify front buffer was poisoned
    assert_eq!(term.front().get(0, 0).unwrap(), &Cell::POISON);
}

#[test]
fn test_subview_clear_and_clipping() {
    let mut buf = Buffer::new(20, 5);
    let clip = Rect::new(5, 1, 10, 3);
    {
        let mut subview = buf.subview_mut(clip);
        // Clear subview with custom background
        subview.clear(Color::Rgb(10, 20, 30));
    }
    assert_eq!(buf.get(5, 1).unwrap().bg, Color::Rgb(10, 20, 30));
    // Outside subview should remain default
    assert_eq!(buf.get(4, 1).unwrap().bg, Color::Reset);

    {
        let mut subview = buf.subview_mut(clip);
        // Test write_str_clipped
        subview.write_str_clipped(0, 0, "1234567890ABCDE", Color::White, Color::Black);
    }
    // Columns 0..10 (global 5..15) should be written
    for (i, ch) in "1234567890".chars().enumerate() {
        assert_eq!(buf.get(5 + i as u16, 1).unwrap().symbol.as_str(), &ch.to_string());
    }
    // Column 15 (global 5 + 10 = 15) must NOT contain 'A'
    assert_ne!(buf.get(15, 1).unwrap().symbol.as_str(), "A");
}

#[test]
fn test_text_overflow_ellipsis_and_wrapping() {
    use tenui_core::TextOverflow;

    let mut buf = Buffer::new(20, 8);
    let mut subview = buf.subview_mut(Rect::new(0, 0, 20, 8));

    // 1. Text fits within max_width
    let written = subview.write_str_overflow(0, 0, "Hello", 10, TextOverflow::Ellipsis, Color::White, Color::Black);
    assert_eq!(written, 5);
    assert_eq!(subview.get(0, 0).unwrap().symbol.as_str(), "H");
    assert_eq!(subview.get(4, 0).unwrap().symbol.as_str(), "o");

    // 2. Text overflows max_width with Ellipsis
    let written2 = subview.write_str_overflow(
        0,
        1,
        "VeryLongTitleExceedingBound",
        10,
        TextOverflow::Ellipsis,
        Color::White,
        Color::Black,
    );
    assert_eq!(written2, 10);
    // Should have 9 characters + '…'
    assert_eq!(subview.get(9, 1).unwrap().symbol.as_str(), "…");
    assert_eq!(subview.get(10, 1).unwrap().symbol.as_str(), " "); // outside max_width unchanged

    // 3. Text overflows with Clip
    let written3 = subview.write_str_overflow(
        0,
        2,
        "VeryLongTitleExceedingBound",
        6,
        TextOverflow::Clip,
        Color::White,
        Color::Black,
    );
    assert_eq!(written3, 6);
    assert_eq!(subview.get(4, 2).unwrap().symbol.as_str(), "L");
    assert_eq!(subview.get(5, 2).unwrap().symbol.as_str(), "o");
    assert_eq!(subview.get(6, 2).unwrap().symbol.as_str(), " ");

    // 4. Wrapping
    let lines = subview.write_str_wrapped(0, 3, "one two three four", 10, 2, Color::White, Color::Black);
    assert_eq!(lines, 2);

    // 5. Horizontal Scrolling
    let written5 = subview.write_str_overflow(
        0,
        4,
        "0123456789ABCDEF",
        8,
        TextOverflow::ScrollHorizontal { offset: 5 },
        Color::White,
        Color::Black,
    );
    assert_eq!(written5, 8);
    assert_eq!(subview.get(0, 4).unwrap().symbol.as_str(), "5");
    assert_eq!(subview.get(7, 4).unwrap().symbol.as_str(), "C");

    // 6. Tab expansion and control char sanitization
    subview.write_str_clipped(0, 5, "A\tB\x07C", Color::White, Color::Black);
    assert_eq!(subview.get(0, 5).unwrap().symbol.as_str(), "A");
    assert_eq!(subview.get(1, 5).unwrap().symbol.as_str(), " ");
    assert_eq!(subview.get(2, 5).unwrap().symbol.as_str(), " ");
    assert_eq!(subview.get(3, 5).unwrap().symbol.as_str(), " ");
    assert_eq!(subview.get(4, 5).unwrap().symbol.as_str(), "B");
    assert_eq!(subview.get(5, 5).unwrap().symbol.as_str(), "C"); // \x07 dropped
}

#[test]
fn test_wrap_with_continuation_marks_and_insets_continuation_lines() {
    use tenui_core::TextOverflow;

    let mut buf = Buffer::new(12, 8);
    let mut sv = buf.subview_mut(Rect::new(0, 0, 12, 8));

    // Width 10, continuation marker "> " with indent 2. "aaaa bbbb cccc" wraps:
    //   line 0 (width 10):           "aaaa bbbb"   (4+1+4=9 fits)
    //   line 1 (cont, width 10-2=8): "> " + "cccc"
    sv.write_str_overflow(
        0,
        0,
        "aaaa bbbb cccc",
        10,
        TextOverflow::WrapWithContinuation {
            indent: 2,
            marker: "> ",
        },
        Color::White,
        Color::Black,
    );

    // First line: no marker, starts at column 0.
    assert_eq!(sv.get(0, 0).unwrap().symbol.as_str(), "a");
    assert_eq!(sv.get(5, 0).unwrap().symbol.as_str(), "b");
    // Continuation line: marker at the left edge, text inset by `indent`.
    assert_eq!(sv.get(0, 1).unwrap().symbol.as_str(), ">");
    assert_eq!(sv.get(1, 1).unwrap().symbol.as_str(), " ");
    assert_eq!(sv.get(2, 1).unwrap().symbol.as_str(), "c");
    assert_eq!(sv.get(5, 1).unwrap().symbol.as_str(), "c");

    // A word longer than the continuation width is hard-broken across inset lines.
    let mut buf2 = Buffer::new(12, 8);
    let mut sv2 = buf2.subview_mut(Rect::new(0, 0, 12, 8));
    let n = sv2.write_str_wrapped_cont(0, 0, "xx yyyyyyyyyy", 6, 8, 2, "↳", Color::White, Color::Black);
    assert!(
        n >= 3,
        "long word should hard-break onto multiple continuation lines, got {n}"
    );
    assert_eq!(sv2.get(0, 0).unwrap().symbol.as_str(), "x"); // first line, no marker
    assert_eq!(sv2.get(0, 1).unwrap().symbol.as_str(), "↳"); // continuation marker present
}
