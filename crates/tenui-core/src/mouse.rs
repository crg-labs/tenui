//! Mouse Event Taxonomy and Subsystem Types.

use crate::{geometry::Point, layout::NodeId};

/// Button identifier in the pointer tracking subsystem.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum MouseButton {
    Left,
    Middle,
    Right,
    WheelUp,
    WheelDown,
    WheelLeft,
    WheelRight,
}

/// Unified mouse events dispatched across the spatial hit-test tree.
#[derive(Clone, Debug, PartialEq)]
pub enum MouseEvent {
    /// Dispatched when a mouse button is depressed.
    Down { button: MouseButton, position: Point },
    /// Dispatched when a mouse button is released.
    Up { button: MouseButton, position: Point },
    /// Synthesized when a button is depressed and released within the click threshold (<= 1.5 cells, <= 350ms).
    Click { button: MouseButton, position: Point },
    /// Synthesized when two clicks occur within the double-click threshold.
    DoubleClick { button: MouseButton, position: Point },
    /// Continuous mouse motion without buttons held.
    Move { position: Point },
    /// Dragging motion while a mouse button is held down beyond the drag threshold.
    Drag {
        button: MouseButton,
        start: Point,
        current: Point,
        delta: Point,
    },
    /// Scroll wheel interaction with horizontal and vertical delta components.
    Scroll { delta_x: f32, delta_y: f32, is_pixel: bool },
    /// Synthesized when the pointer crosses into a node's spatial bounding box.
    Enter { node_id: NodeId },
    /// Synthesized when the pointer crosses out of a node's spatial bounding box.
    Leave { node_id: NodeId },
}

impl From<crossterm::event::MouseEvent> for MouseEvent {
    fn from(ev: crossterm::event::MouseEvent) -> Self {
        let position = Point::new(ev.column, ev.row);
        match ev.kind {
            crossterm::event::MouseEventKind::Down(btn) => MouseEvent::Down {
                button: match btn {
                    crossterm::event::MouseButton::Left => MouseButton::Left,
                    crossterm::event::MouseButton::Middle => MouseButton::Middle,
                    crossterm::event::MouseButton::Right => MouseButton::Right,
                },
                position,
            },
            crossterm::event::MouseEventKind::Up(btn) => MouseEvent::Up {
                button: match btn {
                    crossterm::event::MouseButton::Left => MouseButton::Left,
                    crossterm::event::MouseButton::Middle => MouseButton::Middle,
                    crossterm::event::MouseButton::Right => MouseButton::Right,
                },
                position,
            },
            crossterm::event::MouseEventKind::Drag(btn) => MouseEvent::Drag {
                button: match btn {
                    crossterm::event::MouseButton::Left => MouseButton::Left,
                    crossterm::event::MouseButton::Middle => MouseButton::Middle,
                    crossterm::event::MouseButton::Right => MouseButton::Right,
                },
                start: position,
                current: position,
                delta: Point::ZERO,
            },
            crossterm::event::MouseEventKind::Moved => MouseEvent::Move { position },
            crossterm::event::MouseEventKind::ScrollDown => MouseEvent::Scroll {
                delta_x: 0.0,
                delta_y: 1.0,
                is_pixel: false,
            },
            crossterm::event::MouseEventKind::ScrollUp => MouseEvent::Scroll {
                delta_x: 0.0,
                delta_y: -1.0,
                is_pixel: false,
            },
            crossterm::event::MouseEventKind::ScrollLeft => MouseEvent::Scroll {
                delta_x: -1.0,
                delta_y: 0.0,
                is_pixel: false,
            },
            crossterm::event::MouseEventKind::ScrollRight => MouseEvent::Scroll {
                delta_x: 1.0,
                delta_y: 0.0,
                is_pixel: false,
            },
        }
    }
}
