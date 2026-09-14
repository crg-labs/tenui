//! Glow effect showcase — demonstrates `GlowEffect`, `BloomStack`, `TextEffect::Glow`,
//! and the `view!` macro.
//!
//! Three panels are laid out with `view!`: a neon-cyan outer glow, a pulsing inner glow,
//! and a multi-layer bloom stack. A per-glyph text glow label is rendered below the panels.
//! Each panel renders its content first, then the glow effect is composited over the
//! surrounding/interior cells.
//!
//! Run: `cargo run --example glow`

use std::{
    io,
    time::{Duration, Instant},
};

use tenui::{
    layout::{StyleExt, TerminalLayoutExt},
    prelude::*,
    vfx::{
        glow::{BloomStack, GlowEffect, GlowFalloff},
        text::{TextEffect, TextOpticsCompositor},
    },
    view,
};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    let start = Instant::now();

    loop {
        let t = start.elapsed().as_secs_f32();
        let (w, h) = term.size();

        term.draw(|canvas| {
            canvas.clear(Color::Rgb(10, 10, 20));

            let mut buf = Buffer::new(w, h);
            {
                let mut ui_term = Terminal::with_writer(io::Cursor::new(Vec::new()), w, h);
                ui_term
                    .draw_layout_with_bg(Color::Rgb(10, 10, 20), |ui| {
                        view!(ui => {
                            row [grow: 1.0] {
                                column [grow: 1.0, margin: 2.0] {
                                    leaf [grow: 1.0] {
                                        |subview| {
                                            subview.clear(Color::Rgb(15, 15, 30));
                                            subview.set_string(
                                                2, 1,
                                                "NEON CYAN",
                                                Color::Rgb(0, 255, 255),
                                                Color::Rgb(15, 15, 30),
                                                Modifier::BOLD,
                                            );
                                            subview.write_str_clipped(
                                                2, 3,
                                                "Outer glow · Gaussian falloff",
                                                Color::Rgb(100, 180, 200),
                                                Color::Rgb(15, 15, 30),
                                            );
                                        }
                                    };
                                };
                                column [grow: 1.0, margin: 2.0] {
                                    leaf [grow: 1.0] {
                                        |subview| {
                                            subview.clear(Color::Rgb(15, 15, 30));
                                            subview.set_string(
                                                2, 1,
                                                "PULSE RED",
                                                Color::Rgb(255, 80, 80),
                                                Color::Rgb(15, 15, 30),
                                                Modifier::BOLD,
                                            );
                                            subview.write_str_clipped(
                                                2, 3,
                                                "Inner glow · 2 Hz pulse",
                                                Color::Rgb(200, 120, 120),
                                                Color::Rgb(15, 15, 30),
                                            );
                                        }
                                    };
                                };
                                column [grow: 1.0, margin: 2.0] {
                                    leaf [grow: 1.0] {
                                        |subview| {
                                            subview.clear(Color::Rgb(15, 15, 30));
                                            subview.set_string(
                                                2, 1,
                                                "BLOOM STACK",
                                                Color::Rgb(180, 100, 255),
                                                Color::Rgb(15, 15, 30),
                                                Modifier::BOLD,
                                            );
                                            subview.write_str_clipped(
                                                2, 3,
                                                "Two-layer neon bloom",
                                                Color::Rgb(160, 130, 200),
                                                Color::Rgb(15, 15, 30),
                                            );
                                        }
                                    };
                                };
                            };
                        });
                    })
                    .ok();

                for y in 0..h {
                    for x in 0..w {
                        if let Some(src) = ui_term.front().get(x, y)
                            && let Some(dst) = buf.get_mut(x, y)
                        {
                            *dst = *src;
                        }
                    }
                }
            }

            let col_w = w / 3;
            let margin = 2u16;
            let panel_h = h.saturating_sub(margin * 2);

            let panel1 = Rect::new(margin, margin, col_w.saturating_sub(margin * 2), panel_h);
            let panel2 = Rect::new(col_w + margin, margin, col_w.saturating_sub(margin * 2), panel_h);
            let panel3 = Rect::new(col_w * 2 + margin, margin, col_w.saturating_sub(margin * 2), panel_h);

            let full = Rect::new(0, 0, w, h);

            // Pass 1: neon cyan outer glow
            {
                let mut sv = buf.subview_mut(full);
                GlowEffect::NEON_CYAN
                    .with_outer_radius(3.5)
                    .with_intensity(0.65)
                    .render(&mut sv, panel1, t);
            }

            // Pass 2: pulsing red inner glow
            {
                let mut sv = buf.subview_mut(full);
                GlowEffect::new(Color::Rgb(255, 60, 60))
                    .with_inner_radius(2.5)
                    .with_outer_radius(2.0)
                    .with_intensity(0.5)
                    .with_falloff(GlowFalloff::Exponential)
                    .with_pulse(2.0)
                    .render(&mut sv, panel2, t);
            }

            // Pass 3: multi-layer purple bloom
            {
                let mut sv = buf.subview_mut(full);
                BloomStack::neon(Color::Rgb(160, 80, 255)).render(&mut sv, panel3, t);
            }

            // Pass 4: per-glyph text glow centered below the panels
            {
                let text_glow_label = "TEXT GLOW EFFECT";
                let gx = w.saturating_sub(text_glow_label.len() as u16) / 2;
                let gy = h.saturating_sub(3);
                let mut sv = buf.subview_mut(full);
                TextOpticsCompositor::render_text(
                    &mut sv,
                    gx,
                    gy,
                    text_glow_label,
                    Color::Rgb(0, 255, 200),
                    TextEffect::Glow {
                        glow_color: Color::Rgb(0, 255, 200),
                        radius: 2.5,
                        intensity: 0.7,
                    },
                    t,
                );
            }

            // Title bar
            let title = " TENUI GLOW EFFECTS ";
            let tx = w.saturating_sub(title.len() as u16) / 2;
            buf.set_string(
                tx,
                0,
                title,
                Color::Rgb(200, 200, 255),
                Color::Rgb(10, 10, 20),
                Modifier::BOLD,
            );

            // Footer
            let footer = " press any key to exit ";
            let fx = w.saturating_sub(footer.len() as u16) / 2;
            buf.set_string(
                fx,
                h.saturating_sub(1),
                footer,
                Color::Rgb(80, 80, 100),
                Color::Rgb(10, 10, 20),
                Modifier::empty(),
            );

            // Blit composed buffer to canvas
            for y in 0..h {
                for x in 0..w {
                    if let Some(src) = buf.get(x, y)
                        && let Some(dst) = canvas.get_cell_mut(x, y)
                    {
                        *dst = *src;
                    }
                }
            }
        })?;

        if crossterm::event::poll(Duration::from_millis(16))? {
            let _ = crossterm::event::read()?;
            break;
        }
    }

    Ok(())
}
