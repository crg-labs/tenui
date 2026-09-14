use tenui_core::{Buffer, CanvasSubviewMut, Rect};

use crate::cut::cut_multi;

type LayerPaintFn = Box<dyn FnOnce(&mut CanvasSubviewMut<'_>)>;

struct Layer {
    rect: Rect,
    z_index: i32,
    paint: LayerPaintFn,
}

/// Off-screen spatial layer compositor with coordinate cut-finding occlusion culling.
pub struct Compositor {
    layers: Vec<Layer>,
}

impl Default for Compositor {
    fn default() -> Self {
        Self::new()
    }
}

impl Compositor {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    /// Pushes a paintable layer with an explicit z-index.
    pub fn add_layer<F>(&mut self, rect: Rect, z_index: i32, paint: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>) + 'static,
    {
        self.layers.push(Layer {
            rect,
            z_index,
            paint: Box::new(paint),
        });
    }

    /// Renders layers in z-index order, culling obscured rectangular regions
    /// before committing to the back-buffer to eliminate back-buffer overdraw.
    pub fn composite(mut self, buffer: &mut Buffer) {
        // Sort layers by z_index ascending (lower layers painted first)
        self.layers.sort_by_key(|l| l.z_index);

        let count = self.layers.len();
        for i in 0..count {
            let target_rect = self.layers[i].rect;

            // Collect all bounding boxes from higher z-index layers
            let mut obscurers = Vec::new();
            for j in (i + 1)..count {
                if self.layers[j].z_index > self.layers[i].z_index {
                    let higher_rect = self.layers[j].rect;
                    if target_rect.intersects(&higher_rect) {
                        obscurers.push(higher_rect);
                    }
                }
            }

            // Perform coordinate cut-finding: get visible fragments
            let visible_fragments = cut_multi(target_rect, &obscurers);

            // If completely obscured, drop paint execution completely
            if visible_fragments.is_empty() && !obscurers.is_empty() {
                continue;
            }

            if let Some(paint) = self
                .layers
                .get_mut(i)
                .map(|l| std::mem::replace(&mut l.paint, Box::new(|_| {})))
            {
                if obscurers.is_empty() {
                    // Entire layer is visible: direct buffer write
                    let mut subview = buffer.subview_mut(target_rect);
                    paint(&mut subview);
                } else {
                    // Partially obscured: render into isolated layer scratch buffer,
                    // then commit strictly visible fragments to back-buffer.
                    let mut scratch = Buffer::new(target_rect.width, target_rect.height);
                    {
                        let scratch_rect = scratch.rect();
                        let mut subview = scratch.subview_mut(scratch_rect);
                        paint(&mut subview);
                    }

                    // Blit only visible fragments into the destination back buffer
                    for fragment in visible_fragments {
                        for gy in fragment.top()..fragment.bottom() {
                            for gx in fragment.left()..fragment.right() {
                                let local_x = gx.saturating_sub(target_rect.x);
                                let local_y = gy.saturating_sub(target_rect.y);
                                if let Some(src_cell) = scratch.get(local_x, local_y)
                                    && let Some(dst_cell) = buffer.get_mut(gx, gy)
                                {
                                    *dst_cell = *src_cell;
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Sink for receiving invalidation notifications upon modal or layer dismissal.
pub trait InvalidationSink {
    fn mark_dirty(&mut self, node_id: u64);
}

impl<F: FnMut(u64)> InvalidationSink for F {
    fn mark_dirty(&mut self, node_id: u64) {
        self(node_id);
    }
}

/// Modal descriptor holding bounds and layer metadata.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ModalHandle {
    pub id: u64,
    pub bounds: Rect,
}

impl ModalHandle {
    pub fn new(id: u64, bounds: Rect) -> Self {
        Self { id, bounds }
    }

    pub fn computed_bounds(&self) -> Rect {
        self.bounds
    }
}

/// Advanced LayerCompositor supporting retained spatial indices and modal dismissal invalidation.
pub struct LayerCompositor {
    active_modal: Option<ModalHandle>,
    spatial_index: Vec<(u64, Rect)>,
}

impl Default for LayerCompositor {
    fn default() -> Self {
        Self::new()
    }
}

impl LayerCompositor {
    pub fn new() -> Self {
        Self {
            active_modal: None,
            spatial_index: Vec::new(),
        }
    }

    pub fn register_node(&mut self, node_id: u64, bounds: Rect) {
        if let Some(pos) = self.spatial_index.iter().position(|(id, _)| *id == node_id) {
            self.spatial_index[pos].1 = bounds;
        } else {
            self.spatial_index.push((node_id, bounds));
        }
    }

    pub fn unregister_node(&mut self, node_id: u64) {
        self.spatial_index.retain(|(id, _)| *id != node_id);
    }

    pub fn set_active_modal(&mut self, modal: ModalHandle) {
        self.active_modal = Some(modal);
    }

    pub fn active_modal(&self) -> Option<&ModalHandle> {
        self.active_modal.as_ref()
    }

    /// Dismisses the active modal overlay and invalidates all spatial nodes
    /// whose bounding boxes intersect the dismissed modal's bounds.
    pub fn dismiss_modal<S: InvalidationSink + ?Sized>(&mut self, sink: &mut S) {
        if let Some(modal) = self.active_modal.take() {
            let modal_bounds = modal.computed_bounds();
            for (node_id, bounds) in &self.spatial_index {
                if bounds.intersects(&modal_bounds) {
                    sink.mark_dirty(*node_id);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Color, Modifier};

    use super::*;

    #[test]
    fn single_layer_paints_to_buffer() {
        let mut buf = Buffer::new(10, 5);
        let mut comp = Compositor::new();
        comp.add_layer(Rect::new(0, 0, 10, 5), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'A', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.composite(&mut buf);
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "A");
    }

    #[test]
    fn higher_z_layer_paints_on_top() {
        let mut buf = Buffer::new(10, 5);
        let mut comp = Compositor::new();
        comp.add_layer(Rect::new(0, 0, 10, 5), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(2, 2, 'X', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.add_layer(Rect::new(0, 0, 10, 5), 1, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(2, 2, 'Y', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.composite(&mut buf);
        assert_eq!(buf.get(2, 2).unwrap().as_str(), "Y");
    }

    #[test]
    fn non_overlapping_layers_both_paint() {
        let mut buf = Buffer::new(20, 5);
        let mut comp = Compositor::new();
        comp.add_layer(Rect::new(0, 0, 5, 5), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'L', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.add_layer(Rect::new(10, 0, 5, 5), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'R', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.composite(&mut buf);
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "L");
        assert_eq!(buf.get(10, 0).unwrap().as_str(), "R");
    }

    #[test]
    fn completely_obscured_layer_not_painted() {
        let mut buf = Buffer::new(10, 10);
        let mut comp = Compositor::new();
        comp.add_layer(Rect::new(2, 2, 3, 3), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'H', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.add_layer(Rect::new(0, 0, 10, 10), 1, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(2, 2, 'V', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.composite(&mut buf);
        assert_eq!(buf.get(2, 2).unwrap().as_str(), "V");
    }

    #[test]
    fn z_order_reversed_add_order() {
        let mut buf = Buffer::new(10, 5);
        let mut comp = Compositor::new();
        comp.add_layer(Rect::new(0, 0, 5, 5), 10, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'T', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.add_layer(Rect::new(0, 0, 5, 5), 0, |sv: &mut CanvasSubviewMut<'_>| {
            sv.set_char(0, 0, 'B', Color::Reset, Color::Reset, Modifier::empty());
        });
        comp.composite(&mut buf);
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "T");
    }

    #[test]
    fn layer_compositor_register_and_unregister() {
        let mut lc = LayerCompositor::new();
        lc.register_node(1, Rect::new(0, 0, 10, 10));
        lc.register_node(2, Rect::new(5, 5, 10, 10));
        assert_eq!(lc.spatial_index.len(), 2);

        lc.unregister_node(1);
        assert_eq!(lc.spatial_index.len(), 1);
        assert_eq!(lc.spatial_index[0].0, 2);
    }

    #[test]
    fn layer_compositor_register_updates_existing() {
        let mut lc = LayerCompositor::new();
        lc.register_node(1, Rect::new(0, 0, 10, 10));
        lc.register_node(1, Rect::new(5, 5, 20, 20));
        assert_eq!(lc.spatial_index.len(), 1);
        assert_eq!(lc.spatial_index[0].1, Rect::new(5, 5, 20, 20));
    }

    #[test]
    fn modal_handle_basics() {
        let mh = ModalHandle::new(42, Rect::new(10, 10, 20, 20));
        assert_eq!(mh.id, 42);
        assert_eq!(mh.computed_bounds(), Rect::new(10, 10, 20, 20));
    }

    #[test]
    fn set_and_get_active_modal() {
        let mut lc = LayerCompositor::new();
        assert!(lc.active_modal().is_none());
        lc.set_active_modal(ModalHandle::new(1, Rect::new(0, 0, 10, 10)));
        assert_eq!(lc.active_modal().unwrap().id, 1);
    }

    #[test]
    fn dismiss_modal_invalidates_intersecting_nodes() {
        let mut lc = LayerCompositor::new();
        lc.register_node(1, Rect::new(0, 0, 10, 10));
        lc.register_node(2, Rect::new(50, 50, 5, 5));
        lc.register_node(3, Rect::new(5, 5, 10, 10));
        lc.set_active_modal(ModalHandle::new(99, Rect::new(3, 3, 8, 8)));

        let mut dirty = Vec::new();
        lc.dismiss_modal(&mut |id: u64| dirty.push(id));

        assert!(dirty.contains(&1));
        assert!(dirty.contains(&3));
        assert!(!dirty.contains(&2));
        assert!(lc.active_modal().is_none());
    }

    #[test]
    fn dismiss_no_modal_is_noop() {
        let mut lc = LayerCompositor::new();
        lc.register_node(1, Rect::new(0, 0, 10, 10));
        let mut dirty = Vec::new();
        lc.dismiss_modal(&mut |id: u64| dirty.push(id));
        assert!(dirty.is_empty());
    }
}
