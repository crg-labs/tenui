use tenui_core::{Buffer, Color, Modifier};
use tenui_layout::{Style, StyleExt};
use tenui_std::{
    Column, CommandItem, CommandPalette, ScrollArea, SmoothProgressBar, Sparkline, SplitOrientation, SplitPane,
    TenuiTestHarness, TextEditor, TextInput, TreeNode, TreeView, VirtualTable, downsample_to_ansi16,
    downsample_to_ansi256,
};

#[test]
fn test_cielab_perceptual_downsampling() {
    let red_ansi = downsample_to_ansi16(255, 0, 0);
    assert!(red_ansi == Color::LightRed || red_ansi == Color::Red);

    let green_ansi = downsample_to_ansi16(0, 255, 0);
    assert_eq!(green_ansi, Color::LightGreen);

    let black_ansi = downsample_to_ansi16(0, 0, 0);
    assert_eq!(black_ansi, Color::Black);

    // 256 color cube downsampling
    let blue_256 = downsample_to_ansi256(0, 0, 255);
    match blue_256 {
        Color::Indexed(idx) => assert!(idx >= 16, "Must be indexed color"),
        _ => panic!("Expected indexed color"),
    }
}

#[test]
fn test_tree_view_expansion_and_navigation() {
    let child1 = TreeNode::new(2, "child1.txt", ());
    let child2 = TreeNode::new(3, "child2.txt", ());
    let root = TreeNode::new(1, "src", ()).with_children(vec![child1, child2]);

    let mut tree = TreeView::new(vec![root]);
    assert_eq!(tree.selected_id, Some(1));

    // Initially collapsed: toggle expansion
    tree.toggle_selected();

    // Now expanded: select_next should navigate to child1
    tree.select_next();
    assert_eq!(tree.selected_id, Some(2));

    let mut buf = Buffer::new(20, 5);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);
    tree.render(&mut subview);

    // Row 0 has "▼ src"
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "▼");
}

#[test]
fn test_text_editor_multiline() {
    let mut editor = TextEditor::from_text("Hello\nWorld");
    assert_eq!(editor.text(), "Hello\nWorld");

    // Navigate to end of first line and insert
    editor.buffer_mut().move_home(false);
    editor.buffer_mut().move_end(false);
    editor.insert_char('!');
    assert!(editor.text().starts_with("Hello!"));

    // Undo restores original
    editor.buffer_mut().undo();
    assert!(editor.text().starts_with("Hello\n"));
}

#[test]
fn test_split_pane_rendering() {
    let pane = SplitPane::new(SplitOrientation::Horizontal, 0.5);
    let mut buf = Buffer::new(10, 3);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);

    pane.render(
        &mut subview,
        |left| {
            left.set_string(0, 0, "L", Color::Reset, Color::Reset, Modifier::empty());
        },
        |right| {
            right.set_string(0, 0, "R", Color::Reset, Color::Reset, Modifier::empty());
        },
    );

    // Column 5 should be the divider '│'
    assert_eq!(buf.get(5, 0).unwrap().symbol.as_str(), "│");
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "L");
    assert_eq!(buf.get(6, 0).unwrap().symbol.as_str(), "R");
}

#[test]
fn test_scroll_area_fractional_thumbs() {
    // 100 rows of content inside a 10-row viewport
    let scroll = ScrollArea::new(100, 0);
    let mut buf = Buffer::new(10, 10);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);

    scroll.render_scrollbar(&mut subview);

    // Track exists on rightmost column (x = 9)
    assert!(buf.get(9, 0).is_some());
}

#[test]
fn test_smooth_progress_bar() {
    let pb = SmoothProgressBar::new(0.5); // 50%
    let mut buf = Buffer::new(10, 1);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);

    pb.render(&mut subview, Color::Green, Color::Reset);

    // At 50% of 10 cells, cells 0..4 should be full blocks '█'
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "█");
    assert_eq!(buf.get(4, 0).unwrap().symbol.as_str(), "█");
    assert_eq!(buf.get(5, 0).unwrap().symbol.as_str(), " ");
}

#[test]
fn test_sparkline_chart() {
    let data = vec![1, 3, 5, 8];
    let spark = Sparkline::new(&data);
    let mut buf = Buffer::new(4, 1);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);

    spark.render(&mut subview, Color::Cyan, Color::Reset);

    // Last element is maximum -> should be '█'
    assert_eq!(buf.get(3, 0).unwrap().symbol.as_str(), "█");
}

#[test]
fn test_virtual_table_zero_alloc_large_dataset() {
    struct Item {
        id: usize,
        name: String,
    }

    let dataset: Vec<Item> = (0..10_000)
        .map(|i| Item {
            id: i,
            name: format!("Item #{}", i),
        })
        .collect();

    let columns = vec![
        Column::new("ID", 6, |item: &Item| item.id.to_string()),
        Column::new("Name", 12, |item: &Item| item.name.clone()),
    ];

    let mut table = VirtualTable::new(&dataset, columns);

    let mut buf = Buffer::new(30, 5);
    let full = buf.rect();
    let mut subview = buf.subview_mut(full);

    table.render(&mut subview);

    let cell_char = buf.get(7, 2).unwrap().symbol.as_str();
    assert_eq!(cell_char, "I");

    table.scroll_down(5, 3);
    buf.clear();
    let mut subview = buf.subview_mut(full);
    table.render(&mut subview);

    let row_text: String = (7..14).map(|x| buf.get(x, 2).unwrap().symbol.as_str()).collect();
    assert_eq!(row_text, "Item #5");
}

#[test]
fn test_text_input_editing() {
    let mut input = TextInput::new();
    input.insert_str("Term");
    assert_eq!(input.content, "Term");
    assert_eq!(input.cursor, 4);

    input.delete_backward();
    assert_eq!(input.content, "Ter");
    assert_eq!(input.cursor, 3);

    input.insert('m');
    assert_eq!(input.content, "Term");
}

#[test]
fn test_command_palette_fuzzy_search() {
    let items = vec![
        CommandItem {
            id: "file.open".to_string(),
            title: "Open File".to_string(),
            category: "File".to_string(),
        },
        CommandItem {
            id: "app.quit".to_string(),
            title: "Quit Application".to_string(),
            category: "App".to_string(),
        },
        CommandItem {
            id: "editor.undo".to_string(),
            title: "Undo".to_string(),
            category: "Edit".to_string(),
        },
    ];

    let mut palette = CommandPalette::new(items);
    palette.query.insert_str("quit");

    let filtered = palette.filtered_items();
    assert_eq!(filtered.len(), 1);
    assert_eq!(filtered[0].id, "app.quit");
}

#[test]
fn test_test_harness_assertions() {
    let mut harness = TenuiTestHarness::new(20, 5);

    harness.draw_layout(|ui| {
        ui.row(Style::default(), |row| {
            row.leaf(Style::default().grow(1.0), |subview| {
                subview.set_string(0, 0, "TERMINUS", Color::Green, Color::Reset, Modifier::BOLD);
            });
        });
    });

    harness.assert_text(0, 0, "TERMINUS");
    harness.assert_symbol(0, 0, "T");
    harness.assert_symbol(7, 0, "S");
}

#[test]
fn test_text_view_wrapping_and_horizontal_scroll() {
    use tenui_core::Rect;
    use tenui_std::TextView;

    let text = "Line 1 is very long and has many words\nLine 2 is second\nLine 3 is third";
    let mut view = TextView::new(text).with_line_numbers(false).with_scrollbars(false);

    let mut buf = Buffer::new(15, 6);
    let mut subview = buf.subview_mut(Rect::new(0, 0, 15, 6));

    // 1. Scrolled view
    view.render(&mut subview);
    // Line 1 should be clipped at column 15
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "L");
    assert_eq!(buf.get(1, 0).unwrap().symbol.as_str(), "i");

    // 2. Horizontal scroll by 7 columns
    view.scroll_right(7);
    buf.clear();
    let mut subview2 = buf.subview_mut(Rect::new(0, 0, 15, 6));
    view.render(&mut subview2);
    // 'Line 1 ' skipped, next is 'is very long'
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "i");
    assert_eq!(buf.get(1, 0).unwrap().symbol.as_str(), "s");

    // 3. Toggle wrap mode
    view.toggle_wrap();
    assert!(view.wrap);
    buf.clear();
    let mut subview3 = buf.subview_mut(Rect::new(0, 0, 15, 6));
    view.render(&mut subview3);
    // First line wrapped into row 0 and row 1
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "L");
}
