#![forbid(unsafe_code)]
//! # Tenui Reactive (`tenui-reactive`)
//!
//! Fine-grained reactive signal graph and the Two-Lane Invalidation Engine.
//!
//! `tenui-reactive` decouples cosmetic UI updates from geometric layout reflows,
//! ensuring that terminal interfaces achieve maximum frame throughput by avoiding
//! redundant computations.
//!
//! ## Core Architecture: Two-Lane Invalidation
//!
//! Every state change in Tenui is classified into one of two invalidation lanes via [`Invalidation`]:
//!
//! - **Paint Lane ([`Invalidation::PAINT`])**: Emitted when only cosmetic styling (colors, text content
//!   of identical grapheme width, borders) changes. The layout engine (Taffy) is bypassed completely,
//!   yielding **zero-reflow** repaints.
//! - **Layout Lane ([`Invalidation::LAYOUT`])**: Emitted when dimensions, flex properties, margins,
//!   or text width changes. Triggers geometric re-measurement and child repositioning.
//!
//! ## Primitives
//!
//! - **[`Signal<T>`]**: Fine-grained reactive value container with subscription tracking and
//!   automatic dirty flagging against an [`InvalidationSink`].
//! - **[`batch`] & [`ReactiveContext`]**: Transactional batching scope. Defers all downstream
//!   dirty signals and notification callbacks until the batch finishes, dispatching exactly
//!   one consolidated invalidation pass.
//! - **[`TwoLaneBus`]**: Central event reactor routing invalidation messages to registered nodes.
//! - **[`ReactiveTextNode`]**: Adaptive text container that automatically detects when new content
//!   preserves string grapheme column width (emitting `PAINT`) versus alters width (promoting to `LAYOUT`).
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_reactive::{batch, Invalidation, Signal};
//! use std::cell::Cell;
//! use std::rc::Rc;
//!
//! let count = Signal::new(0);
//! let observed = Rc::new(Cell::new(0));
//!
//! let obs_clone = observed.clone();
//! let count_clone = count.clone();
//! count.subscribe(move || {
//!     obs_clone.set(count_clone.get());
//! });
//!
//! // Multiple mutations within a transactional batch
//! batch(|| {
//!     count.set(1);
//!     count.set(5);
//!     count.set(42);
//! });
//!
//! assert_eq!(count.get(), 42);
//! assert_eq!(observed.get(), 42);
//! ```

pub mod bus;
pub mod invalidation;
pub mod signal;
pub mod text;
pub mod tree;

pub use bus::TwoLaneBus;
pub use invalidation::{Invalidation, InvalidationSink};
pub use signal::{Derived, ReactiveContext, Signal, batch};
pub use text::ReactiveTextNode;
pub use tree::ReactiveTree;
