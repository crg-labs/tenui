#![forbid(unsafe_code)]
//! Tenui retained reactive runtime — the component tree and event reactor.
//!
//! While [`tenui_core::Terminal`] provides immediate-mode rendering, `tenui-runtime`
//! provides a retained-mode application reactor driven by the Two-Lane Invalidation
//! pipeline and the dynamic [`ActiveTicker`](tenui_anim::ActiveTicker) demand scheduler.
//!
//! [`App`] manages the retained loop: it owns a persistent [`taffy::TaffyTree`], a
//! [`TwoLaneBus`](tenui_reactive::TwoLaneBus), an
//! [`ActiveTicker`](tenui_anim::ActiveTicker), and a [`Terminal`](tenui_core::Terminal).
//!
//! On each frame, the runtime:
//! - **Differentiates layout from paint**: skips Taffy reflow entirely when only cosmetic
//!   updates occur ([`Invalidation::PAINT`](tenui_reactive::Invalidation::PAINT)), achieving
//!   zero-reflow redraws.
//! - **Bounds repaints**: re-renders only paint-dirty or newly reflowed node rectangles, letting the
//!   differential SGR coalescer emit only modified cells via the terminal driver.
//!
//! # Immediate vs. Retained Mode
//!
//! - **Immediate Mode** ([`tenui_core::Terminal::draw`]):
//!   Best for simple tools, full-screen transcripts, CLI utilities, and widgets where you
//!   prefer to re-specify the UI state every frame.
//! - **Retained Mode** ([`App`]):
//!   Best for complex multi-panel dashboards, reactive graphs, and hierarchical component
//!   trees with fine-grained state updates.

pub mod app;
pub mod async_bridge;
pub mod error;

pub use app::{App, FrameReport, NodeId};
pub use async_bridge::{AppMessage, AppMessageSender, AsyncBridge};
pub use error::AppError;
