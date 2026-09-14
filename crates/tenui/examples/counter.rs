//! Retained-mode reactive counter — demonstrates Signal, Two-Lane bus, and App reactor.
//!
//! Run: `cargo run --example counter`

use std::{io, rc::Rc, time::Duration};

use tenui::{
    Color, Modifier, Terminal,
    core::InputDemuxer,
    reactive::{Invalidation, Signal},
    runtime::App,
};

fn main() -> io::Result<()> {
    let term = Terminal::new()?;
    let mut app = App::new(term);

    let count = Signal::new(0i32);

    let root = app.root();
    let display = {
        let count = Rc::clone(&count);
        app.add_leaf(
            root,
            taffy::prelude::Style {
                size: taffy::prelude::Size {
                    width: taffy::prelude::percent(1.0),
                    height: taffy::prelude::length(3.0),
                },
                ..Default::default()
            },
            move |sv| {
                let val = count.get();
                let text = format!("  Count: {}  ", val);
                sv.set_string(0, 1, &text, Color::Cyan, Color::Reset, Modifier::BOLD);
            },
        )
    };
    count.bind_invalidation(display, Invalidation::PAINT, app.bus());

    let _help = app.add_leaf(
        root,
        taffy::prelude::Style {
            size: taffy::prelude::Size {
                width: taffy::prelude::percent(1.0),
                height: taffy::prelude::length(2.0),
            },
            ..Default::default()
        },
        move |sv| {
            sv.set_string(
                0,
                0,
                "  [+] increment   [-] decrement   [q] quit",
                Color::DarkGray,
                Color::Reset,
                Modifier::empty(),
            );
        },
    );

    app.render_frame()?;

    let mut demuxer = InputDemuxer::new();
    loop {
        if demuxer.poll(Duration::from_millis(50))? {
            let event = demuxer.read()?;
            if let Some(tenui::core::InputEvent::Key(key)) = event {
                if key.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match key.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => break,
                    crossterm::event::KeyCode::Char('+') | crossterm::event::KeyCode::Char('=') => {
                        count.set(count.get() + 1);
                    }
                    crossterm::event::KeyCode::Char('-') => {
                        count.set(count.get() - 1);
                    }
                    _ => {}
                }
            }
        }
        app.render_frame()?;
    }

    Ok(())
}
