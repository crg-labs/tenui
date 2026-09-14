use std::collections::HashMap;

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub enum SerializedDimension {
    Auto,
    Length(f32),
    Percent(f32),
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum SerializedFlexDirection {
    Row,
    Column,
    RowReverse,
    ColumnReverse,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct SerializedPaneNode {
    pub id: u64,
    pub title: String,
    pub flex_direction: SerializedFlexDirection,
    pub flex_grow: f32,
    pub flex_shrink: f32,
    pub size_width: SerializedDimension,
    pub size_height: SerializedDimension,
    pub children: Vec<SerializedPaneNode>,
    pub component_type: String,
    pub metadata: HashMap<String, String>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WorkspaceSession {
    pub version: String,
    pub root_layout: SerializedPaneNode,
    pub active_focus_id: u64,
    pub active_tab_index: usize,
    pub terminal_dimensions: (u16, u16),
}

impl WorkspaceSession {
    pub fn new(root_layout: SerializedPaneNode, dimensions: (u16, u16)) -> Self {
        let focus_id = root_layout.id;
        Self {
            version: "1.0.0".to_string(),
            root_layout,
            active_focus_id: focus_id,
            active_tab_index: 0,
            terminal_dimensions: dimensions,
        }
    }

    /// Serializes the session AST into human-readable Rusty Object Notation (RON).
    ///
    /// Uses serde so it round-trips exactly with [`from_ron`] (full float precision,
    /// nested children, metadata) rather than a hand-rolled emitter.
    ///
    /// [`from_ron`]: Self::from_ron
    pub fn to_ron(&self) -> String {
        let config = ron::ser::PrettyConfig::default().struct_names(true);
        ron::ser::to_string_pretty(self, config).unwrap_or_default()
    }

    /// Deserializes a session AST previously produced by [`to_ron`].
    ///
    /// [`to_ron`]: Self::to_ron
    pub fn from_ron(s: &str) -> Result<Self, ron::error::SpannedError> {
        ron::from_str(s)
    }
}

/// Headless Engine Daemon Lifecycle and State Handoff
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DaemonLifecycle {
    Running,
    FrozenOnDisconnect,
    Reattached,
}

pub struct DaemonSessionManager {
    pub session: WorkspaceSession,
    pub lifecycle: DaemonLifecycle,
    pub front_buffer_dirty: bool,
}

impl DaemonSessionManager {
    pub fn new(session: WorkspaceSession) -> Self {
        Self {
            session,
            lifecycle: DaemonLifecycle::Running,
            front_buffer_dirty: false,
        }
    }

    /// Intercept client disconnect (SIGHUP), detach stdout, and freeze the engine loop.
    pub fn handle_client_disconnect(&mut self) {
        self.lifecycle = DaemonLifecycle::FrozenOnDisconnect;
    }

    /// Handle new terminal attaching to the daemon socket with new dimensions.
    pub fn handle_client_reattach(&mut self, dimensions: (u16, u16)) {
        self.session.terminal_dimensions = dimensions;
        self.front_buffer_dirty = true;
        self.lifecycle = DaemonLifecycle::Reattached;
    }

    pub fn resume_rendering(&mut self) {
        self.lifecycle = DaemonLifecycle::Running;
        self.front_buffer_dirty = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_session_serialization_ron() {
        let mut metadata = HashMap::new();
        metadata.insert("cwd".into(), "/home/demon".into());

        let root = SerializedPaneNode {
            id: 1,
            title: "Editor".into(),
            flex_direction: SerializedFlexDirection::Row,
            flex_grow: 2.75, // non-round on purpose: proves precision survives the round trip
            flex_shrink: 1.0,
            size_width: SerializedDimension::Percent(100.0),
            size_height: SerializedDimension::Percent(100.0),
            children: vec![SerializedPaneNode {
                id: 2,
                title: "Tree".into(),
                flex_direction: SerializedFlexDirection::Column,
                flex_grow: 0.0,
                flex_shrink: 0.0,
                size_width: SerializedDimension::Length(30.0),
                size_height: SerializedDimension::Auto,
                children: Vec::new(),
                component_type: "FileTree".into(),
                metadata: HashMap::new(),
            }],
            component_type: "WorkspaceSplit".into(),
            metadata,
        };

        let session = WorkspaceSession::new(root, (120, 40));
        let ron = session.to_ron();
        assert!(ron.contains("WorkspaceSession"));

        // Restore is now two-way and lossless.
        let restored = WorkspaceSession::from_ron(&ron).expect("session RON round-trips");
        assert_eq!(restored, session);
        assert_eq!(restored.root_layout.flex_grow, 2.75);
    }

    #[test]
    fn test_daemon_lifecycle_handoff() {
        let root = SerializedPaneNode {
            id: 1,
            title: "Root".into(),
            flex_direction: SerializedFlexDirection::Row,
            flex_grow: 1.0,
            flex_shrink: 1.0,
            size_width: SerializedDimension::Auto,
            size_height: SerializedDimension::Auto,
            children: Vec::new(),
            component_type: "Terminal".into(),
            metadata: HashMap::new(),
        };
        let session = WorkspaceSession::new(root, (80, 24));
        let mut daemon = DaemonSessionManager::new(session);

        assert_eq!(daemon.lifecycle, DaemonLifecycle::Running);

        // Disconnect (SIGHUP)
        daemon.handle_client_disconnect();
        assert_eq!(daemon.lifecycle, DaemonLifecycle::FrozenOnDisconnect);

        // Reattach with resized window
        daemon.handle_client_reattach((100, 30));
        assert_eq!(daemon.lifecycle, DaemonLifecycle::Reattached);
        assert!(daemon.front_buffer_dirty);
        assert_eq!(daemon.session.terminal_dimensions, (100, 30));

        daemon.resume_rendering();
        assert_eq!(daemon.lifecycle, DaemonLifecycle::Running);
        assert!(!daemon.front_buffer_dirty);
    }
}
