use bitflags::bitflags;

bitflags! {
    /// Two-lane invalidation flags distinguishing cosmetic paint changes
    /// from structural geometric reflows.
    #[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
    pub struct Invalidation: u8 {
        const NONE   = 0b00;
        const PAINT  = 0b01;
        const LAYOUT = 0b10;
        const BOTH   = Self::PAINT.bits() | Self::LAYOUT.bits();
    }
}

/// Sink that receives invalidation events when reactive nodes mutate.
pub trait InvalidationSink {
    fn mark_dirty(&self, node_id: u64, flags: Invalidation);
}
