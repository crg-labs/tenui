use std::io::Cursor;

use tenui::{
    core::{Color, Modifier, Terminal},
    layout::{StyleExt, TerminalLayoutExt},
    view,
};

#[test]
fn test_view_macro_expansion() {
    let output = Vec::new();
    let mut term = Terminal::with_writer(Cursor::new(output), 20, 5);

    term.draw_layout(|ui| {
        view!(ui => {
            row [grow: 1.0] {
                leaf [grow: 1.0] {
                    |subview| {
                        subview.set_string(0, 0, "MACRO", Color::Green, Color::Reset, Modifier::empty());
                    }
                };
            };
        });
    })
    .expect("macro expansion draw succeeds");

    let cell = term.front().get(0, 0).unwrap();
    assert_eq!(cell.symbol.as_str(), "M");
}
