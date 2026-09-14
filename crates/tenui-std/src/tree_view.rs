use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// A node in a hierarchical tree.
#[derive(Debug, Clone)]
pub struct TreeNode<T> {
    pub id: u64,
    pub title: String,
    pub data: T,
    pub children: Vec<TreeNode<T>>,
    pub expanded: bool,
}

impl<T> TreeNode<T> {
    pub fn new(id: u64, title: &str, data: T) -> Self {
        Self {
            id,
            title: title.to_string(),
            data,
            children: Vec::new(),
            expanded: false,
        }
    }

    pub fn with_children(mut self, children: Vec<TreeNode<T>>) -> Self {
        self.children = children;
        self
    }
}

struct FlatItem<'a, T> {
    node: &'a TreeNode<T>,
    depth: usize,
    is_last_child: bool,
}

/// A collapsible hierarchical tree view with depth guides and keyboard navigation.
pub struct TreeView<T> {
    pub roots: Vec<TreeNode<T>>,
    pub selected_id: Option<u64>,
    pub scroll_offset: usize,
}

impl<T> TreeView<T> {
    pub fn new(roots: Vec<TreeNode<T>>) -> Self {
        let first_id = roots.first().map(|r| r.id);
        Self {
            roots,
            selected_id: first_id,
            scroll_offset: 0,
        }
    }

    fn flatten<'a>(&'a self) -> Vec<FlatItem<'a, T>> {
        let mut result = Vec::new();
        let total = self.roots.len();
        for (i, root) in self.roots.iter().enumerate() {
            Self::flatten_node(root, 0, i == total - 1, &mut result);
        }
        result
    }

    fn flatten_node<'a>(node: &'a TreeNode<T>, depth: usize, is_last: bool, out: &mut Vec<FlatItem<'a, T>>) {
        out.push(FlatItem {
            node,
            depth,
            is_last_child: is_last,
        });

        if node.expanded {
            let child_count = node.children.len();
            for (idx, child) in node.children.iter().enumerate() {
                Self::flatten_node(child, depth + 1, idx == child_count - 1, out);
            }
        }
    }

    pub fn select_next(&mut self) {
        let items = self.flatten();
        if items.is_empty() {
            return;
        }

        let curr_idx = items
            .iter()
            .position(|it| Some(it.node.id) == self.selected_id)
            .unwrap_or(0);
        let next_idx = (curr_idx + 1).min(items.len() - 1);
        self.selected_id = Some(items[next_idx].node.id);
    }

    pub fn select_prev(&mut self) {
        let items = self.flatten();
        if items.is_empty() {
            return;
        }

        let curr_idx = items
            .iter()
            .position(|it| Some(it.node.id) == self.selected_id)
            .unwrap_or(0);
        let prev_idx = curr_idx.saturating_sub(1);
        self.selected_id = Some(items[prev_idx].node.id);
    }

    pub fn toggle_selected(&mut self) {
        if let Some(target_id) = self.selected_id {
            Self::toggle_recursive(&mut self.roots, target_id);
        }
    }

    fn toggle_recursive(nodes: &mut [TreeNode<T>], target_id: u64) -> bool {
        for n in nodes.iter_mut() {
            if n.id == target_id {
                n.expanded = !n.expanded;
                return true;
            }
            if Self::toggle_recursive(&mut n.children, target_id) {
                return true;
            }
        }
        false
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>) {
        let flat = self.flatten();
        let visible_height = subview.height() as usize;

        for (row, item) in flat.iter().skip(self.scroll_offset).take(visible_height).enumerate() {
            let y = row as u16;
            let is_selected = Some(item.node.id) == self.selected_id;

            let fg = if is_selected { Color::Yellow } else { Color::White };
            let bg = if is_selected { Color::DarkGray } else { Color::Reset };
            let modifier = if is_selected { Modifier::BOLD } else { Modifier::empty() };

            let mut line = String::new();
            // Depth indent guides
            for _ in 0..item.depth {
                line.push_str("│  ");
            }

            // Branch marker based on whether this is the last child
            if item.depth > 0 {
                if item.is_last_child {
                    line.push_str("└─ ");
                } else {
                    line.push_str("├─ ");
                }
            }

            // Expansion symbol
            if !item.node.children.is_empty() {
                if item.node.expanded {
                    line.push_str("▼ ");
                } else {
                    line.push_str("▶ ");
                }
            } else {
                line.push_str("• ");
            }

            line.push_str(&item.node.title);

            let truncated: String = line.chars().take(subview.width() as usize).collect();
            subview.set_string(0, y, &truncated, fg, bg, modifier);
        }
    }
}
