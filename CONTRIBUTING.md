# Contributing to Tenui

Thank you for your interest in contributing to Tenui! This document covers the
basics for getting started.

## Prerequisites

- Rust **1.86+** (edition 2024)
- A terminal emulator that supports the alternate screen

## Building

```sh
cargo build --workspace
```

## Testing

```sh
cargo test --workspace
```

## Code style

The project uses `rustfmt` with the settings in `rustfmt.toml`. Before
submitting a PR, run:

```sh
cargo fmt --all
```

Clippy must also pass with no warnings:

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

## Architecture

Tenui is split into ~23 focused crates under `crates/`. The dependency graph
flows downward:

- **tenui-core** — buffer, cell, color, input, terminal guard
- **tenui-layout** — constraint solver
- **tenui-reactive** — signals, two-lane invalidation bus, reactive tree
- **tenui-compositor** — layer composition and z-ordering
- **tenui-simd** — SIMD-accelerated differential buffer scanning
- **tenui-std** — standard widget library (charts, forms, markdown, syntax, etc.)
- **tenui** — umbrella re-export crate

Other crates (anim, vfx, dnd, text, session, sound, wgpu, collab, media, math,
hid, chaos, runtime, ext, macros, virt) provide optional capabilities.

## Commit messages

Write short, imperative-mood summaries. Prefix with the crate name when the
change is scoped to one crate:

```
fix(tenui-std): prevent heading level overflow in markdown renderer
feat(tenui-core): add tracing facade
```

## Pull requests

- One logical change per PR.
- Include a short description of *why*, not just *what*.
- All CI checks (build, test, clippy, fmt, doc, cargo-deny) must pass.

## License

By contributing you agree that your contributions will be licensed under the
same terms as the project: MIT OR Apache-2.0.
