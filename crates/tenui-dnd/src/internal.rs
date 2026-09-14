use std::any::Any;

use tenui_core::{
    canvas::CanvasSubviewMut,
    color::Color,
    geometry::{Point, Rect},
    layout::NodeId,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DockPlacement {
    SplitLeft,
    SplitRight,
    SplitTop,
    SplitBottom,
    CenterDeck,
}

pub trait DragSource: Any {
    fn item_id(&self) -> u64;
    fn visual_footprint(&self) -> (u16, u16);
    fn render_ghost(&self, surface: &mut CanvasSubviewMut);
}

pub trait DropConsumer: Any {
    fn accept_dock(&mut self, source_id: u64, placement: DockPlacement);
    fn accept_reorder(&mut self, source_id: u64, target_index: usize);
}

#[derive(Clone, Debug)]
pub struct InternalDragSession {
    pub source_node: NodeId,
    pub source_id: u64,
    pub grab_offset: Point,
    pub current_pos: Point,
    pub ghost_size: (u16, u16),
}

impl InternalDragSession {
    /// Compute the hoisted ghost bounding box:
    /// B(G) = [x_p - \delta_x, y_p - \delta_y, W_G, H_G]
    pub fn ghost_bounding_box(&self) -> Rect {
        Rect {
            x: self.current_pos.x.saturating_sub(self.grab_offset.x),
            y: self.current_pos.y.saturating_sub(self.grab_offset.y),
            width: self.ghost_size.0,
            height: self.ghost_size.1,
        }
    }
}

/// The outcome of ending a drag: the drag session, plus an optional drop target
/// (node, its rect, and the resolved dock placement).
pub type DropResult = (InternalDragSession, Option<(NodeId, Rect, DockPlacement)>);

pub struct DockManager {
    pub active_session: Option<InternalDragSession>,
    pub hover_candidate: Option<(NodeId, Rect, DockPlacement)>,
}

impl DockManager {
    pub const ASPECT_RATIO: f32 = 2.0;
    pub const SNAP_THRESHOLD: f32 = 4.0;

    pub fn new() -> Self {
        Self {
            active_session: None,
            hover_candidate: None,
        }
    }

    pub fn is_dragging(&self) -> bool {
        self.active_session.is_some()
    }

    pub fn start_drag(&mut self, source_node: NodeId, source_id: u64, grab_pt: Point, size: (u16, u16)) {
        self.active_session = Some(InternalDragSession {
            source_node,
            source_id,
            grab_offset: grab_pt,
            current_pos: grab_pt,
            ghost_size: size,
        });
        self.hover_candidate = None;
    }

    pub fn update_pointer(&mut self, pt: Point, target_containers: &[(NodeId, Rect)]) {
        if let Some(session) = &mut self.active_session {
            session.current_pos = pt;
            self.hover_candidate = None;

            for &(node_id, rect) in target_containers {
                if node_id == session.source_node {
                    continue;
                }
                if rect.contains(pt.x, pt.y) {
                    let placement = Self::resolve_placement(pt, rect);
                    self.hover_candidate = Some((node_id, rect, placement));
                    break;
                }
            }
        }
    }

    pub fn resolve_placement(pt: Point, rect: Rect) -> DockPlacement {
        let px = pt.x as f32;
        let py = pt.y as f32;
        let rx = rect.x as f32;
        let ry = rect.y as f32;
        let rw = rect.width as f32;
        let rh = rect.height as f32;

        let d_left = px - rx;
        let d_right = (rx + rw) - px;
        let d_top = (py - ry) * Self::ASPECT_RATIO;
        let d_bottom = ((ry + rh) - py) * Self::ASPECT_RATIO;

        let min_d = d_left.min(d_right).min(d_top).min(d_bottom);

        if min_d <= Self::SNAP_THRESHOLD {
            if (min_d - d_top).abs() < f32::EPSILON {
                DockPlacement::SplitTop
            } else if (min_d - d_bottom).abs() < f32::EPSILON {
                DockPlacement::SplitBottom
            } else if (min_d - d_left).abs() < f32::EPSILON {
                DockPlacement::SplitLeft
            } else {
                DockPlacement::SplitRight
            }
        } else {
            DockPlacement::CenterDeck
        }
    }

    /// End active drag session and return the drop target if any.
    pub fn end_drag(&mut self) -> Option<DropResult> {
        let session = self.active_session.take()?;
        let candidate = self.hover_candidate.take();
        Some((session, candidate))
    }

    pub fn cancel_drag(&mut self) {
        self.active_session = None;
        self.hover_candidate = None;
    }

    /// Calculate analytical spring offset for vertical list separation gaps:
    /// \Delta y_j = H_G * (1.0 - (-lambda * t).exp())
    pub fn calculate_spring_separation(ghost_height: f32, elapsed_secs: f32, lambda: f32) -> f32 {
        ghost_height * (1.0 - (-lambda * elapsed_secs).exp()).clamp(0.0, 1.0)
    }

    pub fn render_preview_overlay(&self, surface: &mut CanvasSubviewMut) {
        if let Some((_, rect, placement)) = self.hover_candidate {
            let preview_rect = match placement {
                DockPlacement::SplitLeft => Rect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width / 2,
                    height: rect.height,
                },
                DockPlacement::SplitRight => Rect {
                    x: rect.x + rect.width / 2,
                    y: rect.y,
                    width: rect.width / 2,
                    height: rect.height,
                },
                DockPlacement::SplitTop => Rect {
                    x: rect.x,
                    y: rect.y,
                    width: rect.width,
                    height: rect.height / 2,
                },
                DockPlacement::SplitBottom => Rect {
                    x: rect.x,
                    y: rect.y + rect.height / 2,
                    width: rect.width,
                    height: rect.height / 2,
                },
                DockPlacement::CenterDeck => rect,
            };

            // Stipple shade the target drop region
            let col_start = preview_rect.x;
            let row_start = preview_rect.y;
            let col_end = preview_rect.x + preview_rect.width;
            let row_end = preview_rect.y + preview_rect.height;

            for y in row_start..row_end {
                for x in col_start..col_end {
                    if let Some(cell) = surface.get_cell_mut(x, y) {
                        cell.bg = cell.bg.lerp(Color::CYAN, 0.25);
                        if cell.symbol.as_str() == " " {
                            cell.set_char('░');
                            cell.fg = Color::CYAN;
                        }
                    }
                }
            }
        }
    }
}

impl Default for DockManager {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_dock_edge_snap_thresholds() {
        let container = Rect {
            x: 10,
            y: 5,
            width: 40,
            height: 20,
        };

        // Point near left border (x = 11, y = 15)
        let pt_left = Point { x: 11, y: 15 };
        assert_eq!(
            DockManager::resolve_placement(pt_left, container),
            DockPlacement::SplitLeft
        );

        // Point in deep center (x = 30, y = 15)
        let pt_center = Point { x: 30, y: 15 };
        assert_eq!(
            DockManager::resolve_placement(pt_center, container),
            DockPlacement::CenterDeck
        );

        // Point near top border (x = 25, y = 6) -> d_top = (6 - 5) * 2 = 2 <= 4
        let pt_top = Point { x: 25, y: 6 };
        assert_eq!(
            DockManager::resolve_placement(pt_top, container),
            DockPlacement::SplitTop
        );

        // Point near bottom border (x = 25, y = 24) -> d_bottom = (25 - 24) * 2 = 2 <= 4
        let pt_bottom = Point { x: 25, y: 24 };
        assert_eq!(
            DockManager::resolve_placement(pt_bottom, container),
            DockPlacement::SplitBottom
        );

        // Point near right border (x = 48, y = 15) -> d_right = 50 - 48 = 2 <= 4
        let pt_right = Point { x: 48, y: 15 };
        assert_eq!(
            DockManager::resolve_placement(pt_right, container),
            DockPlacement::SplitRight
        );
    }

    #[test]
    fn test_drag_session_lifecycle() {
        let mut dm = DockManager::new();
        assert!(!dm.is_dragging());

        let node = NodeId::new(1);
        dm.start_drag(node, 42, Point { x: 5, y: 5 }, (10, 4));
        assert!(dm.is_dragging());

        let target_rect = Rect {
            x: 20,
            y: 10,
            width: 30,
            height: 20,
        };
        let target_node = NodeId::new(2);

        // Move over candidate
        dm.update_pointer(Point { x: 21, y: 20 }, &[(target_node, target_rect)]);
        assert!(dm.hover_candidate.is_some());
        let (cand_node, _, placement) = dm.hover_candidate.unwrap();
        assert_eq!(cand_node, target_node);
        assert_eq!(placement, DockPlacement::SplitLeft);

        // End drag
        let (session, drop_result) = dm.end_drag().unwrap();
        assert_eq!(session.source_id, 42);
        assert!(drop_result.is_some());
        assert!(!dm.is_dragging());
    }

    #[test]
    fn test_spring_separation() {
        let gap_0 = DockManager::calculate_spring_separation(4.0, 0.0, 10.0);
        assert_eq!(gap_0, 0.0);

        let gap_later = DockManager::calculate_spring_separation(4.0, 1.0, 10.0);
        assert!(gap_later > 3.9);
    }
}
