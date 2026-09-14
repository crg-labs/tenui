use taffy::prelude::*;
use tenui_core::{CanvasSubviewMut, Rect};
// Unified two-lane invalidation vocabulary: a `CustomWidget`
// declares invalidation in the *same* type the reactive `TwoLaneBus` consumes, so it can
// drive the retained render loop. `PAINT` = paint-only (no Taffy reflow); `BOTH` = reflow
// + paint.
pub use tenui_reactive::Invalidation;

/// The open widget contract allowing third-party libraries identical mechanical
/// access to Taffy layout and buffer slices as first-party primitives.
pub trait CustomWidget: 'static {
    /// Invalidation lane declaration for state changes.
    fn query_invalidation(&self) -> Invalidation;

    /// Direct registration with Taffy's flexbox/grid constraint solver.
    fn layout_measure(&self, known_dimensions: Size<Option<f32>>, available_space: Size<AvailableSpace>) -> Size<f32>;

    /// Zero-cost direct memory rasterization onto a hardware-clipped buffer slice.
    fn paint(&self, surface: &mut CanvasSubviewMut<'_>, bounds: Layout);

    /// Spatial navigation & cursor hit testing.
    fn hit_test(&self, _local_x: u16, _local_y: u16) -> bool {
        false
    }
}

/// Host container for a `CustomWidget` that decouples layout measurement from painting,
/// preserving the Two-Lane Invalidation invariant.
pub struct CustomWidgetContainer<W: CustomWidget> {
    pub widget: W,
    pub last_layout: Option<Layout>,
    pub last_invalidation: Invalidation,
}

impl<W: CustomWidget> CustomWidgetContainer<W> {
    pub fn new(widget: W) -> Self {
        Self {
            widget,
            last_layout: None,
            last_invalidation: Invalidation::BOTH,
        }
    }

    /// Queries whether the widget needs reflow or paint-only invalidation.
    pub fn poll_invalidation(&mut self) -> Invalidation {
        let inv = self.widget.query_invalidation();
        self.last_invalidation = inv;
        inv
    }

    /// Measures layout dimensions using the widget's custom solver.
    pub fn measure(&self, known: Size<Option<f32>>, avail: Size<AvailableSpace>) -> Size<f32> {
        self.widget.layout_measure(known, avail)
    }

    /// Renders the widget onto `canvas` clipped to `bounds`.
    pub fn render(&mut self, canvas: &mut CanvasSubviewMut<'_>, bounds: Layout) {
        self.last_layout = Some(bounds);

        let clip = Rect::new(
            bounds.location.x.max(0.0) as u16,
            bounds.location.y.max(0.0) as u16,
            bounds.size.width.max(0.0) as u16,
            bounds.size.height.max(0.0) as u16,
        );

        let mut subview = canvas.subview_mut(clip);
        self.widget.paint(&mut subview, bounds);
        self.last_invalidation = Invalidation::NONE;
    }

    /// Dispatches hit test query to the underlying widget.
    pub fn hit_test(&self, x: u16, y: u16) -> bool {
        if let Some(bounds) = self.last_layout {
            let bx = bounds.location.x as u16;
            let by = bounds.location.y as u16;
            let bw = bounds.size.width as u16;
            let bh = bounds.size.height as u16;

            if x >= bx && x < bx + bw && y >= by && y < by + bh {
                return self.widget.hit_test(x - bx, y - by);
            }
        }
        false
    }
}
