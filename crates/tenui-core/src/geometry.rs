//! 2D geometry primitives for spatial layout and hit testing.

pub use crate::buffer::Rect;

/// A 2D point in terminal cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Point {
    pub x: u16,
    pub y: u16,
}

impl Point {
    pub const ZERO: Self = Self { x: 0, y: 0 };

    pub const fn new(x: u16, y: u16) -> Self {
        Self { x, y }
    }

    /// Computes Euclidean distance between two points.
    pub fn distance(&self, other: Point) -> f32 {
        let dx = (self.x as f32) - (other.x as f32);
        let dy = (self.y as f32) - (other.y as f32);
        (dx * dx + dy * dy).sqrt()
    }
}
