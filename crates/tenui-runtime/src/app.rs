//! The retained reactive `App` reactor.

use std::{
    collections::HashMap,
    io::{self, Stdout, Write},
    rc::Rc,
};

use taffy::{TaffyTree, prelude::*};
use tenui_anim::ActiveTicker;
use tenui_core::{
    Announcer, Buffer, CanvasSubviewMut, Color, Politeness, Rect, Role, SemanticNode, SemanticTree, Terminal,
};
use tenui_reactive::{Invalidation, InvalidationSink, TwoLaneBus};

/// Application-level node handle (distinct from `taffy::NodeId`), and the key used for
/// invalidation on the [`TwoLaneBus`].
pub type NodeId = u64;

/// What a single [`App::render_frame`] did — useful for tests and telemetry.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FrameReport {
    /// Whether a Taffy layout pass ran this frame (false ⇒ paint-only, zero reflow).
    pub relayout: bool,
    /// Number of node rectangles repainted this frame.
    pub painted: usize,
}

type PaintFn = Box<dyn Fn(&mut CanvasSubviewMut<'_>)>;

/// Accessibility metadata attached to a node (drives the [`SemanticTree`]).
struct SemanticInfo {
    role: Role,
    label: String,
    value: Option<String>,
    focusable: bool,
}

struct NodeRecord {
    taffy: taffy::NodeId,
    bounds: Rect,
    paint: Option<PaintFn>,
    semantic: Option<SemanticInfo>,
}

/// The retained render loop: persistent Taffy tree + Two-Lane invalidation + demand ticker.
pub struct App<W: Write = Stdout> {
    terminal: Terminal<W>,
    taffy: TaffyTree<()>,
    nodes: Vec<NodeRecord>,
    taffy_to_id: HashMap<taffy::NodeId, usize>,
    bus: Rc<TwoLaneBus>,
    ticker: Rc<ActiveTicker>,
    bg: Color,
    needs_full: bool,
    last_size: (u16, u16),
    reflow_passes: u64,
    paint_ops: u64,
    focused: Option<NodeId>,
    announcer: Announcer,
}

impl<W: Write> App<W> {
    /// Builds an app over `terminal` with a root container filling the screen
    /// (flex column by default). Add children with [`add_container`](Self::add_container)
    /// and [`add_leaf`](Self::add_leaf).
    pub fn new(terminal: Terminal<W>) -> Self {
        let mut taffy = TaffyTree::new();
        let root_style = Style {
            display: Display::Flex,
            flex_direction: FlexDirection::Column,
            size: Size {
                width: percent(1.0),
                height: percent(1.0),
            },
            ..Default::default()
        };
        let root = taffy.new_leaf(root_style).expect("taffy root leaf");
        let mut taffy_to_id = HashMap::new();
        taffy_to_id.insert(root, 0usize);
        let last_size = terminal.size();
        Self {
            terminal,
            taffy,
            nodes: vec![NodeRecord {
                taffy: root,
                bounds: Rect::ZERO,
                paint: None,
                semantic: None,
            }],
            taffy_to_id,
            bus: Rc::new(TwoLaneBus::new()),
            ticker: Rc::new(ActiveTicker::new()),
            bg: Color::Reset,
            needs_full: true,
            last_size,
            reflow_passes: 0,
            paint_ops: 0,
            focused: None,
            announcer: Announcer::new(),
        }
    }

    /// The root node (always id 0).
    pub fn root(&self) -> NodeId {
        0
    }

    /// The shared invalidation bus. Bind reactive `Signal`s to it (with a node id and
    /// [`Invalidation`] flags) so their mutations schedule the right lane.
    pub fn bus(&self) -> Rc<TwoLaneBus> {
        Rc::clone(&self.bus)
    }

    /// The demand-driven animation scheduler.
    pub fn ticker(&self) -> Rc<ActiveTicker> {
        Rc::clone(&self.ticker)
    }

    /// The default background used when clearing on a full repaint.
    pub fn set_background(&mut self, bg: Color) {
        self.bg = bg;
        self.needs_full = true;
    }

    /// Adds a container (no paint of its own) under `parent`, returning its node id.
    pub fn add_container(&mut self, parent: NodeId, style: Style) -> NodeId {
        self.add_node(parent, style, None)
    }

    /// Adds a painted leaf under `parent`. `paint` receives a subview clipped to the
    /// node's computed bounds and typically reads reactive `Signal`s.
    pub fn add_leaf<F>(&mut self, parent: NodeId, style: Style, paint: F) -> NodeId
    where
        F: Fn(&mut CanvasSubviewMut<'_>) + 'static,
    {
        self.add_node(parent, style, Some(Box::new(paint)))
    }

    fn add_node(&mut self, parent: NodeId, style: Style, paint: Option<PaintFn>) -> NodeId {
        let taffy_id = self.taffy.new_leaf(style).expect("taffy new_leaf");
        let parent_taffy = self.nodes[parent as usize].taffy;
        self.taffy.add_child(parent_taffy, taffy_id).expect("taffy add_child");
        let id = self.nodes.len();
        self.nodes.push(NodeRecord {
            taffy: taffy_id,
            bounds: Rect::ZERO,
            paint,
            semantic: None,
        });
        self.taffy_to_id.insert(taffy_id, id);
        self.needs_full = true; // structure changed → next frame is a full layout+paint
        id as NodeId
    }

    /// Marks a node paint-dirty (cosmetic change; no reflow).
    pub fn mark_paint(&self, id: NodeId) {
        self.bus.mark_dirty(id, Invalidation::PAINT);
    }

    /// Marks a node layout-dirty (geometry change; triggers subtree reflow + repaint).
    pub fn mark_layout(&self, id: NodeId) {
        self.bus.mark_dirty(id, Invalidation::LAYOUT);
    }

    /// Forces the next frame to fully re-layout and repaint.
    pub fn request_full_repaint(&mut self) {
        self.needs_full = true;
    }

    /// The last computed bounds of a node.
    pub fn node_bounds(&self, id: NodeId) -> Option<Rect> {
        self.nodes.get(id as usize).map(|n| n.bounds)
    }

    /// Total Taffy layout passes run so far (0 delta across a paint-only frame).
    pub fn reflow_passes(&self) -> u64 {
        self.reflow_passes
    }

    /// Total node repaints so far.
    pub fn paint_ops(&self) -> u64 {
        self.paint_ops
    }

    /// Access the underlying terminal (e.g. to emit escapes or read size).
    pub fn terminal_mut(&mut self) -> &mut Terminal<W> {
        &mut self.terminal
    }

    // Accessibility (Semantic tree + focus + announcements)

    /// Attaches an accessible role and name to a node so it appears in the
    /// [`semantic_tree`](Self::semantic_tree). Focusability defaults from the role.
    pub fn set_semantic(&mut self, id: NodeId, role: Role, label: impl Into<String>) {
        if let Some(rec) = self.nodes.get_mut(id as usize) {
            let focusable = matches!(
                role,
                Role::Button | Role::Checkbox | Role::Radio | Role::TextInput | Role::Tab | Role::ListItem
            );
            rec.semantic = Some(SemanticInfo {
                role,
                label: label.into(),
                value: None,
                focusable,
            });
        }
    }

    /// Sets a node's accessible value (e.g. a text field's contents, "checked").
    pub fn set_accessible_value(&mut self, id: NodeId, value: impl Into<String>) {
        if let Some(Some(info)) = self.nodes.get_mut(id as usize).map(|r| r.semantic.as_mut()) {
            info.value = Some(value.into());
        }
    }

    /// The current semantic accessibility tree, mirroring the visual tree with live bounds
    /// and focus state.
    pub fn semantic_tree(&self) -> SemanticTree {
        SemanticTree::new(self.build_semantic(0))
    }

    fn build_semantic(&self, idx: usize) -> SemanticNode {
        let rec = &self.nodes[idx];
        let mut node = match &rec.semantic {
            Some(s) => {
                let mut n = SemanticNode::new(idx as u64, s.role, s.label.clone());
                n.value = s.value.clone();
                n.focusable = s.focusable;
                n
            }
            None => {
                let mut n = SemanticNode::new(idx as u64, Role::Group, String::new());
                n.focusable = false;
                n
            }
        };
        node.bounds = rec.bounds;
        node.focused = self.focused == Some(idx as u64);
        let children = self.taffy.children(rec.taffy).unwrap_or_default();
        for ct in children {
            if let Some(&cidx) = self.taffy_to_id.get(&ct) {
                node.children.push(self.build_semantic(cidx));
            }
        }
        node
    }

    /// The focused node id, if any.
    pub fn focused(&self) -> Option<NodeId> {
        self.focused
    }

    /// Moves focus to `id`, repaints the old and new focus rings, and announces the newly
    /// focused node to the [`Announcer`].
    pub fn set_focus(&mut self, id: NodeId) {
        if self.focused == Some(id) {
            return;
        }
        if let Some(old) = self.focused {
            self.bus.mark_dirty(old, Invalidation::PAINT);
        }
        self.focused = Some(id);
        self.bus.mark_dirty(id, Invalidation::PAINT);
        // Announce the new focus.
        let node = self.build_semantic(id as usize);
        let text = tenui_core::ScreenReaderBridge::describe_node(&node);
        self.announcer.announce(Politeness::Polite, text);
    }

    /// Advances focus to the next focusable node in tab order (wraps).
    pub fn focus_next(&mut self) {
        if let Some(next) = self.semantic_tree().next_focus(self.focused) {
            self.set_focus(next);
        }
    }

    /// Moves focus to the previous focusable node in tab order (wraps).
    pub fn focus_prev(&mut self) {
        if let Some(prev) = self.semantic_tree().prev_focus(self.focused) {
            self.set_focus(prev);
        }
    }

    /// Mutable access to the live-region announcer (push status updates here).
    pub fn announcer_mut(&mut self) -> &mut Announcer {
        &mut self.announcer
    }

    /// Renders one frame, honoring the Two-Lane invariant: Taffy runs only when something
    /// is layout-dirty (or on a forced full frame); otherwise only paint-dirty node rects
    /// are repainted. Returns what the frame did.
    pub fn render_frame(&mut self) -> io::Result<FrameReport> {
        // Resize ⇒ full relayout+repaint.
        let size = self.terminal.size();
        if size != self.last_size {
            self.last_size = size;
            self.needs_full = true;
        }

        let layout_dirty = self.bus.drain_layout_dirty();
        let paint_dirty = self.bus.drain_paint_dirty();

        let do_layout = self.needs_full || !layout_dirty.is_empty();
        if do_layout {
            // Mark just the layout-dirty nodes so Taffy's incremental cache reflows only
            // their subtrees; a full frame recomputes from the root.
            for &nid in &layout_dirty {
                if let Some(rec) = self.nodes.get(nid as usize) {
                    let _ = self.taffy.mark_dirty(rec.taffy);
                }
            }
            let avail = Size {
                width: AvailableSpace::Definite(size.0 as f32),
                height: AvailableSpace::Definite(size.1 as f32),
            };
            let root = self.nodes[0].taffy;
            let _ = self.taffy.compute_layout(root, avail);
            self.recompute_bounds(0, 0, 0);
            self.reflow_passes += 1;
        }

        // Repaint set: a full/relayout frame repaints everything (bounds may have moved);
        // a paint-only frame repaints just the paint-dirty nodes.
        let full = self.needs_full || do_layout;
        let repaint: Vec<usize> = if full {
            (0..self.nodes.len()).collect()
        } else {
            paint_dirty
                .iter()
                .map(|&x| x as usize)
                .filter(|&i| i < self.nodes.len())
                .collect()
        };

        if repaint.is_empty() {
            return Ok(FrameReport {
                relayout: do_layout,
                painted: 0,
            });
        }

        let bg = self.bg;
        let painted = repaint.len();
        // Disjoint field borrows: terminal (mut) + nodes (shared) into the present closure.
        let terminal = &mut self.terminal;
        let nodes = &self.nodes;
        let paint = |back: &mut Buffer| {
            if full {
                back.clear_with_bg(bg);
            }
            for &i in &repaint {
                let rec = &nodes[i];
                if let (Some(paint), false) = (&rec.paint, rec.bounds.is_empty()) {
                    // On a partial repaint, clear this node's rect first so stale cells
                    // from the previous frame don't bleed through when the widget paints
                    // fewer cells than its bounds.
                    if !full {
                        let mut clear = back.subview_mut(rec.bounds);
                        clear.clear(bg);
                    }
                    let mut sub = back.subview_mut(rec.bounds);
                    paint(&mut sub);
                }
            }
        };
        // With the `simd` feature, the retained repaint diff uses the SIMD differential
        // compositor — but only on large grids: its per-cell lane conversion only pays off
        // once the vectorized scan dominates (4K/8K terminals). On
        // ordinary sizes the scalar coalescer (no conversion, no lane buffers) is faster,
        // so we gate on cell count. Profiling-driven; measured on this repo's showcase.
        #[cfg(feature = "simd")]
        {
            const SIMD_MIN_CELLS: usize = 60_000;
            if (size.0 as usize) * (size.1 as usize) >= SIMD_MIN_CELLS {
                terminal.present_indexed(tenui_simd::SimdDiffScanner::diff_buffers, paint)?;
            } else {
                terminal.present(paint)?;
            }
        }
        #[cfg(not(feature = "simd"))]
        terminal.present(paint)?;

        self.paint_ops += painted as u64;
        self.needs_full = false;
        Ok(FrameReport {
            relayout: do_layout,
            painted,
        })
    }

    /// Walks the tree from `idx`, converting Taffy's parent-relative layout into absolute
    /// cell rectangles.
    fn recompute_bounds(&mut self, idx: usize, ox: i32, oy: i32) {
        let taffy_id = self.nodes[idx].taffy;
        let (ax, ay, w, h) = match self.taffy.layout(taffy_id) {
            Ok(l) => {
                let ax = ox + l.location.x.round() as i32;
                let ay = oy + l.location.y.round() as i32;
                (ax, ay, l.size.width.round() as i32, l.size.height.round() as i32)
            }
            Err(_) => (ox, oy, 0, 0),
        };
        self.nodes[idx].bounds = Rect::new(ax.max(0) as u16, ay.max(0) as u16, w.max(0) as u16, h.max(0) as u16);
        let children = self.taffy.children(taffy_id).unwrap_or_default();
        for ct in children {
            if let Some(&cidx) = self.taffy_to_id.get(&ct) {
                self.recompute_bounds(cidx, ax, ay);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_reactive::Signal;

    use super::*;

    fn headless_app(w: u16, h: u16) -> App<Vec<u8>> {
        let term = Terminal::with_writer(Vec::new(), w, h);
        App::new(term)
    }

    fn full_height_leaf() -> Style {
        Style {
            size: Size {
                width: percent(1.0),
                height: length(3.0),
            },
            ..Default::default()
        }
    }

    #[test]
    fn test_first_frame_lays_out_and_paints_all() {
        let mut app = headless_app(40, 12);
        let _a = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "A", Color::White, Color::Reset);
        });
        let _b = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "B", Color::White, Color::Reset);
        });

        let r = app.render_frame().unwrap();
        assert!(r.relayout, "first frame must lay out");
        assert_eq!(app.reflow_passes(), 1);
        // root + 2 leaves painted
        assert_eq!(r.painted, 3);
        // leaf bounds stacked vertically
        let ba = app.node_bounds(1).unwrap();
        let bb = app.node_bounds(2).unwrap();
        assert_eq!(ba.y, 0);
        assert_eq!(bb.y, 3);
    }

    /// The core Two-Lane invariant: a paint-only mutation runs ZERO Taffy passes and
    /// repaints exactly one node.
    #[test]
    fn test_paint_only_mutation_skips_layout() {
        let mut app = headless_app(40, 12);
        let label = Signal::new(0i32);
        let node = {
            let label = Rc::clone(&label);
            app.add_leaf(app.root(), full_height_leaf(), move |sv| {
                sv.write_str_clipped(0, 0, &format!("n={}", label.get()), Color::White, Color::Reset);
            })
        };
        // Bind the signal to the bus: mutations are PAINT-only for this node.
        label.bind_invalidation(node, Invalidation::PAINT, app.bus());

        app.render_frame().unwrap(); // initial full frame
        let reflows_after_first = app.reflow_passes();

        // Cosmetic change of equal width.
        label.set(7);
        let r = app.render_frame().unwrap();

        assert!(!r.relayout, "paint-only mutation must not relayout");
        assert_eq!(app.reflow_passes(), reflows_after_first, "zero extra Taffy passes");
        assert_eq!(r.painted, 1, "only the mutated node repaints");
    }

    #[test]
    fn test_layout_mutation_triggers_reflow() {
        let mut app = headless_app(40, 12);
        let node = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "x", Color::White, Color::Reset);
        });
        app.render_frame().unwrap();
        let before = app.reflow_passes();

        app.mark_layout(node);
        let r = app.render_frame().unwrap();
        assert!(r.relayout);
        assert_eq!(app.reflow_passes(), before + 1);
    }

    #[test]
    fn test_semantic_tree_focus_and_announce() {
        let mut app = headless_app(40, 12);
        let save = app.add_leaf(app.root(), full_height_leaf(), |_| {});
        let cancel = app.add_leaf(app.root(), full_height_leaf(), |_| {});
        app.set_semantic(save, Role::Button, "Save");
        app.set_semantic(cancel, Role::Button, "Cancel");
        app.render_frame().unwrap(); // computes bounds

        let tree = app.semantic_tree();
        assert_eq!(tree.focus_order(), vec![save, cancel]);
        assert!(app.node_bounds(save).unwrap().height > 0); // bounds mirrored in

        app.focus_next(); // -> Save
        assert_eq!(app.focused(), Some(save));
        let announced = app.announcer_mut().drain();
        assert!(announced.iter().any(|a| a.text.contains("Save")));

        app.focus_next(); // -> Cancel
        assert_eq!(app.focused(), Some(cancel));
        app.focus_next(); // wraps -> Save
        assert_eq!(app.focused(), Some(save));

        // A focus change schedules a paint-only frame (focus ring), no reflow.
        let before = app.reflow_passes();
        let r = app.render_frame().unwrap();
        assert!(!r.relayout);
        assert_eq!(app.reflow_passes(), before);
    }

    #[test]
    fn test_clean_frame_is_noop() {
        let mut app = headless_app(30, 8);
        app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "hi", Color::White, Color::Reset);
        });
        app.render_frame().unwrap(); // full
        let r = app.render_frame().unwrap(); // nothing dirty
        assert!(!r.relayout);
        assert_eq!(r.painted, 0);
    }

    #[test]
    fn test_request_full_repaint_forces_relayout() {
        let mut app = headless_app(40, 12);
        app.add_leaf(app.root(), full_height_leaf(), |_| {});
        app.render_frame().unwrap();
        let before = app.reflow_passes();

        app.request_full_repaint();
        let r = app.render_frame().unwrap();
        assert!(r.relayout);
        assert_eq!(app.reflow_passes(), before + 1);
    }

    #[test]
    fn test_adding_node_after_first_frame_triggers_full() {
        let mut app = headless_app(40, 12);
        app.add_leaf(app.root(), full_height_leaf(), |_| {});
        app.render_frame().unwrap();

        app.add_leaf(app.root(), full_height_leaf(), |_| {});
        let r = app.render_frame().unwrap();
        assert!(r.relayout, "structural change must trigger relayout");
        assert_eq!(r.painted, 3); // root + 2 leaves
    }

    #[test]
    fn test_mark_paint_multiple_nodes() {
        let mut app = headless_app(40, 12);
        let a = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "A", Color::Reset, Color::Reset);
        });
        let _b = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "B", Color::Reset, Color::Reset);
        });
        let c = app.add_leaf(app.root(), full_height_leaf(), |sv| {
            sv.write_str_clipped(0, 0, "C", Color::Reset, Color::Reset);
        });
        app.render_frame().unwrap();

        app.mark_paint(a);
        app.mark_paint(c);
        let r = app.render_frame().unwrap();
        assert!(!r.relayout);
        assert_eq!(r.painted, 2);
    }

    #[test]
    fn test_bus_is_shared_rc() {
        let app = headless_app(40, 12);
        let bus1 = app.bus();
        let bus2 = app.bus();
        assert!(Rc::ptr_eq(&bus1, &bus2));
    }

    #[test]
    fn test_batch_signal_single_frame() {
        use tenui_reactive::batch;

        let mut app = headless_app(40, 12);
        let s1 = Signal::new(0i32);
        let s2 = Signal::new(0i32);
        let n1 = {
            let s = Rc::clone(&s1);
            app.add_leaf(app.root(), full_height_leaf(), move |sv| {
                sv.write_str_clipped(0, 0, &format!("{}", s.get()), Color::Reset, Color::Reset);
            })
        };
        let n2 = {
            let s = Rc::clone(&s2);
            app.add_leaf(app.root(), full_height_leaf(), move |sv| {
                sv.write_str_clipped(0, 0, &format!("{}", s.get()), Color::Reset, Color::Reset);
            })
        };
        s1.bind_invalidation(n1, Invalidation::PAINT, app.bus());
        s2.bind_invalidation(n2, Invalidation::PAINT, app.bus());

        app.render_frame().unwrap();

        batch(|| {
            s1.set(10);
            s2.set(20);
        });

        let r = app.render_frame().unwrap();
        assert!(!r.relayout);
        assert_eq!(r.painted, 2);
    }

    #[test]
    fn test_set_background_forces_full_repaint() {
        let mut app = headless_app(40, 12);
        app.add_leaf(app.root(), full_height_leaf(), |_| {});
        app.render_frame().unwrap();
        let before = app.reflow_passes();

        app.set_background(Color::Rgb(30, 30, 30));
        let r = app.render_frame().unwrap();
        assert!(r.relayout);
        assert_eq!(app.reflow_passes(), before + 1);
    }

    #[test]
    fn test_node_bounds_invalid_id() {
        let app = headless_app(40, 12);
        assert!(app.node_bounds(999).is_none());
    }
}
