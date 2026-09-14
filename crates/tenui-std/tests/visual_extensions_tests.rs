use std::path::PathBuf;

use tenui_core::{Buffer, Color};
use tenui_std::{
    ChatMessage, ChatThread, FileEntry, FilePicker, GraphicsProtocol, ImageCanvas, LogStreamer, MultiProgress,
    ReplInput, Spinner, SuggestionItem, TaskStatus, ToolCallCard, WhichKey,
};

#[test]
fn test_image_canvas_rendering_and_formats() {
    // 2x2 test image: Red, Green, Blue, White
    let rgba = vec![255, 0, 0, 255, 0, 255, 0, 255, 0, 0, 255, 255, 255, 255, 255, 255];
    let canvas = ImageCanvas::new(2, 2, rgba);

    // Escape sequences
    let kitty = canvas.format_kitty_apc();
    assert!(kitty.starts_with("\x1b_G"));
    assert!(kitty.contains("s=2,v=2"));

    let iterm = canvas.format_iterm2_inline();
    assert!(iterm.starts_with("\x1b]1337;File=inline=1"));
    assert!(iterm.contains("width=2px;height=2px"));

    // Half-block fallback rendering onto a 2x1 cell buffer (since 1 cell = 2 vertical subpixels)
    let mut buf = Buffer::new(2, 1);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    canvas.render(&mut subview, GraphicsProtocol::HalfBlock);

    // Left cell: top is red, bottom is blue
    let cell_0_0 = buf.get(0, 0).unwrap();
    assert_eq!(cell_0_0.symbol.as_str(), "▀");
    assert_eq!(cell_0_0.fg, Color::Rgb(255, 0, 0));
    assert_eq!(cell_0_0.bg, Color::Rgb(0, 0, 255));

    // Right cell: top is green, bottom is white
    let cell_1_0 = buf.get(1, 0).unwrap();
    assert_eq!(cell_1_0.symbol.as_str(), "▀");
    assert_eq!(cell_1_0.fg, Color::Rgb(0, 255, 0));
    assert_eq!(cell_1_0.bg, Color::Rgb(255, 255, 255));
}

#[test]
fn test_repl_input_suggestions_and_dynamic_flip() {
    let mut repl = ReplInput::new();
    repl.set_text("gi");
    repl.set_suggestions(vec![
        SuggestionItem::new("git checkout").with_detail("switch branches"),
        SuggestionItem::new("git commit").with_detail("record changes"),
        SuggestionItem::new("git clone").with_detail("clone repo"),
    ]);

    assert!(repl.popup_visible);
    assert_eq!(repl.selected_suggestion, Some(0));

    // Navigation
    repl.select_next();
    assert_eq!(repl.selected_suggestion, Some(1));
    repl.select_prev();
    assert_eq!(repl.selected_suggestion, Some(0));
    repl.select_prev(); // Wraps to last
    assert_eq!(repl.selected_suggestion, Some(2));

    // Apply suggestion
    repl.complete_current();
    assert_eq!(repl.text, "git clone");
    assert_eq!(repl.cursor_pos, 9);
    assert!(!repl.popup_visible);

    // Dynamic flip test:
    // When input is near the bottom (y = 8 out of 10), popup flips ABOVE the input line.
    let mut buf = Buffer::new(40, 10);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);

    repl.set_text("run ");
    repl.set_suggestions(vec![SuggestionItem::new("cargo test")]);
    repl.popup_visible = true;

    // Rendering at y=8 in a 10-row viewport should trigger the flip-above logic
    repl.render_at(&mut subview, 8, Color::WHITE, Color::BLACK, "> ");

    // Check that popup exists above row 8 (row 6 has the suggestion text)
    let mut popup_text = String::new();
    for x in 6..20 {
        popup_text.push_str(buf.get(x, 6).unwrap().symbol.as_str());
    }
    assert!(
        popup_text.contains("cargo test"),
        "Popup should contain suggestion: {}",
        popup_text
    );
}

#[test]
fn test_log_streamer_ring_buffer_and_smart_pause() {
    let mut streamer = LogStreamer::new(4);
    assert_eq!(streamer.len(), 0);

    // Push 6 lines into capacity 4
    for i in 1..=6 {
        streamer.push_line(format!("Log message {i}"));
    }

    assert_eq!(streamer.len(), 4);
    // Buffer contains last 4: 3, 4, 5, 6
    assert_eq!(streamer.get_line(0), Some("Log message 3"));
    assert_eq!(streamer.get_line(1), Some("Log message 4"));
    assert_eq!(streamer.get_line(2), Some("Log message 5"));
    assert_eq!(streamer.get_line(3), Some("Log message 6"));
    assert_eq!(streamer.get_line(4), None);

    // Autoscroll smart pause
    assert!(streamer.is_auto_scroll);
    streamer.scroll_up(2);
    assert!(!streamer.is_auto_scroll);

    let mut buf = Buffer::new(40, 6);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    streamer.render(&mut subview, Color::WHITE, Color::BLACK);

    // Status pill should render "[⏸ Autoscroll Paused]" in top-right (row 0)
    let mut rendered_text = String::new();
    for x in 0..40 {
        rendered_text.push_str(buf.get(x, 0).unwrap().symbol.as_str());
    }
    assert!(
        rendered_text.contains("⏸ Autoscroll Paused"),
        "Buffer should show pause indicator: {}",
        rendered_text
    );

    // Resume auto-scroll
    streamer.scroll_to_bottom();
    assert!(streamer.is_auto_scroll);
}

#[test]
fn test_multi_progress_ewma_and_collapse() {
    let mut mp = MultiProgress::new();
    let task_id = mp.add_task("Compiling crates");

    let task = mp.get_task_mut(task_id).unwrap();
    assert_eq!(task.status, TaskStatus::Running);
    assert_eq!(task.rate_bytes_per_sec, 0.0);

    // Update EWMA rate with alpha 0.5
    task.update_rate_ewma(100.0, 0.5);
    assert_eq!(task.rate_bytes_per_sec, 50.0);
    task.update_rate_ewma(100.0, 0.5);
    assert_eq!(task.rate_bytes_per_sec, 75.0);

    // Tick collapse factor
    task.status = TaskStatus::Completed;
    task.collapse_factor = 1.0;
    mp.tick_spring_collapse(0.1);
    let updated = mp.get_task_mut(task_id).unwrap();
    assert!(updated.collapse_factor < 1.0);

    // Render verification
    let mut buf = Buffer::new(50, 4);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    mp.render(&mut subview, Color::WHITE, Color::BLACK);

    let cell = buf.get(0, 0).unwrap();
    assert_ne!(cell.symbol.as_str(), " ");
}

#[test]
fn test_file_picker_nerd_font_cascade_and_navigation() {
    let mut picker = FilePicker::new(PathBuf::from("/workspace"));
    picker.set_entries(vec![
        FileEntry::new("src", true, 0),
        FileEntry::new("main.rs", false, 1024),
        FileEntry::new("README.md", false, 512),
    ]);

    assert_eq!(picker.selected_idx, 0);
    picker.select_next();
    assert_eq!(picker.selected_idx, 1);
    assert_eq!(picker.selected_entry().unwrap().name, "main.rs");

    // Test icon fallback cascade
    let dir_entry = FileEntry::new("docs", true, 0);
    let (icon_nerd, _) = dir_entry.resolve_icon(true);
    let (icon_ascii, _) = dir_entry.resolve_icon(false);
    assert_eq!(icon_nerd, "");
    assert_eq!(icon_ascii, "[DIR]");

    let rs_entry = FileEntry::new("lib.rs", false, 2048);
    let (rs_nerd, _) = rs_entry.resolve_icon(true);
    let (rs_ascii, _) = rs_entry.resolve_icon(false);
    assert_eq!(rs_nerd, "");
    assert_eq!(rs_ascii, "[RS]");

    // Render file picker
    let mut buf = Buffer::new(40, 6);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    picker.render(&mut subview, Color::WHITE, Color::BLACK);

    // Check breadcrumb
    let mut header = String::new();
    for x in 0..20 {
        header.push_str(buf.get(x, 0).unwrap().symbol.as_str());
    }
    assert!(header.contains("workspace"), "Header should contain path: {}", header);
}

#[test]
fn test_which_key_drawer() {
    let mut wk = WhichKey::new("Leader Key (SPC)");
    wk.add_action("f", "Find file");
    wk.add_action("g", "Git status");
    wk.add_action("q", "Quit");

    assert!(!wk.is_open);
    wk.open();
    assert!(wk.is_open);

    let mut buf = Buffer::new(60, 6);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    wk.render(&mut subview);

    // Drawer is mounted at bottom (start_y = 6 - 4 = 2)
    let top_border = buf.get(0, 2).unwrap();
    assert_eq!(top_border.symbol.as_str(), "─");

    // Title rendered in top border
    let mut title_row = String::new();
    for x in 0..30 {
        title_row.push_str(buf.get(x, 2).unwrap().symbol.as_str());
    }
    assert!(title_row.contains("Leader Key (SPC)"));
}

#[test]
fn test_chat_thread_streaming_and_tool_cards() {
    let mut thread = ChatThread::new();
    thread.add_message(ChatMessage::user("Optimize this SQL query"));

    assert_eq!(thread.messages.len(), 1);
    assert!(thread.auto_tail_lock);

    // Stream assistant response chunks
    thread.start_streaming_assistant();
    thread.append_streaming_token("SELECT * FROM users ");
    thread.append_streaming_token("WHERE active = true;");

    assert_eq!(thread.messages.len(), 2);
    let assistant_msg = &thread.messages[1];
    assert_eq!(assistant_msg.content, "SELECT * FROM users WHERE active = true;");
    assert!(assistant_msg.is_streaming);

    thread.finish_streaming();
    assert!(!thread.messages[1].is_streaming);

    // Add tool call card
    let tool_card = ToolCallCard::new("explain_plan", "{\"table\": \"users\"}");
    assert!(tool_card.is_collapsed);
    thread.add_message(ChatMessage::user("run tool").with_tool_call(tool_card));
    assert_eq!(thread.messages.len(), 3);

    // Toggle card expansion
    thread.toggle_tool_card(2);
    assert!(!thread.messages[2].tool_call.as_ref().unwrap().is_collapsed);

    // Render thread
    let mut buf = Buffer::new(60, 12);
    let rect = buf.rect();
    let mut subview = buf.subview_mut(rect);
    thread.render(&mut subview, Color::BLACK);

    let mut text_found = false;
    for y in 0..12 {
        let mut line = String::new();
        for x in 0..50 {
            line.push_str(buf.get(x, y).unwrap().symbol.as_str());
        }
        if line.contains("Optimize this SQL") {
            text_found = true;
            break;
        }
    }
    assert!(text_found, "Chat thread should render user query");
}

#[test]
fn test_spinner_frames() {
    let s = Spinner::BRAILLE_ORBIT;
    assert_eq!(s.style.total_frames(), 10);
    assert_eq!(s.glyph_for_frame(0), '⠋');
    assert_eq!(s.glyph_for_frame(10), '⠋'); // Modulo check

    let micro = Spinner::MICRO_BLOCK;
    assert_eq!(micro.style.total_frames(), 4);
    assert_eq!(micro.glyph_for_frame(1), '▘');

    let bounce = Spinner::BOUNCING_BAR;
    assert_eq!(bounce.glyph_for_frame(0), '▏');
}
