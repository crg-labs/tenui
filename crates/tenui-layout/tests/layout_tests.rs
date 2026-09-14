use std::io::Cursor;

use tenui_core::{Color, Modifier, Terminal};
use tenui_layout::{
    AlignItems, BorderStyle, Dimension, JustifyContent, LengthPercentage, Style, StyleExt, TerminalLayoutExt,
};

#[test]
fn test_flex_row_distribution() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 20, 4);

    term.draw_layout(|ui| {
        ui.row(Style::default(), |row| {
            row.leaf(Style::default().grow(1.0), |subview| {
                assert_eq!(subview.width(), 10);
                assert_eq!(subview.height(), 4);
                subview.set_string(0, 0, "LEFT", Color::Reset, Color::Reset, Modifier::empty());
            });
            row.leaf(Style::default().grow(1.0), |subview| {
                assert_eq!(subview.width(), 10);
                assert_eq!(subview.height(), 4);
                subview.set_string(0, 0, "RIGHT", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
    })
    .expect("draw_layout succeeds");

    // Verify cell locations in front buffer
    let front = term.front();
    assert_eq!(front.get(0, 0).unwrap().symbol.as_str(), "L");
    assert_eq!(front.get(3, 0).unwrap().symbol.as_str(), "T");
    assert_eq!(front.get(10, 0).unwrap().symbol.as_str(), "R");
    assert_eq!(front.get(14, 0).unwrap().symbol.as_str(), "T");
}

#[test]
fn test_flex_column_distribution() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 10, 10);

    term.draw_layout(|ui| {
        ui.column(Style::default(), |col| {
            col.leaf(Style::default().grow(1.0), |subview| {
                assert_eq!(subview.height(), 5);
                subview.set_string(0, 0, "TOP", Color::Reset, Color::Reset, Modifier::empty());
            });
            col.leaf(Style::default().grow(1.0), |subview| {
                assert_eq!(subview.height(), 5);
                subview.set_string(0, 0, "BOTTOM", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
    })
    .expect("draw_layout succeeds");

    let front = term.front();
    assert_eq!(front.get(0, 0).unwrap().symbol.as_str(), "T");
    assert_eq!(front.get(0, 5).unwrap().symbol.as_str(), "B");
}

#[test]
fn test_block_with_border_and_title() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 14, 6);

    term.draw_layout(|ui| {
        ui.block(
            Style::default(),
            Some("Box"),
            BorderStyle::ROUNDED,
            Color::Cyan,
            Color::Reset,
            |inner| {
                inner.leaf(Style::default().grow(1.0), |subview| {
                    subview.set_string(0, 0, "Hi", Color::White, Color::Reset, Modifier::empty());
                });
            },
        );
    })
    .expect("draw_layout succeeds");

    let front = term.front();
    // Rounded top-left corner
    assert_eq!(front.get(0, 0).unwrap().symbol.as_str(), "╭");
    // Rounded top-right corner
    assert_eq!(front.get(13, 0).unwrap().symbol.as_str(), "╮");
    // Title " Box " should be at x=2..6
    assert_eq!(front.get(2, 0).unwrap().symbol.as_str(), " ");
    assert_eq!(front.get(3, 0).unwrap().symbol.as_str(), "B");
    assert_eq!(front.get(4, 0).unwrap().symbol.as_str(), "o");
    assert_eq!(front.get(5, 0).unwrap().symbol.as_str(), "x");
    assert_eq!(front.get(6, 0).unwrap().symbol.as_str(), " ");
}

#[test]
fn test_css_grid_distribution() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 20, 10);

    term.draw_layout(|ui| {
        ui.grid(Style::default().grid_columns(2).grid_rows(2), |grid| {
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 5);
                sub.set_string(0, 0, "C1", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 5);
                sub.set_string(0, 0, "C2", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 5);
                sub.set_string(0, 0, "C3", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 5);
                sub.set_string(0, 0, "C4", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
    })
    .expect("draw_layout succeeds");

    let front = term.front();
    // Top-left cell (0, 0)
    assert_eq!(front.get(0, 0).unwrap().symbol.as_str(), "C");
    assert_eq!(front.get(1, 0).unwrap().symbol.as_str(), "1");
    // Top-right cell (10, 0)
    assert_eq!(front.get(10, 0).unwrap().symbol.as_str(), "C");
    assert_eq!(front.get(11, 0).unwrap().symbol.as_str(), "2");
    // Bottom-left cell (0, 5)
    assert_eq!(front.get(0, 5).unwrap().symbol.as_str(), "C");
    assert_eq!(front.get(1, 5).unwrap().symbol.as_str(), "3");
    // Bottom-right cell (10, 5)
    assert_eq!(front.get(10, 5).unwrap().symbol.as_str(), "C");
    assert_eq!(front.get(11, 5).unwrap().symbol.as_str(), "4");
}

#[test]
fn test_css_grid_column_span() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 30, 8);

    term.draw_layout(|ui| {
        ui.grid(Style::default().grid_columns(3).grid_rows(2), |grid| {
            // Header spanning all 3 columns
            grid.leaf(Style::default().grid_col_span(3), |sub| {
                assert_eq!(sub.width(), 30);
                assert_eq!(sub.height(), 4);
                sub.set_string(0, 0, "SPAN3", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 4);
                sub.set_string(0, 0, "A", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 4);
                sub.set_string(0, 0, "B", Color::Reset, Color::Reset, Modifier::empty());
            });
            grid.leaf(Style::default(), |sub| {
                assert_eq!(sub.width(), 10);
                assert_eq!(sub.height(), 4);
                sub.set_string(0, 0, "C", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
    })
    .expect("draw_layout succeeds");

    let front = term.front();
    assert_eq!(front.get(0, 0).unwrap().symbol.as_str(), "S");
    assert_eq!(front.get(0, 4).unwrap().symbol.as_str(), "A");
    assert_eq!(front.get(10, 4).unwrap().symbol.as_str(), "B");
    assert_eq!(front.get(20, 4).unwrap().symbol.as_str(), "C");
}

#[test]
fn test_style_ext_intrinsic_and_safe_center() {
    let s = Style::default().safe_center().min_content().gap_cells(2.0, 1.0);

    assert_eq!(s.align_items, Some(AlignItems::SAFE_CENTER));
    assert_eq!(s.justify_content, Some(JustifyContent::SAFE_CENTER));
    assert_eq!(s.size.width, Dimension::min_content());
    assert_eq!(s.size.height, Dimension::min_content());
    assert_eq!(s.gap.width, LengthPercentage::length(2.0));
    assert_eq!(s.gap.height, LengthPercentage::length(1.0));
}
