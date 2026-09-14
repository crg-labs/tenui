#![forbid(unsafe_code)]
//! # Tenui Virt (`tenui-virt`)
//!
//! Heterogeneous dynamic-height list virtualization with $O(\log N)$ prefix-sum indexing.
//!
//! `tenui-virt` powers virtualized views containing items with variable, dynamic row heights
//! (such as chat transcripts, collapsible code blocks, and mixed-height feeds):
//!
//! ## Core Primitives
//!
//! - **[`FenwickTree`]**: Binary Indexed Tree maintaining running prefix sums of item heights.
//!   Provides $O(\log N)$ point updates and $O(\log N)$ binary search to find visible item ranges.
//! - **[`VirtualList<T>`]**: Virtualized collection manager that computes visible slice ranges
//!   ([`VirtualList::resolve_visible_range`]) and maintains continuous scroll anchoring when item heights
//!   mutate above the active viewport.
//!
//! ## Runnable Example: Variable Height Virtualization
//!
//! ```rust
//! use tenui_virt::VirtualList;
//!
//! let items = vec!["Short message", "Longer multi-line message", "Another short one"];
//! let mut list = VirtualList::new(items, 1);
//!
//! // Item 1 expands to 5 rows (e.g. multi-line code block)
//! list.set_item_height(1, 5);
//!
//! // Total virtual height: 1 + 5 + 1 = 7 rows
//! assert_eq!(list.total_height(), 7);
//!
//! // Resolve visible item indices for a 4-row viewport
//! let (start_idx, end_idx, sub_offset) = list.resolve_visible_range(4);
//! assert_eq!(start_idx, 0);
//! ```

pub mod list;

pub use list::{FenwickTree, VirtualList};
