//! Spatial layout node identifiers and hit-testing primitives.

/// A unique identifier for a layout node or container in the visual hierarchy.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, PartialOrd, Ord)]
pub struct NodeId(pub usize);

impl NodeId {
    pub const fn new(id: usize) -> Self {
        Self(id)
    }

    pub const fn id(&self) -> usize {
        self.0
    }
}
