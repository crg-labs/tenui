# Tenui Architecture & Core Concepts

Tenui is designed around mechanical sympathy: every abstraction aligns with modern CPU cache hierarchies, terminal emulator escape parsers, and synchronous frame pacing.

---

## 1. The Rendering Pipeline

Traditional terminal libraries either redraw the full screen on every frame (causing visible flickering and heavy I/O overhead) or perform complex string diffing. Tenui uses a double-buffered differential rendering pipeline:

```text
       ┌────────────────────────┐
       │ Application Draw Loop  │
       └───────────┬────────────┘
                   │ Renders into
                   ▼
       ┌────────────────────────┐
       │   Back Buffer (Grid)   │
       └───────────┬────────────┘
                   │
                   ├─────────────────────────┐
                   ▼                         ▼
         [Scalar Diff Engine]     [SIMD Vector Scanner]
         (Terminals <= Full HD)   (AVX2 / NEON for 4K/8K)
                   │                         │
                   └───────────┬─────────────┘
                               │ Identifies modified cell runs
                               ▼
       ┌────────────────────────────────────────────────────────┐
       │                  SgrCoalescer Engine                   │
       │  • Minimizes cursor movement (CUP \x1b[y;xH)           │
       │  • Coalesces foreground & background colors            │
       │  • Flushes contiguous styled text runs                 │
       └───────────────────────┬────────────────────────────────┘
                               │ Emits compact ANSI byte stream
                               ▼
       ┌────────────────────────────────────────────────────────┐
       │               Terminal Stdout Device                   │
       │  (Wrapped in \x1b[?2026h synchronized output block)    │
       └───────────────────────┬────────────────────────────────┘
                               │
                   Buffer Pointers Swapped
                               │
       ┌───────────────────────┴┐
       │  Front Buffer (Grid)   │ (Matches visible terminal screen)
       └────────────────────────┘
```

### Why It Is Fast

1. **8-Byte Compact Cell Layout**: Each cell in a Tenui [`Buffer`](https://docs.rs/tenui-core/latest/tenui_core/struct.Buffer.html) fits in a tight 8-byte union with an extended grapheme cluster (EGC) intern pool. Cells reside contiguously in memory, fitting cleanly in L1/L2 CPU cache lines.
2. **Differential SGR Coalescing**: When traversing dirty cells, [`SgrCoalescer`](https://docs.rs/tenui-core/latest/tenui_core/struct.SgrCoalescer.html) tracks active foreground color, background color, and modifier bitmasks. If adjacent changed cells share styles, no intermediate escape codes are emitted.
3. **Synchronized Output**: Frames are framed within `\x1b[?2026h` (begin synchronized update) and `\x1b[?2026l` (end synchronized update). Terminal emulators that support this modern standard update their display in a single atomic screen refresh, completely eliminating tearing.
4. **SIMD Vectorization (`tenui-simd`)**: For 4K and 8K terminal windows (which can contain 50,000+ cells), `SimdDiffScanner` packs cells into 256-bit vector lanes and uses AVX2 / AVX-512 / NEON vector instructions to scan hundreds of cells per CPU cycle.

---

## 2. Immediate Mode vs. Retained Mode

Tenui supports two distinct programming paradigms:

| Metric | Immediate Mode (`Terminal::draw`) | Retained Mode (`tenui-runtime::App`) |
| :--- | :--- | :--- |
| **State Ownership** | Application owns all state; UI re-declared every frame. | State bound to a persistent component tree and signals. |
| **Mental Model** | Simple, linear frame loop (like Dear ImGui). | Reactive graph (like SolidJS or Flutter). |
| **Layout Cost** | Taffy layout calculated during each frame render. | Taffy layout computed only when `Invalidation::LAYOUT` is emitted. |
| **Ideal For** | CLI utilities, single-purpose tools, streaming chat shells. | Complex multi-panel IDEs, real-time monitors, data dashboards. |

### The Two-Lane Invalidation Engine

In retained mode (`tenui-runtime` and `tenui-reactive`), every state mutation is categorized into one of two lanes:

- **Paint Lane (`Invalidation::PAINT`)**: Fired when visual attributes change (e.g. background color, text color, or new text of identical grapheme width). Taffy layout reflow is **completely skipped**, giving zero-reflow repaints.
- **Layout Lane (`Invalidation::LAYOUT`)**: Fired when dimensions, margins, padding, or text column width change. Triggers localized or full flexbox reflow.

---

## 3. The Layered Architecture

Tenui is structured as a stack of specialized crates allowing progressive disclosure:

```text
Layer 7: Standard Library (tenui-std)
  └── Syntax highlighting, Myers diffs, Markdown, VirtualTable, Forms, PTY
Layer 6: Extensibility & Plugins (tenui-ext)
  └── WebAssembly (WIT) component plugins, Tower middleware, scripting bridges
Layer 5: Resilience & Boundaries (tenui-std, tenui-chaos)
  └── Subprocess crash isolation, PTY fuzzer, Unicode BiDi reordering
Layer 4: Extended Protocols (tenui-core)
  └── OSC 11 background query, OSC 8 hyperlinks, IME cursor sync, DevTools
Layer 3: Sub-Cell Kinetics (tenui-anim)
  └── Analytical spring dynamics, 1/8th Unicode micro-stepping, ActiveTicker
Layer 2: Off-Screen Spatial Compositor (tenui-compositor)
  └── Occlusion culling, WICG spatial navigation, Braille & half-block canvases
Layer 1: Two-Lane Invalidation Engine (tenui-reactive)
  └── Fine-grained Signal<T>, transactional batching, paint vs. layout lanes
Layer 0: Pure-Rust Immediate-Mode Flexbox (tenui-layout)
  └── Taffy integration, immediate UI builder, border primitives
Milestone 0: The Bare Metal Core (tenui-core)
  └── 8-byte Cell, Buffer, differential SgrCoalescer, RAII panic guard
```

Each layer can be consumed independently without pulling in upper layers. For example, a command-line tool can depend exclusively on `tenui-core` for bare-metal differential rendering with zero transitive dependencies.

---

## 4. Sync vs. Async

Tenui's rendering pipeline is synchronous by design — `Terminal::draw` and `App::render_frame` block the calling thread until the frame is flushed to stdout. This keeps the hot path allocation-free and deterministic, which matters for frame pacing at 60+ fps.

### When sync is enough

Most TUI applications are event-driven: wait for input, update state, redraw. The standard pattern is a polling loop:

```rust
loop {
    if demuxer.poll(Duration::from_millis(50))? {
        let event = demuxer.read()?;
        // handle event, mutate state
    }
    app.render_frame()?;
}
```

This works well for interactive tools, dashboards, editors, and anything whose state changes only in response to user input or a timer tick. No async runtime is needed.

### When you need async

Some applications must react to external data sources — network responses, file watchers, database streams, LLM token streams — that arrive on their own schedule. Spawning a background thread and sharing state via `Arc<Mutex<_>>` works but introduces lock contention on the render path.

Tenui provides `AsyncBridge<M>` (behind the `tokio` feature flag) as a channel-based alternative:

```rust
// In Cargo.toml:
// tenui = { version = "0.1", features = ["tokio"] }

let bridge = AsyncBridge::new();
let sender = bridge.sender();

// Spawn a background task that sends messages back to the UI thread
sender.spawn_task(|tx| async move {
    let data = fetch_something().await;
    tx.send(MyMsg::DataArrived(data)).ok();
    tx.request_redraw().ok();
});

// In the main loop, drain the bridge alongside input events
loop {
    for msg in bridge.drain() {
        match msg {
            AppMessage::User(m) => handle(m),
            AppMessage::Redraw => {},
            AppMessage::Quit => return Ok(()),
        }
    }
    app.render_frame()?;
}
```

The key properties:

- **The render thread never blocks on async work.** `bridge.drain()` is a non-blocking `try_recv` loop — if no messages are ready, it returns an empty vec and the frame renders immediately.
- **`AppMessageSender` is `Clone + Send`.** Hand it to any number of background tasks. Each task sends domain-typed messages (`AppMessage::User(M)`) back to the main thread.
- **`spawn_task` requires the `tokio` feature** and assumes a Tokio runtime is active. Without the feature, `AsyncBridge` still works as a plain `mpsc` channel for manual thread-based producers.

### Decision guide

| Scenario | Approach |
| :--- | :--- |
| Input-driven UI (editor, form, menu) | Sync loop with `InputDemuxer::poll` |
| Periodic updates (clock, system monitor) | Sync loop with a timer tick |
| Network/IO-driven updates (chat, log tail, API dashboard) | `AsyncBridge` + `tokio` feature |
| Mixed (interactive + background fetch) | `AsyncBridge` for background work, sync input handling in the same loop |
