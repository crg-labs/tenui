#![forbid(unsafe_code)]
//! # Tenui Core
//!
//! **Tenui Core** is the foundational rendering engine, protocol driver, and primitive
//! layout substrate of the Tenui TUI framework.
//!
//! ## The Rendering Pipeline: Front/Back Buffering & Differential SGR
//!
//! Tenui utilizes an immediate-mode differential rendering pipeline engineered for minimum latency
//! and zero visual tearing:
//!
//! ```text
//! ┌────────────────┐          ┌────────────────┐
//! │  Front Buffer  │          │  Back Buffer   │ (Render targets painted here)
//! └───────┬────────┘          └───────┬────────┘
//!         │                           │
//!         └───────────┬───────────────┘
//!                     ▼
//!         ┌────────────────────────┐
//!         │    Diff Comparator     │ (Identifies modified cells only)
//!         └───────────┬────────────┘
//!                     ▼
//!         ┌────────────────────────┐
//!         │     SgrCoalescer       │ (Batches color/modifier state transitions)
//!         └───────────┬────────────┘
//!                     ▼
//!         ┌────────────────────────┐
//!         │ Synchronized Terminal  │ (Emits atomic escape sequences \x1b[?2026h/l)
//!         └────────────────────────┘
//! ```
//!
//! 1. **Immediate-Mode Paint**: Callers render into a [`CanvasSubviewMut`] representing a rectangular slice
//!    of the back [`Buffer`].
//! 2. **Cell Diffing**: When [`Terminal::draw`] finishes, the back buffer is compared against the front buffer.
//!    Unchanged cells are skipped with zero byte emissions.
//! 3. **SGR State Coalescing**: [`SgrCoalescer`] analyzes the dirty runs and emits escape sequences only when
//!    foreground, background, or text attributes transition, minimizing ANSI protocol overhead.
//! 4. **Synchronized Output**: Terminal outputs are wrapped in standard terminal synchronization escapes
//!    (`\x1b[?2026h` .. `\x1b[?2026l`), eliminating flicker on modern terminal emulators.
//!
//! ## Compact 8-Byte Cell Architecture
//!
//! Terminal grids are represented by continuous arrays of [`Cell`] structs. Each cell uses an inlined
//! representation for standard Unicode codepoints and box-drawing glyphs, avoiding heap allocation on the hot path.
//!
//! ## Headless Buffer Example
//!
//! ```rust
//! use tenui_core::{Buffer, Rect, Color, Modifier};
//!
//! let mut buffer = Buffer::new(40, 5);
//! {
//!     let mut canvas = buffer.subview_mut(Rect::new(0, 0, 40, 5));
//!     canvas.write_str_clipped(2, 1, "Tenui Core", Color::Yellow, Color::Reset);
//! }
//!
//! let cell = buffer.get(2, 1).expect("cell exists");
//! assert_eq!(cell.symbol.as_str(), "T");
//! assert_eq!(cell.fg, Color::Yellow);
//! ```

/// Semantic accessibility tree, live region announcements, and screen-reader transcript generation.
pub mod a11y;
/// Unicode Bidirectional (BiDi) algorithm text reordering for RTL languages.
pub mod bidi;
/// In-memory 2D character grid buffers and subview clipping windows.
pub mod buffer;
/// Low-level 8-byte cell representations, grapheme clustering, and modifiers.
pub mod cell;
/// System clipboard integrations and OSC 52 terminal copy escapes.
pub mod clipboard;
/// High-resolution frame rate clock and frame delta budget regulation.
pub mod clock;
/// Terminal color representations: ANSI 16, ANSI 256, 24-bit TrueColor, and Reset.
pub mod color;
/// Remote DevTools protocol and telemetry ring buffers.
pub mod devtools;
/// Differential cell scanner comparing front and back buffers.
pub mod diff;
/// Drag-and-drop file path parsing and sanitization.
pub mod dnd;
/// Runtime drag-and-drop hit target dispatcher.
pub mod dnd_manager;
/// Abstract terminal driver contracts for physical, headless, and virtual targets.
pub mod driver;
/// Typed error enums for terminal initialization, restore, and I/O failures.
pub mod error;
/// Network-aware flow control adapting frame rates and color fidelity over slow connections.
pub mod flow;
/// 2D geometric primitives: points, coordinates, and boundaries.
pub mod geometry;
/// RAII terminal lifecycle guard ensuring clean exit and raw-mode restoration.
pub mod guard;
/// Clickable OSC 8 terminal hyperlink formatting.
pub mod hyperlink;
/// Hardware cursor styling and IME composition positioning.
pub mod ime;
/// Non-alternate screen inline viewport rendering in standard shell scrollback.
pub mod inline;
/// Unified keyboard, mouse, paste, and capability input event demuxing.
pub mod input;
/// Abstract layout node identifiers.
pub mod layout;
/// Mouse button and action event structures.
pub mod mouse;
/// Spatial mouse target hit testing and event routing.
pub mod mouse_dispatcher;
/// SGR 1006 / 1016 mouse protocol decoder.
pub mod mouse_parser;
/// Non-blocking buffered terminal writer with adaptive backpressure handling.
pub mod nonblocking;
/// OSC 11 ambient background color query and luminance auditing.
pub mod osc11;
/// Text selection bounds and range calculations.
pub mod selection;
/// Differential SGR escape sequence coalescer.
pub mod sgr;
/// Primary terminal handle managing alternate screens, raw mode, and frame loops.
pub mod terminal;

/// Canvas subview re-exports.
pub mod canvas {
    pub use crate::buffer::CanvasSubviewMut;
}

pub use a11y::{Announcement, Announcer, Politeness, Role, ScreenReaderBridge, SemanticNode, SemanticTree};
pub use bidi::{bidi_reorder, contains_rtl, is_rtl_char};
pub use buffer::{Buffer, CanvasSubviewMut, Rect, TextOverflow};
pub use cell::{Cell, CellData, CellSpillover, CompactSymbol, Modifier, TerminalCapabilities, UnderlineStyle};
pub use clipboard::{Clipboard, ClipboardCommand, ClipboardTarget, apply_clipboard_command, encode_osc52};
pub use clock::FrameClock;
pub use color::Color;
pub use devtools::{DevToolsProtocol, FrameTelemetry, TimeTravelRingBuffer};
pub use dnd::PathSanitizer;
pub use dnd_manager::{DndManager, DragEvent, DropTargetEntry};
pub use driver::{TerminalDriver, VirtualTerminalDriver};
pub use error::TerminalError;
pub use flow::{ColorTier, NetworkBandwidthState, SshFlowController};
pub use geometry::Point;
pub use guard::TerminalGuard;
pub use hyperlink::format_osc8_hyperlink;
pub use ime::{CursorStyle, format_cursor_sync};
pub use inline::InlineViewport;
pub use input::{InputDemuxer, InputEvent, PixelDecoupler};
pub use layout::NodeId;
pub use mouse::{MouseButton, MouseEvent};
pub use mouse_dispatcher::{MouseDispatcher, MouseTargetEntry};
pub use mouse_parser::SgrMouseParser;
pub use nonblocking::{FrameStatus, NonBlockingWriter};
pub use osc11::{is_light_theme, osc11_query, parse_osc11_response, relative_luminance};
pub use selection::Selection;
pub use sgr::SgrCoalescer;
pub use terminal::Terminal;
