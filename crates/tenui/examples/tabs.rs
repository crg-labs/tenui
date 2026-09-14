//! Tabbed layout with StatusBar — demonstrates TabBar, StatusBar, and immediate-mode layout.
//!
//! Run: `cargo run --example tabs`

use std::io;
use std::time::Duration;

use tenui::layout::{BorderStyle, draw_border_with_title};
use tenui::std_widgets::{StatusBar, StatusSegment, Tab, TabBar, ThemePalette};
use tenui::{Color, Rect, Terminal, TextOverflow};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let palette = ThemePalette::catppuccin_mocha();

    let mut tab_bar = TabBar::new(vec![
        Tab::new("Overview"),
        Tab::new("Logs").closable(),
        Tab::new("Settings").closable(),
    ]);
    tab_bar.select(0);

    loop {
        if crossterm::event::poll(Duration::from_millis(50))? {
            let ev = crossterm::event::read()?;
            if let crossterm::event::Event::Key(k) = ev {
                if k.kind != crossterm::event::KeyEventKind::Press {
                    continue;
                }
                match k.code {
                    crossterm::event::KeyCode::Char('q') | crossterm::event::KeyCode::Esc => break,
                    crossterm::event::KeyCode::Tab => tab_bar.select_next(),
                    crossterm::event::KeyCode::BackTab => tab_bar.select_prev(),
                    crossterm::event::KeyCode::Char('w') => {
                        if tab_bar.tabs.len() > 1 {
                            tab_bar.close(tab_bar.active);
                        }
                    }
                    _ => {}
                }
            }
        }

        term.draw(|canvas| {
            let w = canvas.width();
            let h = canvas.height();
            canvas.clear(Color::Reset);

            // Tab bar at top
            {
                let mut sv = canvas.subview_mut(Rect::new(0, 0, w, 1));
                tab_bar.render(&mut sv, &palette);
            }

            // Content area
            {
                let content_h = h.saturating_sub(3);
                let mut sv = canvas.subview_mut(Rect::new(0, 1, w, content_h));
                draw_border_with_title(
                    &mut sv,
                    BorderStyle::ROUNDED,
                    &tab_bar.tabs[tab_bar.active].label,
                    TextOverflow::Clip,
                    Color::Cyan,
                    Color::Reset,
                );

                let content = match tab_bar.active {
                    0 => "Welcome to the Overview tab.\n\nThis example demonstrates TabBar and StatusBar widgets.",
                    1 => "[2026-09-13 00:15:42] INFO  Server started on :8080\n[2026-09-13 00:15:43] DEBUG Connection pool initialized\n[2026-09-13 00:15:44] INFO  Health check: OK",
                    2 => "Theme: Dark\nFont size: 14\nAuto-save: enabled",
                    _ => "",
                };
                for (i, line) in content.lines().enumerate() {
                    sv.write_str_clipped(2, 2 + i as u16, line, Color::Reset, Color::Reset);
                }
            }

            // Status bar at bottom
            {
                let tab_name = &tab_bar.tabs[tab_bar.active].label;
                let status = StatusBar::new()
                    .left(StatusSegment::text(format!(" {} ", tab_name)).bold())
                    .center(StatusSegment::text("tenui v0.1.0"))
                    .right(StatusSegment::text("[Tab] switch  [w] close  [q] quit "));
                let mut sv = canvas.subview_mut(Rect::new(0, h - 1, w, 1));
                status.render(&mut sv, &palette);
            }
        })?;
    }

    Ok(())
}
