//! Spatial Drag-and-Drop Manager and Hit-Testing Dispatcher.

use std::path::PathBuf;

use crate::{buffer::Rect, dnd::PathSanitizer, geometry::Point, layout::NodeId};

/// Discrete events in the spatial drag-and-drop lifecycle.
#[derive(Clone, Debug, PartialEq)]
pub enum DragEvent {
    /// Dispatched when the mouse pointer first enters a registered drop node carrying a payload.
    DragEnter(NodeId),
    /// Dispatched continuously as the drag moves across the target container.
    DragOver(NodeId, Point),
    /// Dispatched when the pointer exits the target container bounds.
    DragLeave(NodeId),
    /// Dispatched when external files are released over the target container.
    FileDrop(NodeId, Vec<PathBuf>),
}

/// Registration descriptor for an active drop target in the spatial layout index.
#[derive(Clone, Debug, PartialEq)]
pub struct DropTargetEntry {
    pub node_id: NodeId,
    pub bounds: Rect,
    pub accepted_extensions: Option<Vec<String>>,
}

/// Central spatial drag-and-drop event bus mapping OS pointer coordinates
/// and paste payloads to target layout nodes.
#[derive(Default, Debug)]
pub struct DndManager {
    drop_targets: Vec<DropTargetEntry>,
    current_hover_target: Option<NodeId>,
    is_dragging: bool,
}

impl DndManager {
    pub fn new() -> Self {
        Self {
            drop_targets: Vec::new(),
            current_hover_target: None,
            is_dragging: false,
        }
    }

    /// Registers a spatial drop target with computed layout bounds.
    pub fn register_target(&mut self, entry: DropTargetEntry) {
        self.drop_targets.push(entry);
    }

    /// Convenience registration helper for node bounds.
    pub fn register_rect(&mut self, node_id: NodeId, bounds: Rect, accepted_extensions: Option<Vec<String>>) {
        self.drop_targets.push(DropTargetEntry {
            node_id,
            bounds,
            accepted_extensions,
        });
    }

    /// Clears all registered drop targets prior to a new frame layout pass.
    pub fn clear_targets(&mut self) {
        self.drop_targets.clear();
    }

    /// Evaluates mouse pointer coordinates to update hover states and dispatches lifecycle events.
    pub fn handle_pointer_move(&mut self, pt: Point) -> Option<DragEvent> {
        let matching_target = self.drop_targets.iter().rev().find(|target| {
            pt.x >= target.bounds.x
                && pt.x < target.bounds.x.saturating_add(target.bounds.width)
                && pt.y >= target.bounds.y
                && pt.y < target.bounds.y.saturating_add(target.bounds.height)
        });

        let new_hover = matching_target.map(|t| t.node_id);

        match (self.current_hover_target, new_hover) {
            (None, Some(new_id)) => {
                self.current_hover_target = Some(new_id);
                self.is_dragging = true;
                Some(DragEvent::DragEnter(new_id))
            }
            (Some(old_id), Some(new_id)) if old_id != new_id => {
                self.current_hover_target = Some(new_id);
                Some(DragEvent::DragEnter(new_id))
            }
            (Some(id), Some(_)) => Some(DragEvent::DragOver(id, pt)),
            (Some(old_id), None) => {
                self.current_hover_target = None;
                self.is_dragging = false;
                Some(DragEvent::DragLeave(old_id))
            }
            (None, None) => None,
        }
    }

    /// Intercepts drop payloads when hovering over an active drop zone and delivers
    /// sanitized file paths directly to the target node.
    pub fn handle_potential_drop(&mut self, payload: &str) -> Option<(NodeId, Vec<PathBuf>)> {
        if let Some(target_id) = self.current_hover_target {
            let paths = PathSanitizer::parse_drop_payload(payload);
            if !paths.is_empty() {
                self.current_hover_target = None;
                self.is_dragging = false;
                return Some((target_id, paths));
            }
        }
        None
    }

    /// Convenience method to position pointer and drop simultaneously.
    pub fn handle_drop_at(&mut self, pt: Point, payload: &str) -> Option<(NodeId, Vec<PathBuf>)> {
        self.handle_pointer_move(pt);
        self.handle_potential_drop(payload)
    }

    /// Returns whether a specific node is currently the active hover drop target.
    pub fn is_hovered(&self, node_id: NodeId) -> bool {
        self.current_hover_target == Some(node_id)
    }

    /// Returns the currently hovered target node ID, if any.
    pub fn current_hover_target(&self) -> Option<NodeId> {
        self.current_hover_target
    }

    /// Returns true if a drag operation is actively in flight over any target.
    pub fn is_dragging(&self) -> bool {
        self.is_dragging
    }
}
