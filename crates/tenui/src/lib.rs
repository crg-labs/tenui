#![forbid(unsafe_code)]
//! # Tenui
//!
//! **Tenui** is a modular, high-performance terminal user interface (TUI) framework for Rust.
//! It unifies the mechanical sympathy and predictable efficiency of immediate-mode rendering
//! with the compositional power of flexbox layout, fine-grained reactivity, and an enterprise widget suite.
//!
//! ## Core Architecture & Feature Layers
//!
//! Tenui is organized as a layered stack of focused, cohesive crates through progressive disclosure:
//!
//! ```text
//! ┌────────────────────────────────────────────────────────────────────────┐
//! │  Standard Suite (tenui-std) — Chat, Tables, Code, Diff, Charts, Themes │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │  Retained Runtime (tenui-runtime) — App Reactor, Focus, Invalidation   │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │  Layer 2 (tenui-compositor) — Z-Layers, Occlusion Cut, Spatial Nav     │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │  Layer 1 (tenui-reactive)   — Two-Lane Bus (PAINT vs LAYOUT), Batch    │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │  Layer 0 (tenui-layout)     — Immediate Flexbox Layout via Taffy       │
//! ├────────────────────────────────────────────────────────────────────────┤
//! │  Core (tenui-core)          — Cell, Buffer, Diff Coalescer, Terminal   │
//! └────────────────────────────────────────────────────────────────────────┘
//! ```
//!
//! ### Progressive Feature Tiers
//!
//! - **Core ([`core`])**: The bare-metal foundation. Provides the compact 8-byte [`Cell`] data union,
//!   double-buffered [`Buffer`], differential [`SgrCoalescer`] emission engine, RAII [`TerminalGuard`],
//!   and non-blocking terminal I/O drivers.
//! - **Layer 0 (`layer0` / [`layout`])**: Pure-Rust immediate-mode flexbox layout engine built on Taffy.
//!   Allows declarative styling and proportional grid positioning without an asynchronous runtime.
//! - **Layer 1 (`layer1` / [`reactive`])**: Fine-grained reactive signal graph and the **Two-Lane Invalidation Engine**.
//!   Explicitly distinguishes cosmetic changes ([`Invalidation::PAINT`](reactive::Invalidation::PAINT)) from
//!   structural changes ([`Invalidation::LAYOUT`](reactive::Invalidation::LAYOUT)), guaranteeing that paint-only
//!   updates never trigger layout reflow.
//! - **Layer 2 (`layer2` / [`compositor`])**: Off-screen spatial layer compositor ([`LayerCompositor`](compositor::LayerCompositor)),
//!   coordinate cut-finding occlusion culling, WICG spatial navigation, and sub-cell vector graphics
//!   (Braille, half-block, and quadrant rasterizers).
//! - **Kinetics (`anim` / [`anim`])**: Closed-form analytical spring dynamics ([`AnalyticalSpring`](anim::AnalyticalSpring)),
//!   sub-cell Unicode 1/8th fractional micro-stepping ([`MicroStepper`](anim::MicroStepper)), and zero-idle
//!   power scheduling ([`ActiveTicker`](anim::ActiveTicker)).
//! - **Runtime (`runtime` / [`runtime`])**: Retained-mode [`App`](runtime::App) reactor driving a persistent Taffy tree,
//!   focus management, and the two-lane invalidation bus.
//! - **Standard Library (`std` / [`std_widgets`])**: Batteries-included widget suite including [`ChatThread`](std_widgets::ChatThread),
//!   [`VirtualTable`](std_widgets::VirtualTable), [`TextInput`](std_widgets::TextInput), [`TextEditor`](std_widgets::TextEditor),
//!   [`CodeView`](std_widgets::CodeView), [`DiffView`](std_widgets::DiffView), [`Form`](std_widgets::Form),
//!   [`LinePlot`](std_widgets::LinePlot), [`BarChart`](std_widgets::BarChart), [`TreeView`](std_widgets::TreeView),
//!   [`FilePicker`](std_widgets::FilePicker), and [`ThemePalette`](std_widgets::ThemePalette).
//! - **Visual Effects (`vfx` / [`vfx`])**: Procedural sub-cell shaders, acrylic blur filters, particle kinematics,
//!   and CRT/shimmer/glitch effects.
//! - **Extensibility (`ext` / [`ext`])**: The open [`CustomWidget`](ext::CustomWidget) trait, Tower-inspired
//!   input middleware pipeline, WASM component model contract, and embedded Lua/Rhai scripting bridge.
//! - **Macros (`macros` / [`macros`])**: Compile-time declarative [`view!`] macro for structured UI composition.
//! - **Advanced Systems (`advanced`)**: Drag-and-drop docking ([`dnd`]), virtual list acceleration ([`virt`]),
//!   text buffers with branching undo ([`text`]), daemon workspace persistence ([`session`]), tactile audio cues ([`sound`]),
//!   and hardware-accelerated GPU terminal rendering ([`wgpu`]).
//! - **Frontier Systems (`frontier`])**: Vectorized SIMD differential scanning ([`simd`]), multi-seat CRDT presence ([`collab`]),
//!   half-block video streaming ([`media`]), sub-cell math typesetting ([`math`]), hardware/MIDI input bridges ([`hid`]),
//!   and stream chaos fuzzing ([`chaos`]).
//!
//! ## Quickstart: Immediate-Mode "Hello, TUI"
//!
//! Below is a minimal, fully functional immediate-mode program:
//!
//! ```no_run
//! use std::io;
//! use tenui::{Terminal, Rect, Color, Modifier};
//!
//! fn main() -> io::Result<()> {
//!     // Initialize terminal and acquire RAII guard (raw mode + alternate screen)
//!     let mut term = Terminal::new()?;
//!
//!     // Render an immediate-mode frame
//!     term.draw(|canvas| {
//!         let (w, h) = (canvas.width(), canvas.height());
//!         canvas.clear(Color::Reset);
//!         canvas.write_str_clipped(
//!             2,
//!             1,
//!             "Hello from Tenui!",
//!             Color::Cyan,
//!             Color::Reset,
//!         );
//!     })?;
//!
//!     Ok(())
//! }
//! ```
//!
//! For automated testing and headless assertion, buffers can be inspected in memory without a TTY:
//!
//! ```rust
//! use tenui::{Buffer, Rect, Color};
//!
//! let mut buffer = Buffer::new(40, 5);
//! {
//!     let mut canvas = buffer.subview_mut(Rect::new(0, 0, 40, 5));
//!     canvas.write_str_clipped(1, 1, "Headless Test", Color::Green, Color::Reset);
//! }
//!
//! assert_eq!(buffer.get(1, 1).unwrap().symbol.as_str(), "H");
//! assert_eq!(buffer.get(2, 1).unwrap().symbol.as_str(), "e");
//! ```
//!
//! ## Flagship Example: Streaming Chat Client
//!
//! For a complete, real-world application showcasing Tenui's three-zone shell (collapsible
//! sidebar, hybrid conversational transcript, multi-line composer), live SSE token streaming,
//! reasoning blocks, mouse drag-resizing, and theme switching, run:
//!
//! ```bash
//! cargo run --release --example chat
//! ```
//!
//! Located at [`crates/tenui/examples/chat/`](https://github.com/crg-labs/tenui/tree/main/crates/tenui/examples/chat),
//! it serves as the canonical worked case study for building flagship terminal applications.
//!
//! ## The Prelude
//!
//! For convenience, common types and traits can be imported via:
//!
//! ```rust
//! use tenui::prelude::*;
//! ```

pub use crossterm;
pub use taffy;
/// Sub-cell kinetics, analytical spring physics, and dynamic occupancy ticker.
#[cfg(feature = "anim")]
pub use tenui_anim as anim;
/// Deterministic stream chaos fuzzing and fault injection.
#[cfg(feature = "chaos")]
pub use tenui_chaos as chaos;
/// Multi-seat real-time collaborative text editing backed by CRDT algorithms.
#[cfg(feature = "collab")]
pub use tenui_collab as collab;
/// Layer 2: Off-screen spatial layer compositor, occlusion culling, and spatial navigation.
#[cfg(feature = "layer2")]
pub use tenui_compositor as compositor;
/// Core terminal rendering engine, differential SGR coalescer, cell primitives, and terminal driver.
pub use tenui_core as core;
// Convenient root re-exports
pub use tenui_core::{
    Announcer, Buffer, CanvasSubviewMut, Cell, Clipboard, ClipboardCommand, ClipboardTarget, Color, DndManager,
    DragEvent, DropTargetEntry, FrameClock, Modifier, MouseButton, MouseDispatcher, MouseEvent, MouseTargetEntry,
    NodeId, PathSanitizer, Point, Politeness, Rect, Role, ScreenReaderBridge, Selection, SemanticNode, SemanticTree,
    SgrCoalescer, SgrMouseParser, Terminal, TerminalGuard, TextOverflow, apply_clipboard_command, encode_osc52,
};
/// Drag-and-drop session management and multi-panel dock layout.
#[cfg(feature = "dnd")]
pub use tenui_dnd as dnd;
/// Extensibility, WASM component model contract, Tower input middleware, and scripting bridges.
#[cfg(feature = "ext")]
pub use tenui_ext as ext;
/// Hardware knob, MIDI, and rotary controller input reactor.
#[cfg(feature = "hid")]
pub use tenui_hid as hid;
/// Layer 0: Immediate-mode flexbox layout engine built on Taffy.
#[cfg(feature = "layer0")]
pub use tenui_layout as layout;
/// Declarative UI macros (`view!`).
#[cfg(feature = "macros")]
pub use tenui_macros as macros;
#[cfg(feature = "macros")]
pub use tenui_macros::view;
/// Mathematical formula typesetting in terminal cells.
#[cfg(feature = "math")]
pub use tenui_math as math;
/// Sub-cell half-block video streaming and media decoding.
#[cfg(feature = "media")]
pub use tenui_media as media;
/// Layer 1: Two-Lane Invalidation reactive signal graph (`PAINT` vs `LAYOUT`).
#[cfg(feature = "layer1")]
pub use tenui_reactive as reactive;
/// Retained-mode application reactor and component tree runtime.
#[cfg(feature = "runtime")]
pub use tenui_runtime as runtime;
/// Daemon session persistence and serialized workspace layouts.
#[cfg(feature = "session")]
pub use tenui_session as session;
/// Vectorized SIMD differential scanning for ultra-high-resolution viewports.
#[cfg(feature = "simd")]
pub use tenui_simd as simd;
/// Tactile audio feedback engine for terminal interactions.
#[cfg(feature = "sound")]
pub use tenui_sound as sound;
/// Batteries-included standard widget library and theme system.
#[cfg(feature = "std")]
pub use tenui_std as std_widgets;
/// Text editing buffer with multi-cursor support and branching undo history.
#[cfg(feature = "text")]
pub use tenui_text as text;
/// Procedural visual effects, shaders, acrylic blur, and particle kinematics.
#[cfg(feature = "vfx")]
pub use tenui_vfx as vfx;
/// High-capacity virtualized list widgets with Fenwick tree coordinate acceleration.
#[cfg(feature = "virt")]
pub use tenui_virt as virt;
/// Hardware-accelerated GPU terminal rendering pipeline using WGPU.
#[cfg(feature = "wgpu")]
pub use tenui_wgpu as wgpu;

/// Common imports for rapid application development with Tenui.
pub mod prelude {
    #[cfg(feature = "chaos")]
    pub use crate::chaos::ChaosStreamFuzzer;
    #[cfg(feature = "collab")]
    pub use crate::collab::{
        CollabOperation, CollaborativeText, CrdtOp, MultiSeatManager, OpId, SeatPresence, TextCrdt,
    };
    pub use crate::core::{
        Announcer, Buffer, CanvasSubviewMut, Cell, Clipboard, ClipboardCommand, ClipboardTarget, Color, DndManager,
        DragEvent, DropTargetEntry, FrameClock, Modifier, MouseButton, MouseDispatcher, MouseEvent, MouseTargetEntry,
        NodeId, PathSanitizer, Point, Politeness, Rect, Role, ScreenReaderBridge, Selection, SemanticNode,
        SemanticTree, SgrCoalescer, SgrMouseParser, Terminal, TerminalGuard, TextOverflow, apply_clipboard_command,
        encode_osc52,
    };
    #[cfg(feature = "dnd")]
    pub use crate::dnd::{DockManager, DockPlacement, DragSource, DropConsumer, InternalDragSession};
    #[cfg(feature = "ext")]
    pub use crate::ext::{
        Capability, ClipboardLayer, CustomWidget, CustomWidgetContainer, InputMiddleware, MiddlewarePipeline,
        NativePluginRuntime, PluginManager, PluginPermissions, PluginRuntime, WidgetPlugin,
    };
    #[cfg(feature = "hid")]
    pub use crate::hid::{
        HardwareAction, HidEventReactor, KeyLedFeedback, RotaryImpulseController, normalize_hid_16bit,
        normalize_midi_7bit, normalize_midi_14bit,
    };
    #[cfg(feature = "layer0")]
    pub use crate::layout::{BorderStyle, Style, StyleExt, TerminalLayoutExt};
    #[cfg(feature = "macros")]
    pub use crate::macros::view;
    #[cfg(feature = "math")]
    pub use crate::math::{MathNode, MathTypesetter};
    #[cfg(feature = "media")]
    pub use crate::media::{HalfBlockVideoConverter, MediaStreamController, PixelFormat, VideoFrame};
    #[cfg(feature = "layer1")]
    pub use crate::reactive::{Derived, Invalidation, ReactiveContext, Signal, TwoLaneBus, batch};
    #[cfg(feature = "runtime")]
    pub use crate::runtime::{App, FrameReport};
    #[cfg(feature = "session")]
    pub use crate::session::{
        DaemonLifecycle, DaemonSessionManager, SerializedDimension, SerializedFlexDirection, SerializedPaneNode,
        WorkspaceSession,
    };
    // Frontier subsystems
    #[cfg(feature = "simd")]
    pub use crate::simd::{AlignedCell, SimdDiffScanner};
    #[cfg(feature = "sound")]
    pub use crate::sound::{SoundCue, TactileAudioEngine};
    #[cfg(feature = "std")]
    pub use crate::std_widgets::*;
    #[cfg(feature = "text")]
    pub use crate::text::{
        BranchingUndoTree, Cursor, CursorSet, SearchState, TextBuffer, TextEditDelta, UndoTreeNode, apply_delta,
        copy_selection, cut_selection, paste,
    };
    #[cfg(feature = "vfx")]
    pub use crate::vfx::prelude::*;
    #[cfg(feature = "virt")]
    pub use crate::virt::{FenwickTree, VirtualList};
    #[cfg(feature = "wgpu")]
    pub use crate::wgpu::{CellInstance, GpuPipelineConfig, WgpuCanvasRunner};
}
