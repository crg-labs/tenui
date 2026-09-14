# Getting Started with Tenui

This guide takes you from an empty Rust binary to a running, interactive terminal application with mouse support, styled layouts, and responsive event handling.

---

## 1. Project Setup

Add `tenui` to your `Cargo.toml`:

```toml
[dependencies]
tenui = "0.1"
```

Tenui provides a unified facade and prelude covering everything needed for immediate-mode terminal development:

```rust
use tenui::prelude::*;
use std::io;
use std::time::Duration;
```

---

## 2. The Minimal Immediate-Mode Loop

At the center of an immediate-mode Tenui application is [`Terminal`](https://docs.rs/tenui-core/latest/tenui_core/struct.Terminal.html). When instantiated via `Terminal::new()`, it automatically:
- Enters terminal raw mode.
- Switches to the terminal alternate screen buffer (`\x1b[?1049h`).
- Enables synchronized output (`\x1b[?2026h`) and mouse capture.
- Installs an RAII panic guard that guarantees terminal restoration even if your code panics.

Here is a minimal, complete application:

```rust,no_run
use tenui::prelude::*;
use std::io;
use std::time::Duration;

fn main() -> io::Result<()> {
    // 1. Initialize terminal in raw mode on the alternate screen
    let mut terminal = Terminal::new()?;

    // 2. Main event loop
    loop {
        // 3. Render the frame onto the back buffer
        terminal.draw(|subview| {
            subview.set_string(
                2,
                1,
                "Hello, Tenui!",
                Color::Cyan,
                Color::Reset,
                Modifier::BOLD,
            );
            subview.set_string(
                2,
                3,
                "Press 'q' or Esc to quit.",
                Color::DarkGray,
                Color::Reset,
                Modifier::empty(),
            );
        })?;

        // 4. Poll for input events with a timeout (e.g. 50ms)
        if let Ok(Some(event)) = terminal.poll_event(Duration::from_millis(50)) {
            match event {
                InputEvent::Key(key) => {
                    use crossterm::event::KeyCode;
                    if key.code == KeyCode::Char('q') || key.code == KeyCode::Esc {
                        break;
                    }
                }
                _ => {}
            }
        }
    }

    // When `terminal` drops, it restores the normal screen and raw mode.
    Ok(())
}
```

---

## 3. Understanding `terminal.draw` and Subviews

When you call `terminal.draw(|subview| { ... })`:
1. Tenui verifies the current terminal dimensions. If a window resize occurred, it automatically re-allocates internal front and back buffers.
2. The back buffer is cleared.
3. Your closure receives a mutable [`CanvasSubviewMut`](https://docs.rs/tenui-core/latest/tenui_core/struct.CanvasSubviewMut.html) representing the bounded printable area.
4. When the closure finishes, the internal differential engine (`SgrCoalescer`) compares the back buffer with the front buffer, emitting ANSI escape sequences **only for the cells that changed**.
5. The front and back buffer pointers are swapped in zero-allocation constant time.

### Subview Primitives

`CanvasSubviewMut` provides direct, boundary-checked cell manipulation:

```rust,no_run
terminal.draw(|subview| {
    // Write a single character with styling
    subview.set_char(0, 0, '★', Color::Yellow, Color::Reset, Modifier::BOLD);

    // Write a string with automatic clipping to subview boundaries
    subview.set_string(2, 0, "Starred Item", Color::White, Color::Reset, Modifier::empty());

    // Slice into a nested bounded sub-region
    let mut inner = subview.subview_mut(Rect::new(2, 2, 20, 5));
    inner.set_string(0, 0, "Inside nested box", Color::Green, Color::Reset, Modifier::ITALIC);
})?;
```

---

## 4. Adding Flexbox Layouts with `tenui-layout`

Drawing manual cell coordinates is fine for simple scripts, but modern applications require responsive, fluid layouts. Tenui integrates CSS Flexbox via [`tenui-layout`](https://docs.rs/tenui-layout):

```rust,no_run
use tenui::prelude::*;
use tenui_layout::{BorderStyle, Style, StyleExt, TerminalLayoutExt};

terminal.draw_layout(|ui| {
    // Root row container
    ui.row(Style::default().flex_row(), |ui| {
        // Left sidebar (fixed 20 cells wide)
        ui.block(
            Style::default().width_cells(20.0),
            Some("Sidebar"),
            BorderStyle::ROUNDED,
            Color::Cyan,
            Color::Reset,
            |ui| {
                ui.text(Style::default(), "• Dashboard", Color::White, Color::Reset, Modifier::empty());
                ui.text(Style::default(), "• Settings", Color::DarkGray, Color::Reset, Modifier::empty());
            },
        );

        // Main content area (grows to fill remaining width)
        ui.block(
            Style::default().grow(1.0),
            Some("Main Content"),
            BorderStyle::ROUNDED,
            Color::Green,
            Color::Reset,
            |ui| {
                ui.text(Style::default(), "Welcome to the application!", Color::White, Color::Reset, Modifier::BOLD);
            },
        );
    });
})?;
```

---

## 5. Handling Mouse Events

Tenui supports high-precision SGR mouse tracking:
- **Left / Right / Middle click**
- **Mouse dragging**
- **Vertical and horizontal scroll wheel**
- **Cursor position tracking**

```rust,no_run
use crossterm::event::{MouseButton, MouseEventKind};

if let Ok(Some(event)) = terminal.poll_event(Duration::from_millis(16)) {
    match event {
        InputEvent::Mouse(mouse) => match mouse.kind {
            MouseEventKind::Down(MouseButton::Left) => {
                let (click_x, click_y) = (mouse.column, mouse.row);
                // Check if user clicked within an interactive region
            }
            MouseEventKind::ScrollDown => {
                // Scroll down in active view
            }
            MouseEventKind::ScrollUp => {
                // Scroll up in active view
            }
            _ => {}
        },
        _ => {}
    }
}
```

---

## 6. Next Steps

- Explore the core rendering pipeline in [**Framework Concepts**](concepts.md).
- Learn how standard widgets (tables, code blocks, chat threads) are built in [**Widget Catalog Tour**](widgets.md).
- Study the real-world multi-file application in the standalone **tenui-chat** project (built on this framework).
