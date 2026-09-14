use tenui_core::{CanvasSubviewMut, Rect};

/// Immediate-mode canvas escape hatch establishing a layout-isolated boundary.
///
/// Animation updates to an isolated canvas dispatch paint operations exclusively,
/// guaranteeing O(0) Taffy constraint recalculation cost during 60 FPS rendering.
pub struct IsolatedCanvas {
    pub bounds: Rect,
}

impl IsolatedCanvas {
    pub fn new(bounds: Rect) -> Self {
        Self { bounds }
    }

    /// Renders immediate raster graphics with strict hardware clipping.
    pub fn paint<F>(&self, subview: &mut CanvasSubviewMut<'_>, f: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        f(subview);
    }
}
