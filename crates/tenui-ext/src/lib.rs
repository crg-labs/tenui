#![forbid(unsafe_code)]
//! # Tenui Ext (`tenui-ext`)
//!
//! Extensibility, plugin architecture, Tower-inspired input middleware, and scripting runtime bridge.
//!
//! `tenui-ext` provides modular extension points for the Tenui framework:
//!
//! ## Core Architecture
//!
//! - **Input Middleware Pipeline ([`MiddlewarePipeline`])**: Tower-inspired event interception onion
//!   chain dispatching input events through composable layers:
//!   - [`VimModalLayer`]: Modal key state machine (Normal, Insert, Visual, Command).
//!   - [`KeymapTrieFilter`]: Prefix-tree keybinding resolver supporting multi-stroke chords (e.g. `g g`, `Ctrl+X Ctrl+S`).
//!   - [`MacroRecorder`]: Keystroke sequence recorder and replay engine.
//! - **Plugin Host ([`PluginHost`])**: Native Rust plugin runtime using WIT-inspired contracts
//!   ([`WidgetPlugin`]). A sandboxed WASM backend is planned but not yet wired.
//! - **[`CustomWidget`]**: Trait enabling custom retained-mode widgets to integrate directly with
//!   Tenui's Two-Lane Invalidation Engine.
//! - **Embedded Scripting Bridge ([`ScriptEngineBridge`])**: Dynamic script execution for user customization.
//!
//! ## Runnable Example: Input Middleware Dispatch
//!
//! ```rust
//! use tenui_core::InputEvent;
//! use tenui_ext::{EventResult, MiddlewareContext, MiddlewarePipeline};
//!
//! let mut pipeline = MiddlewarePipeline::new();
//! let mut cx = MiddlewareContext::default();
//!
//! // Unhandled events pass cleanly through the pipeline
//! let event = InputEvent::Paste("sample".into());
//! let result = pipeline.dispatch(&event, &mut cx);
//! assert_eq!(result, EventResult::Ignored);
//! ```

pub mod middleware;
pub mod runtime;
pub mod scripting;
pub mod widget;
pub mod wit;

pub use middleware::{
    ClipboardLayer, EventResult, InputMiddleware, KeymapTrieFilter, MacroRecorder, MiddlewareContext,
    MiddlewarePipeline, VimModalLayer, VimMode,
};
pub use runtime::{NativePluginRuntime, PluginManager, PluginRuntime};
pub use scripting::{ScriptEngineBridge, ScriptValue};
pub use widget::{CustomWidget, CustomWidgetContainer, Invalidation};
pub use wit::{
    Capability, CellPaint, LayoutConstraints, PluginCapabilityError, PluginHost, PluginPermissions, WidgetPlugin,
};
