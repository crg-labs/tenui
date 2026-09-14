# Tenui

A TUI framework

```toml
[dependencies]
tenui = "0.1"
```

```rust,no_run
use std::io;
use tenui::{Color, Modifier, Terminal};

fn main() -> io::Result<()> {
    let mut term = Terminal::new()?;
    term.draw(|canvas| {
        canvas.set_string(2, 1, "Hello, Tenui!", Color::Cyan, Color::Reset, Modifier::BOLD);
    })?;
    Ok(())
}
```

## What it does

Double-buffered differential rendering with SGR coalescing — only changed cells hit stdout. Two rendering modes:

- **Immediate** — `Terminal::draw` clears and redraws each frame. Simple.
- **Retained** — `App` reactor with `Signal<T>`, two-lane invalidation (PAINT vs LAYOUT), and Taffy flexbox. Only dirty regions recompute.

Default features give you core through widgets. Opt in to the rest:

```toml
tenui = { version = "0.1", features = ["tokio", "simd", "media"] }
```

## Examples

```bash
cargo run -p tenui --example hello
cargo run -p tenui --example counter
cargo run -p tenui --example dashboard
cargo run -p tenui --example tabs
cargo run -p tenui --example custom_widget
```

## Docs

- [Getting Started](docs/getting-started.md)
- [Concepts](docs/concepts.md) — rendering pipeline, immediate vs retained, sync vs async
- [Widgets](docs/widgets.md)

## License

MIT OR Apache-2.0
