#![forbid(unsafe_code)]
//! # Tenui Collab (`tenui-collab`)
//!
//! Multi-seat real-time collaboration, convergent sequence CRDT, and presence tracking.
//!
//! `tenui-collab` enables multi-user pair programming, collaborative buffer editing,
//! and remote cursor presence across distributed terminal sessions.
//!
//! ## Core Primitives
//!
//! - **[`TextCrdt`]**: Replicated sequence CRDT based on causal tree / RGA semantics.
//!   Guarantees eventual convergence across replicas regardless of operation arrival order,
//!   with idempotent op application and automatic dependency buffering.
//! - **[`CrdtOp`] & [`OpId`]**: Globally unique, totally ordered operation identifiers and mutation
//!   records exchanged between collaborating peers.
//! - **[`MultiSeatManager`] & [`SeatPresence`]**: Presence management tracking active user seats,
//!   assigned color identities, selection bounds, and remote caret coordinates.
//! - **[`CollaborativeText`]**: High-level editor adapter tying together local keystrokes, CRDT synchronization,
//!   and remote peer carets.
//!
//! ## Runnable Example: Convergent Multi-Site Editing
//!
//! ```rust
//! use tenui_collab::TextCrdt;
//!
//! // Create two independent replicas with distinct site IDs
//! let mut site_a = TextCrdt::new(1);
//! let mut site_b = TextCrdt::new(2);
//!
//! // Site A inserts text locally
//! let op_a = site_a.local_insert(0, 'A');
//!
//! // Site B inserts text locally
//! let op_b = site_b.local_insert(0, 'B');
//!
//! // Replicas exchange operations
//! site_b.apply(op_a);
//! site_a.apply(op_b);
//!
//! // Both replicas converge to the exact same text
//! assert_eq!(site_a.text(), site_b.text());
//! ```

pub mod crdt;
pub mod editor;
pub mod seat;

pub use crdt::{CrdtOp, OpId, TextCrdt};
pub use editor::CollaborativeText;
pub use seat::{CollabOperation, MultiSeatManager, SeatPresence};
