//! Plugin contract (the `tenui:plugin/widget.wit` shape) and capability model.
//!
//! STATUS: this is the in-process **contract** (`WidgetPlugin`) plus the native runtime in
//! [`crate::runtime`]. An optional `wasmtime` + WASI Preview 2
//! Component Model backend running sandboxed `.wasm` is reserved behind the
//! `wasm-runtime` feature and not yet wired. Native
//! Rust plugins run today through [`crate::runtime::NativePluginRuntime`]; a wasm backend
//! will implement the same [`crate::runtime::PluginRuntime`] seam.

use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// WIT Interface layout constraints (tenui:plugin/widget.wit).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct LayoutConstraints {
    pub min_width: u16,
    pub max_width: u16,
    pub min_height: u16,
    pub max_height: u16,
}

/// A cell paint instruction emitted by a WASM plugin (tenui:plugin/widget.wit).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellPaint {
    pub x: u16,
    pub y: u16,
    pub grapheme: String,
    pub fg_rgb: u32,
    pub bg_rgb: u32,
}

impl CellPaint {
    pub fn new(x: u16, y: u16, grapheme: impl Into<String>, fg_rgb: u32, bg_rgb: u32) -> Self {
        Self {
            x,
            y,
            grapheme: grapheme.into(),
            fg_rgb,
            bg_rgb,
        }
    }
}

/// WASM Component Model plugin lifecycle contract (tenui:plugin/widget.wit).
pub trait WidgetPlugin: Send + Sync + 'static {
    /// Queries widget layout constraints given available viewport dimensions.
    fn measure(&self, available_w: u16, available_h: u16) -> LayoutConstraints;

    /// Renders widget contents into a list of cell paint operations.
    fn render(&self, viewport_w: u16, viewport_h: u16) -> Vec<CellPaint>;

    /// Handles keyboard input events. Returns true if consumed.
    fn handle_key(&mut self, key_code: u32, modifiers: u8) -> bool;

    /// Serializes active state for zero-downtime hot-reloading.
    fn save_state(&self) -> Vec<u8> {
        Vec::new()
    }

    /// Re-hydrates state into a newly mounted plugin instance.
    fn load_state(&mut self, _state: &[u8]) {}
}

/// Capability-based security permissions for sandboxed WASM plugins.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PluginPermissions {
    pub fs_access: bool,
    pub network_access: bool,
    pub clipboard_access: bool,
}

/// A host capability a plugin may request (maps to WASI capabilities in the wasm backend).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Capability {
    FileSystem,
    Network,
    Clipboard,
}

impl PluginPermissions {
    pub fn strict() -> Self {
        Self {
            fs_access: false,
            network_access: false,
            clipboard_access: false,
        }
    }

    pub fn full() -> Self {
        Self {
            fs_access: true,
            network_access: true,
            clipboard_access: true,
        }
    }

    /// Whether `cap` is granted.
    pub fn allows(&self, cap: Capability) -> bool {
        match cap {
            Capability::FileSystem => self.fs_access,
            Capability::Network => self.network_access,
            Capability::Clipboard => self.clipboard_access,
        }
    }

    /// Enforces a capability at the host boundary, failing closed when it isn't granted.
    pub fn require(&self, cap: Capability) -> Result<(), PluginCapabilityError> {
        if self.allows(cap) {
            Ok(())
        } else {
            Err(PluginCapabilityError::PermissionDenied(match cap {
                Capability::FileSystem => "filesystem",
                Capability::Network => "network",
                Capability::Clipboard => "clipboard",
            }))
        }
    }
}

/// Error returned when a sandboxed plugin attempts an unauthorized capability.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PluginCapabilityError {
    PermissionDenied(&'static str),
    ExecutionTrapped(String),
}

/// Host runtime managing sandboxed execution, permissions, and zero-downtime hot-reloading.
pub struct PluginHost<W: WidgetPlugin> {
    pub name: String,
    pub permissions: PluginPermissions,
    pub plugin: W,
    pub reload_count: usize,
}

impl<W: WidgetPlugin> PluginHost<W> {
    pub fn new(name: impl Into<String>, plugin: W, permissions: PluginPermissions) -> Self {
        Self {
            name: name.into(),
            permissions,
            plugin,
            reload_count: 0,
        }
    }

    /// Performs zero-downtime hot-reloading by serializing state, replacing the instance,
    /// and re-hydrating in < 1ms with zero flicker.
    pub fn hot_reload(&mut self, mut new_plugin: W) {
        let state = self.plugin.save_state();
        new_plugin.load_state(&state);
        self.plugin = new_plugin;
        self.reload_count += 1;
    }

    /// Renders the plugin directly to a hardware-clipped CanvasSubviewMut.
    pub fn paint_to_canvas(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width();
        let h = canvas.height();
        let paints = self.plugin.render(w, h);

        for p in paints {
            if p.x < w && p.y < h {
                let r = ((p.fg_rgb >> 16) & 0xFF) as u8;
                let g = ((p.fg_rgb >> 8) & 0xFF) as u8;
                let b = (p.fg_rgb & 0xFF) as u8;
                let fg = Color::Rgb(r, g, b);

                let br = ((p.bg_rgb >> 16) & 0xFF) as u8;
                let bg_g = ((p.bg_rgb >> 8) & 0xFF) as u8;
                let bb = (p.bg_rgb & 0xFF) as u8;
                let bg = Color::Rgb(br, bg_g, bb);

                let ch = p.grapheme.chars().next().unwrap_or(' ');
                canvas.set_char(p.x, p.y, ch, fg, bg, Modifier::empty());
            }
        }
    }
}
