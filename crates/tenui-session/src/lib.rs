#![forbid(unsafe_code)]
//! # Tenui Session (`tenui-session`)
//!
//! Workspace layout serialization, RON persistence, and headless session restore.
//!
//! `tenui-session` allows complex multi-pane terminal layouts and workspaces to be
//! serialized into human-readable Rusty Object Notation (RON) and restored identically
//! across application restarts or remote daemon reconnections.
//!
//! ## Core Primitives
//!
//! - **[`WorkspaceSession`]**: Root session snapshot holding the layout tree, active tab index,
//!   focused pane ID, and terminal dimensions. Supports lossless serialization via [`WorkspaceSession::to_ron`]
//!   and deserialization via [`WorkspaceSession::from_ron`].
//! - **[`SerializedPaneNode`]**: Hierarchical representation of flexbox split panes, nested containers,
//!   component metadata, and dimensions.
//! - **[`DaemonSessionManager`]**: Headless session manager managing session lifecycle transitions
//!   ([`DaemonLifecycle`]) for background daemon processes and SSH reconnections.
//!
//! ## Runnable Example
//!
//! ```rust
//! use tenui_session::{SerializedDimension, SerializedFlexDirection, SerializedPaneNode, WorkspaceSession};
//! use std::collections::HashMap;
//!
//! let root = SerializedPaneNode {
//!     id: 1,
//!     title: "Editor".into(),
//!     flex_direction: SerializedFlexDirection::Row,
//!     flex_grow: 1.0,
//!     flex_shrink: 1.0,
//!     size_width: SerializedDimension::Auto,
//!     size_height: SerializedDimension::Auto,
//!     children: Vec::new(),
//!     component_type: "CodeView".into(),
//!     metadata: HashMap::new(),
//! };
//!
//! let session = WorkspaceSession::new(root, (80, 24));
//! let ron = session.to_ron();
//! assert!(ron.contains("Editor"));
//!
//! let restored = WorkspaceSession::from_ron(&ron).expect("valid RON session");
//! assert_eq!(restored.root_layout.title, "Editor");
//! ```

pub mod schema;

pub use schema::{
    DaemonLifecycle, DaemonSessionManager, SerializedDimension, SerializedFlexDirection, SerializedPaneNode,
    WorkspaceSession,
};
