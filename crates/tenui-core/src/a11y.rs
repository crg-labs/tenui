//! Accessibility: the internal Semantic Accessibility Tree, live-region announcer, and
//! screen-reader bridge.
//!
//! Immediate-mode TUIs are "stateless at the visual layer": the engine keeps no memory of
//! which interactive nodes exist, their spatial relationships, or tab order. Tenui closes this gap with
//! a retained [`SemanticTree`] (roles, accessible names/values, bounds, focus & tab order)
//! that the runtime maintains alongside the visual tree, plus an [`Announcer`] live-region
//! protocol and a [`ScreenReaderBridge`] that renders both to a textual transcript a screen
//! reader (or an automated test) can consume.

use std::{
    collections::VecDeque,
    io::{self, Write},
    sync::atomic::{AtomicBool, Ordering},
};

use crate::buffer::Rect;

/// The accessible role of a node (a pragmatic subset of the ARIA roles that map to TUIs).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Role {
    /// Non-interactive grouping container.
    Group,
    /// Static text / label.
    Label,
    Button,
    Checkbox,
    Radio,
    TextInput,
    List,
    ListItem,
    Tab,
    Dialog,
    /// A live region whose updates should be announced.
    Status,
    Other,
}

impl Role {
    /// The word a screen reader speaks for this role.
    pub fn spoken(&self) -> &'static str {
        match self {
            Role::Group => "group",
            Role::Label => "label",
            Role::Button => "button",
            Role::Checkbox => "checkbox",
            Role::Radio => "radio button",
            Role::TextInput => "text field",
            Role::List => "list",
            Role::ListItem => "list item",
            Role::Tab => "tab",
            Role::Dialog => "dialog",
            Role::Status => "status",
            Role::Other => "element",
        }
    }
}

/// A node in the semantic accessibility tree.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticNode {
    pub id: u64,
    pub role: Role,
    /// Accessible name (e.g. a button's text, a field's label).
    pub label: String,
    /// Accessible value (e.g. a text field's contents, "checked"/"unchecked").
    pub value: Option<String>,
    pub bounds: Rect,
    pub focusable: bool,
    pub focused: bool,
    pub children: Vec<SemanticNode>,
}

impl SemanticNode {
    pub fn new(id: u64, role: Role, label: impl Into<String>) -> Self {
        Self {
            id,
            role,
            label: label.into(),
            value: None,
            bounds: Rect::ZERO,
            focusable: matches!(
                role,
                Role::Button | Role::Checkbox | Role::Radio | Role::TextInput | Role::Tab | Role::ListItem
            ),
            focused: false,
            children: Vec::new(),
        }
    }

    pub fn with_value(mut self, value: impl Into<String>) -> Self {
        self.value = Some(value.into());
        self
    }

    pub fn with_bounds(mut self, bounds: Rect) -> Self {
        self.bounds = bounds;
        self
    }

    pub fn child(mut self, node: SemanticNode) -> Self {
        self.children.push(node);
        self
    }
}

/// A retained accessibility tree.
#[derive(Debug, Clone, PartialEq)]
pub struct SemanticTree {
    pub root: SemanticNode,
}

impl SemanticTree {
    pub fn new(root: SemanticNode) -> Self {
        Self { root }
    }

    /// Pre-order list of focusable node ids — the tab order.
    pub fn focus_order(&self) -> Vec<u64> {
        let mut out = Vec::new();
        Self::collect_focusable(&self.root, &mut out);
        out
    }

    fn collect_focusable(node: &SemanticNode, out: &mut Vec<u64>) {
        if node.focusable {
            out.push(node.id);
        }
        for c in &node.children {
            Self::collect_focusable(c, out);
        }
    }

    /// The id of the next focusable node after `current` in tab order (wraps around).
    /// With no `current`, returns the first focusable node.
    pub fn next_focus(&self, current: Option<u64>) -> Option<u64> {
        let order = self.focus_order();
        if order.is_empty() {
            return None;
        }
        match current.and_then(|c| order.iter().position(|&id| id == c)) {
            Some(pos) => Some(order[(pos + 1) % order.len()]),
            None => Some(order[0]),
        }
    }

    /// The id of the previous focusable node (wraps around).
    pub fn prev_focus(&self, current: Option<u64>) -> Option<u64> {
        let order = self.focus_order();
        if order.is_empty() {
            return None;
        }
        match current.and_then(|c| order.iter().position(|&id| id == c)) {
            Some(pos) => Some(order[(pos + order.len() - 1) % order.len()]),
            None => Some(order[order.len() - 1]),
        }
    }

    /// The currently focused node, if any.
    pub fn focused(&self) -> Option<&SemanticNode> {
        Self::find_focused(&self.root)
    }

    fn find_focused(node: &SemanticNode) -> Option<&SemanticNode> {
        if node.focused {
            return Some(node);
        }
        node.children.iter().find_map(Self::find_focused)
    }
}

/// How urgently a live-region update should be announced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Politeness {
    /// Announce when the reader is idle (does not interrupt).
    Polite,
    /// Announce immediately, interrupting current speech.
    Assertive,
}

/// A queued live-region announcement.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Announcement {
    pub politeness: Politeness,
    pub text: String,
}

/// A live-region announcer: the app pushes updates, the bridge drains and speaks them.
#[derive(Debug, Default)]
pub struct Announcer {
    queue: VecDeque<Announcement>,
}

impl Announcer {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn announce(&mut self, politeness: Politeness, text: impl Into<String>) {
        self.queue.push_back(Announcement {
            politeness,
            text: text.into(),
        });
    }

    /// Convenience for a polite announcement.
    pub fn polite(&mut self, text: impl Into<String>) {
        self.announce(Politeness::Polite, text);
    }

    /// Convenience for an assertive (interrupting) announcement.
    pub fn assertive(&mut self, text: impl Into<String>) {
        self.announce(Politeness::Assertive, text);
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }

    /// Removes and returns all pending announcements (assertive first, then FIFO).
    pub fn drain(&mut self) -> Vec<Announcement> {
        let mut items: Vec<Announcement> = self.queue.drain(..).collect();
        items.sort_by_key(|a| match a.politeness {
            Politeness::Assertive => 0,
            Politeness::Polite => 1,
        });
        items
    }
}

/// Renders semantic information to a textual form a screen reader (or a test) consumes.
pub struct ScreenReaderBridge;

impl ScreenReaderBridge {
    /// A single spoken line for a node, e.g. `button "Save"` or `text field "Name": Alice`
    /// (with `, focused` appended when focused).
    pub fn describe_node(node: &SemanticNode) -> String {
        let mut s = format!("{} \"{}\"", node.role.spoken(), node.label);
        if let Some(v) = &node.value {
            s.push_str(&format!(": {}", v));
        }
        if node.focused {
            s.push_str(", focused");
        }
        s
    }

    /// An indented outline of the whole tree (one node per line).
    pub fn describe_tree(tree: &SemanticTree) -> String {
        let mut out = String::new();
        Self::describe_into(&tree.root, 0, &mut out);
        out
    }

    fn describe_into(node: &SemanticNode, depth: usize, out: &mut String) {
        for _ in 0..depth {
            out.push_str("  ");
        }
        out.push_str(&Self::describe_node(node));
        out.push('\n');
        for c in &node.children {
            Self::describe_into(c, depth + 1, out);
        }
    }

    /// Emits the current focus line followed by any pending announcements, draining them.
    pub fn emit<W: Write>(writer: &mut W, tree: &SemanticTree, announcer: &mut Announcer) -> io::Result<()> {
        if let Some(focused) = tree.focused() {
            writeln!(writer, "[focus] {}", Self::describe_node(focused))?;
        }
        for a in announcer.drain() {
            let tag = match a.politeness {
                Politeness::Assertive => "!",
                Politeness::Polite => "·",
            };
            writeln!(writer, "[{}] {}", tag, a.text)?;
        }
        writer.flush()
    }
}

// Global accessibility preferences honored across subsystems

static REDUCE_MOTION: AtomicBool = AtomicBool::new(false);
static HIGH_CONTRAST: AtomicBool = AtomicBool::new(false);

/// Enables/disables the global "reduce motion" preference. Animation subsystems should
/// snap to final state (or shorten durations) when this is on.
pub fn set_reduce_motion(on: bool) {
    REDUCE_MOTION.store(on, Ordering::Relaxed);
}

/// Whether "reduce motion" is currently requested.
pub fn reduce_motion() -> bool {
    REDUCE_MOTION.load(Ordering::Relaxed)
}

/// Enables/disables the global "high contrast" preference. Theme/VFX should honor it.
pub fn set_high_contrast(on: bool) {
    HIGH_CONTRAST.store(on, Ordering::Relaxed);
}

/// Whether "high contrast" is currently requested.
pub fn high_contrast() -> bool {
    HIGH_CONTRAST.load(Ordering::Relaxed)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_tree() -> SemanticTree {
        let root = SemanticNode::new(0, Role::Dialog, "Settings")
            .child(SemanticNode::new(1, Role::Label, "Name"))
            .child(SemanticNode::new(2, Role::TextInput, "Name").with_value("Alice"))
            .child({
                let mut ok = SemanticNode::new(3, Role::Button, "OK");
                ok.focused = true;
                ok
            })
            .child(SemanticNode::new(4, Role::Button, "Cancel"));
        SemanticTree::new(root)
    }

    #[test]
    fn test_focus_order_is_focusables_in_preorder() {
        let t = sample_tree();
        // Label is not focusable; TextInput + both Buttons are.
        assert_eq!(t.focus_order(), vec![2, 3, 4]);
    }

    #[test]
    fn test_tab_navigation_wraps() {
        let t = sample_tree();
        assert_eq!(t.next_focus(Some(2)), Some(3));
        assert_eq!(t.next_focus(Some(4)), Some(2)); // wrap
        assert_eq!(t.prev_focus(Some(2)), Some(4)); // wrap back
        assert_eq!(t.next_focus(None), Some(2)); // first
    }

    #[test]
    fn test_describe_node_and_focused() {
        let t = sample_tree();
        assert_eq!(t.focused().map(|n| n.id), Some(3));
        assert_eq!(
            ScreenReaderBridge::describe_node(t.focused().unwrap()),
            "button \"OK\", focused"
        );
        let field = SemanticNode::new(2, Role::TextInput, "Name").with_value("Alice");
        assert_eq!(ScreenReaderBridge::describe_node(&field), "text field \"Name\": Alice");
    }

    #[test]
    fn test_announcer_orders_assertive_first_and_drains() {
        let mut a = Announcer::new();
        a.polite("saved");
        a.assertive("error!");
        let drained = a.drain();
        assert_eq!(drained[0].text, "error!"); // assertive first
        assert_eq!(drained[1].text, "saved");
        assert!(a.is_empty());
    }

    #[test]
    fn test_bridge_emit_transcript() {
        let t = sample_tree();
        let mut announcer = Announcer::new();
        announcer.polite("field updated");
        let mut out = Vec::new();
        ScreenReaderBridge::emit(&mut out, &t, &mut announcer).unwrap();
        let s = String::from_utf8(out).unwrap();
        assert!(s.contains("[focus] button \"OK\", focused"));
        assert!(s.contains("field updated"));
        assert!(announcer.is_empty());
    }

    #[test]
    fn test_global_prefs() {
        set_reduce_motion(true);
        assert!(reduce_motion());
        set_reduce_motion(false);
        assert!(!reduce_motion());
    }
}
