// tenui/tests/test_advanced_subsystems.rs
//! Deterministic Integration & Verification Harness for Tenui Advanced Subsystems Specification.

use std::{collections::HashMap, time::Duration};

use tenui_core::{
    Buffer, Color, Modifier, NodeId,
    geometry::{Point, Rect},
};
use tenui_dnd::internal::{DockManager, DockPlacement};
use tenui_session::schema::{
    DaemonSessionManager, SerializedDimension, SerializedFlexDirection, SerializedPaneNode, WorkspaceSession,
};
use tenui_sound::synth::{SoundCue, TactileAudioEngine};
use tenui_text::multi_cursor::{BranchingUndoTree, Cursor, CursorSet, TextEditDelta};
use tenui_virt::list::{FenwickTree, VirtualList};
use tenui_wgpu::pipeline::{GpuPipelineConfig, TERMINAL_WGSL_SHADER, WgpuCanvasRunner};

// Section 7 Exact Tests from Specification

#[test]
fn test_fenwick_tree_prefix_and_binary_search() {
    // 5 items with dynamic heights: [3, 1, 8, 4, 2]
    let heights = [3, 1, 8, 4, 2];
    let mut ft = FenwickTree::new(5, 0);
    for (i, &h) in heights.iter().enumerate() {
        ft.add(i + 1, h);
    }

    assert_eq!(ft.prefix_sum(1), 3);
    assert_eq!(ft.prefix_sum(3), 12); // 3 + 1 + 8 = 12
    assert_eq!(ft.prefix_sum(5), 18); // Total vertical height

    // Seek offset y = 10 -> falls inside item 3 (prefix_sum(2)=4, prefix_sum(3)=12)
    let idx = ft.find_index_at_offset(10);
    assert_eq!(idx, 2);
}

#[test]
fn test_multi_cursor_overlapping_coalesce() {
    let mut cursors = CursorSet::new(5);
    // Add overlapping cursors
    cursors.add_cursor(Cursor { anchor: 4, head: 8 });
    cursors.add_cursor(Cursor { anchor: 7, head: 12 });

    // Invariant: Overlapping selections coalesce into single span: [4, 12]
    let list: Vec<_> = cursors.iter().copied().collect();
    assert_eq!(list.len(), 1);
    assert_eq!(list[0].start(), 4);
    assert_eq!(list[0].end(), 12);
}

#[test]
fn test_dock_edge_snap_thresholds() {
    let container = Rect {
        x: 10,
        y: 5,
        width: 40,
        height: 20,
    };

    // Point near left border (x = 11, y = 15)
    let pt_left = Point { x: 11, y: 15 };
    assert_eq!(
        DockManager::resolve_placement(pt_left, container),
        DockPlacement::SplitLeft
    );

    // Point in deep center (x = 30, y = 15)
    let pt_center = Point { x: 30, y: 15 };
    assert_eq!(
        DockManager::resolve_placement(pt_center, container),
        DockPlacement::CenterDeck
    );
}

// Extended Invariant Tests for Subsystems 1 to 6

#[test]
fn test_dock_manager_ghost_hoisting_and_preview_stipple() {
    let mut dm = DockManager::new();
    let source = NodeId::new(10);
    let target = NodeId::new(20);
    let container = Rect {
        x: 0,
        y: 0,
        width: 40,
        height: 20,
    };

    dm.start_drag(source, 100, Point { x: 5, y: 5 }, (12, 6));
    assert!(dm.is_dragging());

    // Update pointer to top edge of target container -> SplitTop
    dm.update_pointer(Point { x: 20, y: 1 }, &[(target, container)]);
    assert!(dm.hover_candidate.is_some());
    let (_, _, placement) = dm.hover_candidate.unwrap();
    assert_eq!(placement, DockPlacement::SplitTop);

    // Render preview overlay onto a buffer slice
    let mut buffer = Buffer::new(40, 20);
    let mut subview = buffer.subview_mut(container);
    dm.render_preview_overlay(&mut subview);

    // Check that cells in the SplitTop region (y: 0..10, x: 0..40) received stipple and cyan blend
    let top_cell = buffer.get(5, 2).unwrap();
    assert_eq!(top_cell.symbol.as_str(), "░");
    assert_eq!(top_cell.fg, Color::CYAN);

    // Bottom region should remain untouched blank
    let bottom_cell = buffer.get(5, 15).unwrap();
    assert_eq!(bottom_cell.symbol.as_str(), " ");
}

#[test]
fn test_virtual_list_dynamic_heights_and_anchoring() {
    let items: Vec<usize> = (0..50).collect();
    let mut list = VirtualList::new(items, 4); // 50 items * 4 rows = 200 rows total

    assert_eq!(list.total_height(), 200);

    // Scroll to row 40 (starts at item 10)
    list.scroll_offset = 40.0;
    let (start_idx, end_idx, sub_offset) = list.resolve_visible_range(16);
    assert_eq!(start_idx, 10);
    assert_eq!(sub_offset, 0.0);
    assert_eq!(end_idx, 15); // Covers half-open range [10..15] spanning 40..56

    // Asynchronous event: item 2 resizes from 4 rows to 10 rows (delta +6)
    // Item 2 is above the active viewport anchor (item 10)
    list.set_item_height(2, 10);
    // Continuous Scroll Anchoring Invariant: scroll_offset absorbs delta so visual content does not jump!
    assert_eq!(list.scroll_offset, 46.0);

    // Resolving again produces identical visible items in the viewport
    let (start_after, _, _) = list.resolve_visible_range(16);
    assert_eq!(start_after, 10);
}

#[test]
fn test_multi_cursor_column_block_and_undo_branching() {
    let lines = ["export PORT=3000", "export HOST=localhost", "export USER=admin"];
    // Select column block "export " (indices 0..7 across rows 0..2)
    let block = CursorSet::from_rectangular_block(&lines, 0, 0, 7, 2);
    assert_eq!(block.len(), 3);

    let mut undo_tree = BranchingUndoTree::new(vec![Cursor::point(0)]);

    // Commit edit 1
    undo_tree.commit_edit(
        vec![TextEditDelta {
            range: (0, 0),
            replaced_text: "".into(),
            inserted_text: "let a = 1;".into(),
        }],
        vec![Cursor::point(10)],
    );

    // Commit edit 2
    undo_tree.commit_edit(
        vec![TextEditDelta {
            range: (10, 10),
            replaced_text: "".into(),
            inserted_text: "let b = 2;".into(),
        }],
        vec![Cursor::point(20)],
    );

    // Undo edit 2
    let (deltas, cursors) = undo_tree.undo().unwrap();
    assert_eq!(deltas[0].inserted_text, "let b = 2;");
    assert_eq!(cursors[0].head, 10);

    // Fork new edit branch from edit 1
    undo_tree.commit_edit(
        vec![TextEditDelta {
            range: (10, 10),
            replaced_text: "".into(),
            inserted_text: "let c = 3;".into(),
        }],
        vec![Cursor::point(20)],
    );

    assert_eq!(undo_tree.current_node, 3);
    // Undo to root edit 1
    undo_tree.undo();
    assert_eq!(undo_tree.current_node, 1);
    assert_eq!(undo_tree.current_branch_count(), 2); // Branch 0 (b=2) and Branch 1 (c=3)
}

#[test]
fn test_session_serialization_and_daemon_lifecycle() {
    let mut meta = HashMap::new();
    meta.insert("editor".into(), "tenui-code".into());

    let root = SerializedPaneNode {
        id: 1,
        title: "Main".into(),
        flex_direction: SerializedFlexDirection::Column,
        flex_grow: 1.0,
        flex_shrink: 1.0,
        size_width: SerializedDimension::Percent(100.0),
        size_height: SerializedDimension::Percent(100.0),
        children: vec![SerializedPaneNode {
            id: 2,
            title: "Side".into(),
            flex_direction: SerializedFlexDirection::Row,
            flex_grow: 0.0,
            flex_shrink: 0.0,
            size_width: SerializedDimension::Length(25.0),
            size_height: SerializedDimension::Auto,
            children: Vec::new(),
            component_type: "Sidebar".into(),
            metadata: HashMap::new(),
        }],
        component_type: "Root".into(),
        metadata: meta,
    };

    let session = WorkspaceSession::new(root, (100, 30));
    let ron = session.to_ron();
    assert!(ron.contains("WorkspaceSession("));
    assert!(ron.contains("\"tenui-code\""));
    assert!(ron.contains("terminal_dimensions: (100, 30)"));

    let mut daemon = DaemonSessionManager::new(session);
    daemon.handle_client_disconnect();
    assert_eq!(daemon.lifecycle, tenui_session::DaemonLifecycle::FrozenOnDisconnect);

    daemon.handle_client_reattach((140, 45));
    assert_eq!(daemon.lifecycle, tenui_session::DaemonLifecycle::Reattached);
    assert_eq!(daemon.session.terminal_dimensions, (140, 45));
    assert!(daemon.front_buffer_dirty);
}

#[test]
fn test_tactile_audio_synthesizer_and_model() {
    let mut engine = TactileAudioEngine::new().with_dec_audio(true);
    let mut out = Vec::new();

    // Verify DECPS frequency escapes for mechanical switch clicks
    engine.play_cue(SoundCue::KeycapDepress, &mut out).unwrap();
    assert_eq!(out, b"\x1b[10;50;850;8t"); // 850 Hz, 8ms

    out.clear();
    engine.play_cue(SoundCue::KeycapRelease, &mut out).unwrap();
    assert_eq!(out, b"\x1b[10;30;1150;5t"); // 1150 Hz spring return

    // Paced bell fallback test
    let mut bell_engine = TactileAudioEngine::new()
        .with_dec_audio(false)
        .with_bell_throttle(Duration::from_millis(50));

    out.clear();
    bell_engine.play_cue(SoundCue::ValidationFailure, &mut out).unwrap();
    assert_eq!(out, b"\x07");

    // Second bell within throttle duration is silenced
    out.clear();
    bell_engine.play_cue(SoundCue::ValidationFailure, &mut out).unwrap();
    assert!(out.is_empty());
}

#[test]
fn test_wgpu_canvas_buffer_translation_and_wgsl() {
    let mut buffer = Buffer::new(20, 10);
    buffer.set_string(
        0,
        0,
        "GPU Canvas",
        Color::Rgb(255, 128, 0),
        Color::Black,
        Modifier::BOLD,
    );

    let mut runner = WgpuCanvasRunner::new(GpuPipelineConfig {
        grid_cols: 20,
        grid_rows: 10,
        enable_crt_scanlines: true,
        cell_pixel_size: (12, 24),
    });

    runner.update_from_buffer(&buffer);
    assert_eq!(runner.instance_buffer.len(), 200);

    let cell0 = runner.instance_buffer[0];
    assert_eq!(cell0.grid_pos, [0.0, 0.0]);
    assert_eq!(cell0.fg_color[0], 1.0);
    assert!((cell0.fg_color[1] - 128.0 / 255.0).abs() < 0.01);
    assert_eq!(cell0.fg_color[2], 0.0);
    assert!(cell0.attributes & Modifier::BOLD.bits() as u32 != 0);

    // WGSL Shader validation
    assert!(TERMINAL_WGSL_SHADER.contains("@vertex"));
    assert!(TERMINAL_WGSL_SHADER.contains("@fragment"));
    assert!(TERMINAL_WGSL_SHADER.contains("cell_size_ndc"));
}
