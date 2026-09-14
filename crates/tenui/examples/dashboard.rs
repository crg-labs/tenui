//! Dashboard example — sparklines, bar chart, and progress bars in a split layout.
//!
//! Run: `cargo run --example dashboard`

use std::{io, time::Duration};

use tenui::{
    Color, Modifier, Rect, Terminal, TextOverflow,
    layout::{BorderStyle, draw_border_with_title},
};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let mut tick = 0u64;

    loop {
        if crossterm::event::poll(Duration::from_millis(0))? {
            let ev = crossterm::event::read()?;
            if let crossterm::event::Event::Key(k) = ev
                && k.kind == crossterm::event::KeyEventKind::Press
                && matches!(
                    k.code,
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc
                )
            {
                break;
            }
        }

        term.draw(|canvas| {
            let w = canvas.width();
            let h = canvas.height();
            canvas.clear(Color::Reset);

            canvas.set_string(2, 0, " Tenui Dashboard ", Color::Cyan, Color::Reset, Modifier::BOLD);

            let half_w = w / 2;
            let body_h = h.saturating_sub(2);

            // Left panel: sparkline
            {
                let mut sv = canvas.subview_mut(Rect::new(0, 1, half_w, body_h));
                draw_border_with_title(
                    &mut sv,
                    BorderStyle::ROUNDED,
                    "CPU Usage",
                    TextOverflow::Clip,
                    Color::Green,
                    Color::Reset,
                );
                let inner_w = half_w.saturating_sub(2);
                let inner_h = body_h.saturating_sub(2);
                for x in 0..inner_w {
                    let phase = (tick as f64 * 0.1 + x as f64 * 0.3).sin();
                    let bar_h = ((phase + 1.0) / 2.0 * inner_h as f64) as u16;
                    for y in 0..inner_h {
                        let ry = inner_h - 1 - y;
                        let ch = if y < bar_h { '|' } else { ' ' };
                        let color = if y < bar_h { Color::Green } else { Color::Reset };
                        sv.set_char(x + 1, ry + 1, ch, color, Color::Reset, Modifier::empty());
                    }
                }
            }

            // Right panel: progress bars
            {
                let mut sv = canvas.subview_mut(Rect::new(half_w, 1, w - half_w, body_h));
                draw_border_with_title(
                    &mut sv,
                    BorderStyle::ROUNDED,
                    "Services",
                    TextOverflow::Clip,
                    Color::Yellow,
                    Color::Reset,
                );
                let bars = [("API", 0.87), ("Database", 0.45), ("Cache", 0.92), ("Queue", 0.33)];
                let inner_w = (w - half_w).saturating_sub(4);
                for (i, (name, pct)) in bars.iter().enumerate() {
                    let y = 2 + i as u16 * 2;
                    let label = format!("{:<10} {:>3.0}%", name, pct * 100.0);
                    sv.set_string(2, y, &label, Color::Reset, Color::Reset, Modifier::empty());
                    let bar_w = (inner_w as f64 * pct) as u16;
                    for x in 0..inner_w {
                        let ch = if x < bar_w { '\u{2588}' } else { '\u{2591}' };
                        let color = if *pct > 0.8 {
                            Color::Green
                        } else if *pct > 0.5 {
                            Color::Yellow
                        } else {
                            Color::Red
                        };
                        sv.set_char(2 + x, y + 1, ch, color, Color::Reset, Modifier::empty());
                    }
                }
            }

            // Footer
            canvas.set_string(2, h - 1, "[q] quit", Color::DarkGray, Color::Reset, Modifier::empty());
        })?;

        tick += 1;
        std::thread::sleep(Duration::from_millis(100));
    }

    Ok(())
}
