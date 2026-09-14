//! Spatial Mouse Hit-Testing, Dispatcher, and Event Bubbling Subsystem.

use std::time::Instant;

use crate::{
    buffer::Rect,
    geometry::Point,
    layout::NodeId,
    mouse::{MouseButton, MouseEvent},
};

/// Registration descriptor for an active mouse interaction target.
pub struct MouseTargetEntry {
    pub node_id: NodeId,
    pub bounds: Rect,
    pub z_index: u16,
    pub on_click: Option<Box<dyn Fn(Point) + Send + Sync>>,
    pub on_drag: Option<Box<dyn Fn(Point, Point) + Send + Sync>>,
    pub on_down: Option<Box<dyn Fn(MouseButton, Point) + Send + Sync>>,
    pub on_up: Option<Box<dyn Fn(MouseButton, Point) + Send + Sync>>,
    pub on_enter: Option<Box<dyn Fn() + Send + Sync>>,
    pub on_leave: Option<Box<dyn Fn() + Send + Sync>>,
}

impl MouseTargetEntry {
    pub fn new(node_id: NodeId, bounds: Rect) -> Self {
        Self {
            node_id,
            bounds,
            z_index: 0,
            on_click: None,
            on_drag: None,
            on_down: None,
            on_up: None,
            on_enter: None,
            on_leave: None,
        }
    }

    pub fn with_z_index(mut self, z_index: u16) -> Self {
        self.z_index = z_index;
        self
    }

    pub fn with_on_click<F: Fn(Point) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_click = Some(Box::new(f));
        self
    }

    pub fn with_on_drag<F: Fn(Point, Point) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_drag = Some(Box::new(f));
        self
    }

    pub fn with_on_down<F: Fn(MouseButton, Point) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_down = Some(Box::new(f));
        self
    }

    pub fn with_on_up<F: Fn(MouseButton, Point) + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_up = Some(Box::new(f));
        self
    }

    pub fn with_on_enter<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_enter = Some(Box::new(f));
        self
    }

    pub fn with_on_leave<F: Fn() + Send + Sync + 'static>(mut self, f: F) -> Self {
        self.on_leave = Some(Box::new(f));
        self
    }
}

/// Central spatial mouse event dispatcher with Z-index sorting,
/// modal barriers, :hover / :active state tracking, and click synthesis.
pub struct MouseDispatcher {
    targets: Vec<MouseTargetEntry>,
    current_hover: Option<NodeId>,
    pressed_target: Option<(NodeId, MouseButton, Point, Instant)>,
    last_click: Option<(NodeId, MouseButton, Point, Instant)>,
    active_modal: Option<(NodeId, Rect)>,
    on_outside_click: Option<Box<dyn Fn(Point) + Send + Sync>>,
    is_any_event_promoted: bool,
}

impl Default for MouseDispatcher {
    fn default() -> Self {
        Self::new()
    }
}

impl MouseDispatcher {
    pub fn new() -> Self {
        Self {
            targets: Vec::new(),
            current_hover: None,
            pressed_target: None,
            last_click: None,
            active_modal: None,
            on_outside_click: None,
            is_any_event_promoted: false,
        }
    }

    pub fn register_target(&mut self, entry: MouseTargetEntry) {
        self.targets.push(entry);
    }

    pub fn clear_targets(&mut self) {
        self.targets.clear();
    }

    /// Sets an active modal barrier. While active, clicks outside the modal
    /// boundary fire the outside click callback and suppress underlying targets.
    pub fn set_modal_barrier(&mut self, node_id: NodeId, bounds: Rect) {
        self.active_modal = Some((node_id, bounds));
    }

    pub fn clear_modal_barrier(&mut self) {
        self.active_modal = None;
    }

    pub fn set_on_outside_click<F: Fn(Point) + Send + Sync + 'static>(&mut self, f: F) {
        self.on_outside_click = Some(Box::new(f));
    }

    /// Evaluates point hit-testing across registered targets in reverse Z-order.
    pub fn hit_test(&self, pt: Point) -> Option<&MouseTargetEntry> {
        // If an active modal barrier is present and pointer is outside its bounds,
        // underlying targets are obscured and not hit.
        if let Some((_, modal_bounds)) = self.active_modal
            && !modal_bounds.contains_point(pt)
        {
            return None;
        }

        self.targets
            .iter()
            .enumerate()
            .filter(|(_, t)| t.bounds.contains_point(pt))
            .max_by_key(|(idx, t)| (t.z_index, *idx))
            .map(|(_, t)| t)
    }

    /// Dispatches an incoming mouse event through spatial hit-testing,
    /// updating hover state, synthesizing clicks, and handling drag gestures.
    pub fn dispatch(&mut self, event: MouseEvent) -> Option<MouseEvent> {
        match event {
            MouseEvent::Down { button, position } => {
                // Modal barrier check
                if let Some((_, modal_bounds)) = self.active_modal
                    && !modal_bounds.contains_point(position)
                {
                    if let Some(cb) = self.on_outside_click.as_ref() {
                        cb(position);
                    }
                    return None;
                }

                if let Some(target) = self.hit_test(position) {
                    let target_id = target.node_id;
                    if let Some(cb) = target.on_down.as_ref() {
                        cb(button, position);
                    }
                    self.pressed_target = Some((target_id, button, position, Instant::now()));
                    return Some(MouseEvent::Down { button, position });
                }
                None
            }

            MouseEvent::Up { button, position } => {
                if let Some((pressed_id, pressed_btn, start_pos, start_time)) = self.pressed_target.take()
                    && pressed_btn == button
                    && let Some(target) = self.targets.iter().find(|t| t.node_id == pressed_id)
                {
                    if let Some(cb) = target.on_up.as_ref() {
                        cb(button, position);
                    }

                    let dist = start_pos.distance(position);
                    let elapsed = start_time.elapsed().as_millis();

                    // Synthesize Click if released within threshold (<= 1.5 cells, <= 350ms)
                    if dist <= 1.5 && elapsed <= 350 {
                        if let Some(cb) = target.on_click.as_ref() {
                            cb(position);
                        }

                        // Check double-click synthesis
                        let is_double = if let Some((last_id, last_btn, last_pos, last_time)) = self.last_click {
                            last_id == pressed_id
                                && last_btn == button
                                && last_pos.distance(position) <= 1.5
                                && last_time.elapsed().as_millis() <= 400
                        } else {
                            false
                        };

                        self.last_click = Some((pressed_id, button, position, Instant::now()));

                        if is_double {
                            return Some(MouseEvent::DoubleClick { button, position });
                        } else {
                            return Some(MouseEvent::Click { button, position });
                        }
                    }
                }
                Some(MouseEvent::Up { button, position })
            }

            MouseEvent::Move { position } => {
                let hit_node = self.hit_test(position).map(|t| t.node_id);

                // Hover state machine (Enter / Leave transitions)
                if hit_node != self.current_hover {
                    if let Some(old_id) = self.current_hover
                        && let Some(target) = self.targets.iter().find(|t| t.node_id == old_id)
                        && let Some(cb) = target.on_leave.as_ref()
                    {
                        cb();
                    }
                    if let Some(new_id) = hit_node
                        && let Some(target) = self.targets.iter().find(|t| t.node_id == new_id)
                        && let Some(cb) = target.on_enter.as_ref()
                    {
                        cb();
                    }
                    self.current_hover = hit_node;
                }

                // If a button is pressed and movement exceeds 1.5 cells, synthesize Drag
                if let Some((pressed_id, pressed_btn, start_pos, _)) = self.pressed_target
                    && start_pos.distance(position) > 1.5
                {
                    if let Some(target) = self.targets.iter().find(|t| t.node_id == pressed_id)
                        && let Some(cb) = target.on_drag.as_ref()
                    {
                        cb(start_pos, position);
                    }
                    return Some(MouseEvent::Drag {
                        button: pressed_btn,
                        start: start_pos,
                        current: position,
                        delta: Point::new(
                            position.x.saturating_sub(start_pos.x),
                            position.y.saturating_sub(start_pos.y),
                        ),
                    });
                }

                Some(MouseEvent::Move { position })
            }

            MouseEvent::Scroll {
                delta_x,
                delta_y,
                is_pixel,
            } => Some(MouseEvent::Scroll {
                delta_x,
                delta_y,
                is_pixel,
            }),

            _ => None,
        }
    }

    /// Returns the currently hovered node ID, if any.
    pub fn current_hover(&self) -> Option<NodeId> {
        self.current_hover
    }

    /// Returns true if a specific node is currently hovered.
    pub fn is_hovered(&self, node_id: NodeId) -> bool {
        self.current_hover == Some(node_id)
    }

    /// Returns true if a specific node is currently pressed (:active).
    pub fn is_pressed(&self, node_id: NodeId) -> bool {
        self.pressed_target
            .as_ref()
            .map(|(id, _, _, _)| *id == node_id)
            .unwrap_or(false)
    }

    /// Adaptive motion tracking gate: returns true if high-frequency Any-Event (1003h)
    /// tracking is required (i.e. when hover or drag listeners are active).
    pub fn needs_any_event_tracking(&self) -> bool {
        self.is_any_event_promoted
            || self.pressed_target.is_some()
            || self
                .targets
                .iter()
                .any(|t| t.on_enter.is_some() || t.on_leave.is_some() || t.on_drag.is_some())
    }

    pub fn set_any_event_promoted(&mut self, promoted: bool) {
        self.is_any_event_promoted = promoted;
    }
}
