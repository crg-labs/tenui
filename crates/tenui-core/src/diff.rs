use crate::{buffer::Buffer, cell::Cell};

/// A difference item between two buffers.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CellDiff<'a> {
    pub x: u16,
    pub y: u16,
    pub previous: Option<&'a Cell>,
    pub current: &'a Cell,
}

/// Computes differences between two buffers. Useful for headless test assertions
/// and analytics.
pub struct BufferDiff<'a> {
    pub changes: Vec<CellDiff<'a>>,
}

impl<'a> BufferDiff<'a> {
    pub fn compute(front: &'a Buffer, back: &'a Buffer) -> Self {
        let mut changes = Vec::new();
        let width = back.width.min(front.width);
        let height = back.height.min(front.height);

        for y in 0..height {
            for x in 0..width {
                let current = match back.get(x, y) {
                    Some(c) => c,
                    None => continue,
                };
                let previous = front.get(x, y);

                if previous != Some(current) {
                    changes.push(CellDiff {
                        x,
                        y,
                        previous,
                        current,
                    });
                }
            }
        }

        Self { changes }
    }

    pub fn is_empty(&self) -> bool {
        self.changes.is_empty()
    }

    pub fn count(&self) -> usize {
        self.changes.len()
    }
}
