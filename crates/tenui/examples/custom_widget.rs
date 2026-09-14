//! How to build a custom widget — a simple analog clock face.
//!
//! Run: `cargo run --example custom_widget`

use std::{io, time::Duration};

use tenui::{CanvasSubviewMut, Color, Modifier, Rect, Terminal};

struct ClockWidget {
    hour: u8,
    minute: u8,
    second: u8,
}

impl ClockWidget {
    fn from_now() -> Self {
        let secs_since_midnight = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs()
            % 86400;
        Self {
            hour: (secs_since_midnight / 3600 % 12) as u8,
            minute: (secs_since_midnight / 60 % 60) as u8,
            second: (secs_since_midnight % 60) as u8,
        }
    }

    fn render(&self, sv: &mut CanvasSubviewMut<'_>) {
        let w = sv.width();
        let h = sv.height();
        let cx = w / 2;
        let cy = h / 2;
        let radius = cx.min(cy).saturating_sub(1);

        // Draw hour marks
        for i in 0..12 {
            let angle = (i as f64) * std::f64::consts::PI / 6.0 - std::f64::consts::FRAC_PI_2;
            let x = cx as f64 + (radius as f64 * 2.0) * angle.cos();
            let y = cy as f64 + radius as f64 * angle.sin();
            let ch = if i == 0 { '1' } else { char::from(b'0' + (i as u8 % 10)) };
            sv.set_char(
                x.round() as u16,
                y.round() as u16,
                ch,
                Color::Yellow,
                Color::Reset,
                Modifier::BOLD,
            );
        }

        // Draw hands
        let draw_hand = |sv: &mut CanvasSubviewMut<'_>, angle: f64, len: f64, ch: char, color: Color| {
            for step in 1..=(len as usize) {
                let t = step as f64 / len;
                let x = cx as f64 + (len * 2.0 * t) * angle.cos();
                let y = cy as f64 + (len * t) * angle.sin();
                let px = x.round() as u16;
                let py = y.round() as u16;
                if px < w && py < h {
                    sv.set_char(px, py, ch, color, Color::Reset, Modifier::empty());
                }
            }
        };

        let hour_angle =
            (self.hour as f64 + self.minute as f64 / 60.0) * std::f64::consts::PI / 6.0 - std::f64::consts::FRAC_PI_2;
        let min_angle = self.minute as f64 * std::f64::consts::PI / 30.0 - std::f64::consts::FRAC_PI_2;
        let sec_angle = self.second as f64 * std::f64::consts::PI / 30.0 - std::f64::consts::FRAC_PI_2;

        draw_hand(sv, hour_angle, radius as f64 * 0.5, '\u{2588}', Color::White);
        draw_hand(sv, min_angle, radius as f64 * 0.7, '\u{2593}', Color::Cyan);
        draw_hand(sv, sec_angle, radius as f64 * 0.9, '\u{00B7}', Color::Red);

        sv.set_char(cx, cy, '\u{25CF}', Color::White, Color::Reset, Modifier::empty());
    }
}

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;

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

            let clock = ClockWidget::from_now();
            let size = w.min(h * 2).min(40);
            let cx = (w.saturating_sub(size)) / 2;
            let cy = (h.saturating_sub(size / 2)) / 2;
            let mut sv = canvas.subview_mut(Rect::new(cx, cy, size, size / 2));
            clock.render(&mut sv);

            let time_str = format!("{:02}:{:02}:{:02}", clock.hour, clock.minute, clock.second);
            canvas.set_string((w - 8) / 2, h - 2, &time_str, Color::Cyan, Color::Reset, Modifier::BOLD);
            canvas.set_string(2, h - 1, "[q] quit", Color::DarkGray, Color::Reset, Modifier::empty());
        })?;

        std::thread::sleep(Duration::from_secs(1));
    }

    Ok(())
}
