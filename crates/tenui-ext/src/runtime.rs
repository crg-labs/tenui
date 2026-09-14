//! Plugin runtime abstraction — the backend seam and the native (in-process) runtime.
//!
//! [`PluginRuntime`] is the object-safe interface the host drives to execute a mounted
//! plugin regardless of backend. [`NativePluginRuntime`] runs native Rust [`WidgetPlugin`]s
//! with capability enforcement and hot-reload; an optional `wasmtime`
//! backend (reserved behind the `wasm-runtime` feature) implements this same trait, so
//! [`PluginManager`] and callers don't change when it is enabled.

use tenui_core::CanvasSubviewMut;

use crate::wit::{Capability, LayoutConstraints, PluginCapabilityError, PluginHost, PluginPermissions, WidgetPlugin};

/// Backend-agnostic execution interface for one mounted plugin instance.
pub trait PluginRuntime {
    /// Layout constraints for the given available space.
    fn measure(&self, available_w: u16, available_h: u16) -> LayoutConstraints;

    /// Paints the plugin into a hardware-clipped subview.
    fn render(&self, canvas: &mut CanvasSubviewMut<'_>);

    /// Delivers a key event; returns whether it was consumed.
    fn handle_key(&mut self, key_code: u32, modifiers: u8) -> bool;

    /// Enforces a host capability at the sandbox boundary (fails closed).
    fn check_capability(&self, cap: Capability) -> Result<(), PluginCapabilityError>;

    /// Hot-reload generations performed on this instance.
    fn reload_count(&self) -> usize;
}

/// Native (in-process) runtime for a Rust [`WidgetPlugin`], with capability enforcement
/// and zero-downtime hot-reload (via the plugin's `save_state`/`load_state`).
pub struct NativePluginRuntime<W: WidgetPlugin> {
    host: PluginHost<W>,
}

impl<W: WidgetPlugin> NativePluginRuntime<W> {
    pub fn new(name: impl Into<String>, plugin: W, permissions: PluginPermissions) -> Self {
        Self {
            host: PluginHost::new(name, plugin, permissions),
        }
    }

    /// Hot-reloads with a fresh instance, carrying serialized state across (typed — hence
    /// on the concrete runtime rather than the object-safe trait).
    pub fn hot_reload(&mut self, new_plugin: W) {
        self.host.hot_reload(new_plugin);
    }

    pub fn name(&self) -> &str {
        &self.host.name
    }
}

impl<W: WidgetPlugin> PluginRuntime for NativePluginRuntime<W> {
    fn measure(&self, available_w: u16, available_h: u16) -> LayoutConstraints {
        self.host.plugin.measure(available_w, available_h)
    }

    fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        self.host.paint_to_canvas(canvas);
    }

    fn handle_key(&mut self, key_code: u32, modifiers: u8) -> bool {
        self.host.plugin.handle_key(key_code, modifiers)
    }

    fn check_capability(&self, cap: Capability) -> Result<(), PluginCapabilityError> {
        self.host.permissions.require(cap)
    }

    fn reload_count(&self) -> usize {
        self.host.reload_count
    }
}

/// The host-side registry of mounted plugins, addressed by name.
#[derive(Default)]
pub struct PluginManager {
    plugins: Vec<(String, Box<dyn PluginRuntime>)>,
}

impl PluginManager {
    pub fn new() -> Self {
        Self::default()
    }

    /// Mounts a plugin runtime under `name` (replacing any existing one with that name).
    pub fn mount(&mut self, name: impl Into<String>, runtime: Box<dyn PluginRuntime>) {
        let name = name.into();
        if let Some(slot) = self.plugins.iter_mut().find(|(n, _)| *n == name) {
            slot.1 = runtime;
        } else {
            self.plugins.push((name, runtime));
        }
    }

    pub fn len(&self) -> usize {
        self.plugins.len()
    }

    pub fn is_empty(&self) -> bool {
        self.plugins.is_empty()
    }

    pub fn get(&self, name: &str) -> Option<&dyn PluginRuntime> {
        self.plugins.iter().find(|(n, _)| n == name).map(|(_, r)| r.as_ref())
    }

    pub fn get_mut(&mut self, name: &str) -> Option<&mut (dyn PluginRuntime + 'static)> {
        self.plugins
            .iter_mut()
            .find(|(n, _)| n == name)
            .map(|(_, r)| r.as_mut())
    }

    /// Names of all mounted plugins, in mount order.
    pub fn names(&self) -> impl Iterator<Item = &str> {
        self.plugins.iter().map(|(n, _)| n.as_str())
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;
    use crate::wit::{CellPaint, LayoutConstraints};

    struct Counter {
        count: u32,
    }
    impl WidgetPlugin for Counter {
        fn measure(&self, _w: u16, _h: u16) -> LayoutConstraints {
            LayoutConstraints {
                min_width: 1,
                max_width: 10,
                min_height: 1,
                max_height: 1,
            }
        }
        fn render(&self, _w: u16, _h: u16) -> Vec<CellPaint> {
            // Paint the count as a digit at (0,0).
            let ch = std::char::from_digit(self.count % 10, 10).unwrap_or('0');
            vec![CellPaint::new(0, 0, ch.to_string(), 0xFFFFFF, 0x000000)]
        }
        fn handle_key(&mut self, key_code: u32, _mods: u8) -> bool {
            if key_code == b' ' as u32 {
                self.count += 1;
                true
            } else {
                false
            }
        }
        fn save_state(&self) -> Vec<u8> {
            self.count.to_le_bytes().to_vec()
        }
        fn load_state(&mut self, state: &[u8]) {
            if let Ok(bytes) = state.try_into() {
                self.count = u32::from_le_bytes(bytes);
            }
        }
    }

    #[test]
    fn test_native_runtime_render_and_key() {
        let mut mgr = PluginManager::new();
        mgr.mount(
            "counter",
            Box::new(NativePluginRuntime::new(
                "counter",
                Counter { count: 4 },
                PluginPermissions::strict(),
            )),
        );
        assert_eq!(mgr.len(), 1);

        // Key dispatch.
        let consumed = mgr.get_mut("counter").unwrap().handle_key(b' ' as u32, 0);
        assert!(consumed);

        // Render into a buffer.
        let mut buf = Buffer::new(4, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 4, 1));
            mgr.get("counter").unwrap().render(&mut sub);
        }
        assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "5"); // 4 + one space press
    }

    #[test]
    fn test_capability_enforcement_fails_closed() {
        let rt = NativePluginRuntime::new("p", Counter { count: 0 }, PluginPermissions::strict());
        assert_eq!(
            rt.check_capability(Capability::Network),
            Err(PluginCapabilityError::PermissionDenied("network"))
        );

        let rt2 = NativePluginRuntime::new(
            "p",
            Counter { count: 0 },
            PluginPermissions {
                clipboard_access: true,
                ..PluginPermissions::strict()
            },
        );
        assert!(rt2.check_capability(Capability::Clipboard).is_ok());
        assert!(rt2.check_capability(Capability::FileSystem).is_err());
    }

    #[test]
    fn test_hot_reload_preserves_state() {
        let mut rt = NativePluginRuntime::new("p", Counter { count: 41 }, PluginPermissions::strict());
        // New instance starts at 0 but re-hydrates to 41 via save/load_state.
        rt.hot_reload(Counter { count: 0 });
        assert_eq!(rt.reload_count(), 1);
        let mut buf = Buffer::new(2, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 2, 1));
            rt.render(&mut sub);
        }
        assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "1"); // 41 % 10
    }
}
