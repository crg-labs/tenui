#![forbid(unsafe_code)]
//! # Tenui Text (`tenui-text`)
//!
//! Multi-cursor text editing engine, branching undo/redo trees, clipboard operations, and substring search.
//!
//! `tenui-text` serves as the foundational text manipulation substrate for Tenui text widgets
//! (including `TextInput`, `TextArea`, and `TextEditor`).
//!
//! ## Core Architecture
//!
//! - **[`TextBuffer`]**: The central grapheme-aware editing core combining text storage, cursor/selection
//!   management, and branching undo/redo history.
//! - **[`Cursor`] & [`CursorSet`]**: Non-overlapping multi-cursor topologies supporting simultaneous
//!   parallel edits across multiple lines and columns.
//! - **[`BranchingUndoTree`]**: Non-linear tree-structured undo/redo history. Unlike linear undo stacks
//!   that discard future edits when branching, the tree retains all modification branches.
//! - **[`clipboard_ops`]**: Standard clipboard operations ([`copy_selection`], [`cut_selection`], [`paste`])
//!   interacting with terminal clipboard protocols (OSC 52 and system clipboards).
//! - **[`search`]**: Exact and regular expression substring search across buffer contents.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_text::TextBuffer;
//!
//! let mut buffer = TextBuffer::from_text("Hello world");
//!
//! // Insert text and perform an edit
//! buffer.insert(" beautiful");
//! assert_eq!(buffer.text(), " beautifulHello world");
//!
//! // Undo the edit
//! buffer.undo();
//! assert_eq!(buffer.text(), "Hello world");
//!
//! // Redo the edit along branch 0
//! buffer.redo(0);
//! assert_eq!(buffer.text(), " beautifulHello world");
//! ```

pub mod buffer;
pub mod clipboard_ops;
pub mod multi_cursor;
pub mod search;

pub use buffer::TextBuffer;
pub use clipboard_ops::{apply_delta, copy_selection, cut_selection, paste};
pub use multi_cursor::{BranchingUndoTree, Cursor, CursorSet, TextEditDelta, UndoTreeNode};
pub use search::{SearchState, find_all};
