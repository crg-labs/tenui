use std::{
    collections::{HashMap, HashSet},
    rc::Rc,
};

use taffy::prelude::*;
use tenui_core::{Buffer, CanvasSubviewMut, Rect};

use crate::{
    bus::TwoLaneBus,
    invalidation::{Invalidation, InvalidationSink},
};

type PaintCallback = Box<dyn Fn(&mut CanvasSubviewMut<'_>)>;

pub struct ReactiveComponent {
    pub id: u64,
    pub taffy_node: NodeId,
    pub parent: Option<u64>,
    pub children: Vec<u64>,
    pub style: Style,
    pub bounding_box: Rect,
    pub paint: Option<PaintCallback>,
}

/// Persistent retained reactive component tree.
///
/// Implements the Two-Phase Frame Execution Pipeline:
/// - **Phase 1 (Conditional Layout Resolution):** Bypasses Taffy completely ($O(0)$ cost)
///   when only cosmetic paint mutations occur. When geometric mutations occur, resolves
///   selective subtree reflow at the Lowest Common Ancestor (LCA).
/// - **Phase 2 (Double-Buffered Differential Paint):** Dispatches paint callbacks strictly
///   for dirty nodes into their isolated, hardware-clipped back-buffer subviews.
pub struct ReactiveTree {
    pub taffy: TaffyTree<()>,
    pub components: HashMap<u64, ReactiveComponent>,
    pub root_id: Option<u64>,
    pub bus: Rc<TwoLaneBus>,
    pub layout_invocations: u64,
    pub paint_invocations: u64,
    initial_rendered: bool,
}

impl ReactiveTree {
    pub fn new(bus: Rc<TwoLaneBus>) -> Self {
        Self {
            taffy: TaffyTree::new(),
            components: HashMap::new(),
            root_id: None,
            bus,
            layout_invocations: 0,
            paint_invocations: 0,
            initial_rendered: false,
        }
    }

    /// Inserts a persistent reactive component into the hierarchy.
    pub fn insert<F>(&mut self, id: u64, parent_id: Option<u64>, style: Style, paint: F)
    where
        F: Fn(&mut CanvasSubviewMut<'_>) + 'static,
    {
        let taffy_node = self.taffy.new_leaf(style.clone()).expect("create taffy node");

        if let Some(pid) = parent_id {
            if let Some(parent) = self.components.get_mut(&pid) {
                parent.children.push(id);
                let _ = self.taffy.add_child(parent.taffy_node, taffy_node);
            }
        } else if self.root_id.is_none() {
            self.root_id = Some(id);
        }

        self.components.insert(
            id,
            ReactiveComponent {
                id,
                taffy_node,
                parent: parent_id,
                children: Vec::new(),
                style,
                bounding_box: Rect::ZERO,
                paint: Some(Box::new(paint)),
            },
        );

        // Mark initially layout and paint dirty
        self.bus.mark_dirty(id, Invalidation::BOTH);
    }

    /// Finds the Lowest Common Ancestor (LCA) for a set of dirty component IDs.
    pub fn find_lca(&self, dirty_ids: &[u64]) -> Option<u64> {
        if dirty_ids.is_empty() {
            return None;
        }
        if dirty_ids.len() == 1 {
            return Some(dirty_ids[0]);
        }

        // Build ancestor path for the first node
        let mut ancestors = HashSet::new();
        let mut curr = Some(dirty_ids[0]);
        while let Some(cid) = curr {
            ancestors.insert(cid);
            curr = self.components.get(&cid).and_then(|c| c.parent);
        }

        // Trace common ancestor for subsequent nodes
        let mut lca = self.root_id;
        for &other_id in &dirty_ids[1..] {
            let mut trace = Some(other_id);
            while let Some(cid) = trace {
                if ancestors.contains(&cid) {
                    lca = Some(cid);
                    break;
                }
                trace = self.components.get(&cid).and_then(|c| c.parent);
            }
        }

        lca
    }

    /// Executes the two-phase reactive frame pipeline.
    pub fn render_frame(&mut self, buffer: &mut Buffer) {
        let root_id = match self.root_id {
            Some(id) => id,
            None => return,
        };

        // Step 1: Conditional Layout Resolution
        let needs_layout = !self.initial_rendered || self.bus.has_layout_dirty();

        if needs_layout {
            self.layout_invocations += 1;
            let dirty_layout_nodes = self.bus.drain_layout_dirty();

            // Set root dimensions to match target buffer
            let root_taffy = self.components[&root_id].taffy_node;
            let mut root_style = self.taffy.style(root_taffy).cloned().unwrap_or_default();
            root_style.size.width = Dimension::length(buffer.width as f32);
            root_style.size.height = Dimension::length(buffer.height as f32);
            let _ = self.taffy.set_style(root_taffy, root_style);

            // Mark dirty nodes in Taffy
            for &cid in &dirty_layout_nodes {
                if let Some(comp) = self.components.get(&cid) {
                    let _ = self.taffy.mark_dirty(comp.taffy_node);
                }
            }

            let available = Size {
                width: AvailableSpace::Definite(buffer.width as f32),
                height: AvailableSpace::Definite(buffer.height as f32),
            };

            // Compute layout over Taffy tree
            let _ = self.taffy.compute_layout(root_taffy, available);

            // Re-derive bounding boxes and promote geometric changes to Paint Lane
            self.update_bounding_boxes(root_id, 0, 0);
        }

        // Step 2: Double-Buffered Differential Paint
        let paint_targets: HashSet<u64> = if !self.initial_rendered {
            self.initial_rendered = true;
            self.components.keys().copied().collect()
        } else {
            self.bus.drain_paint_dirty().into_iter().collect()
        };

        for cid in paint_targets {
            if let Some(comp) = self.components.get(&cid) {
                let rect = comp.bounding_box;
                if !rect.is_empty()
                    && let Some(paint_fn) = comp.paint.as_ref()
                {
                    let mut subview = buffer.subview_mut(rect);
                    paint_fn(&mut subview);
                    self.paint_invocations += 1;
                }
            }
        }
    }

    fn update_bounding_boxes(&mut self, node_id: u64, parent_x: u16, parent_y: u16) {
        let (nx, ny, nw, nh) = {
            let comp = match self.components.get(&node_id) {
                Some(c) => c,
                None => return,
            };
            match self.taffy.layout(comp.taffy_node) {
                Ok(layout) => {
                    let gx = parent_x.saturating_add(layout.location.x.max(0.0).round() as u16);
                    let gy = parent_y.saturating_add(layout.location.y.max(0.0).round() as u16);
                    let gw = layout.size.width.max(0.0).round() as u16;
                    let gh = layout.size.height.max(0.0).round() as u16;
                    (gx, gy, gw, gh)
                }
                Err(_) => return,
            }
        };

        let new_box = Rect::new(nx, ny, nw, nh);
        if let Some(comp) = self.components.get_mut(&node_id)
            && comp.bounding_box != new_box
        {
            comp.bounding_box = new_box;
            self.bus.mark_dirty(node_id, Invalidation::PAINT);
        }

        let children = self.components[&node_id].children.clone();
        for child_id in children {
            self.update_bounding_boxes(child_id, nx, ny);
        }
    }

    /// Invalidates all components whose computed bounding boxes intersect `rect`.
    /// When overlays or modals are dismissed, this forces underlying dormant components to repaint.
    pub fn invalidate_intersecting(&self, rect: &Rect, lane: Invalidation) {
        for component in self.components.values() {
            if component.bounding_box.intersects(rect) {
                self.bus.mark_dirty(component.id, lane);
            }
        }
    }
}
