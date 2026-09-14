#![forbid(unsafe_code)]
//! # Tenui DnD (`tenui-dnd`)
//!
//! Internal spatial drag-and-drop and interactive pane docking engine.
//!
//! `tenui-dnd` provides mouse-driven drag-and-drop interactions for tiling window managers,
//! tab reordering, and panel docking:
//!
//! ## Core Primitives
//!
//! - **[`DockPlacement`]**: Cardinal docking zones ([`DockPlacement::SplitLeft`], [`DockPlacement::SplitRight`],
//!   [`DockPlacement::SplitTop`], [`DockPlacement::SplitBottom`], [`DockPlacement::CenterDeck`]).
//! - **[`InternalDragSession`]**: Active drag transaction tracking grab offsets, current pointer coordinates,
//!   and ghost preview bounding boxes ([`InternalDragSession::ghost_bounding_box`]).
//! - **[`DockManager`]**: Spatial collision evaluator determining dock snap zones based on anisotropic
//!   aspect-ratio-adjusted Euclidean distance thresholds.
//! - **[`DragSource`] & [`DropConsumer`]**: Traits defining drag capabilities and drop acceptance handlers.
//!
//! ## Runnable Example: Ghost Box Positioning
//!
//! ```rust
//! use tenui_core::{Point, Rect};
//! use tenui_dnd::InternalDragSession;
//!
//! let session = InternalDragSession {
//!     source_node: tenui_core::NodeId(1),
//!     source_id: 42,
//!     grab_offset: Point { x: 2, y: 1 },
//!     current_pos: Point { x: 10, y: 5 },
//!     ghost_size: (12, 4),
//! };
//!
//! let ghost_box = session.ghost_bounding_box();
//! assert_eq!(ghost_box, Rect::new(8, 4, 12, 4));
//! ```

pub mod internal;

pub use internal::{DockManager, DockPlacement, DragSource, DropConsumer, InternalDragSession};
