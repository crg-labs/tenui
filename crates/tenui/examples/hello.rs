//! Minimal Tenui app — renders a greeting and waits for a keypress.
//!
//! Run: `cargo run --example hello`

use std::io;
use std::time::Duration;

use tenui::{Color, Modifier, Terminal};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;

    term.draw(|canvas| {
        let w = canvas.width();
        canvas.clear(Color::Reset);
        canvas.write_str_clipped(2, 1, "Hello from Tenui!", Color::Cyan, Color::Reset);
        canvas.write_str_clipped(2, 3, "A thin, refined TUI framework for Rust.", Color::Reset, Color::Reset);

        let quit_msg = "Press any key to exit.";
        let x = w.saturating_sub(quit_msg.len() as u16 + 2);
        canvas.set_string(x, canvas.height().saturating_sub(1), quit_msg, Color::DarkGray, Color::Reset, Modifier::ITALIC);
    })?;

    loop {
        if crossterm::event::poll(Duration::from_millis(100))? {
            let _ = crossterm::event::read()?;
            break;
        }
    }

    Ok(())
}
