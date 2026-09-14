use std::marker::PhantomData;

use taffy::prelude::*;
use tenui_core::{Buffer, CanvasSubviewMut, Color, Modifier, Rect, TextOverflow};

use crate::{
    border::{BorderStyle, draw_border, draw_border_with_title},
    style::StyleExt,
};

type PaintFn<'a> = Box<dyn FnOnce(&mut CanvasSubviewMut<'_>) + 'a>;

struct LayoutNode<'a> {
    id: NodeId,
    paint: Option<PaintFn<'a>>,
    children: Vec<usize>,
}

/// Immediate-mode UI layout tree builder with lifetime `'a` allowing borrowing
/// from the caller's frame stack.
pub struct Ui<'a> {
    tree: TaffyTree<()>,
    nodes: Vec<LayoutNode<'a>>,
    current_parent: Option<usize>,
    _marker: PhantomData<&'a ()>,
}

impl<'a> Default for Ui<'a> {
    fn default() -> Self {
        Self::new()
    }
}

impl<'a> Ui<'a> {
    pub fn new() -> Self {
        Self {
            tree: TaffyTree::new(),
            nodes: Vec::new(),
            current_parent: None,
            _marker: PhantomData,
        }
    }

    /// Appends a container node with children.
    pub fn container<F>(&mut self, style: Style, f: F)
    where
        F: FnOnce(&mut Ui<'a>),
    {
        let id = self.tree.new_leaf(style).expect("new taffy node");
        let node_idx = self.nodes.len();
        self.nodes.push(LayoutNode {
            id,
            paint: None,
            children: Vec::new(),
        });

        if let Some(parent_idx) = self.current_parent {
            self.nodes[parent_idx].children.push(node_idx);
        }

        let prev_parent = self.current_parent;
        self.current_parent = Some(node_idx);
        f(self);
        self.current_parent = prev_parent;
    }

    /// Appends a row container (flex_direction: Row).
    pub fn row<F>(&mut self, style: Style, f: F)
    where
        F: FnOnce(&mut Ui<'a>),
    {
        self.container(style.flex_row(), f);
    }

    /// Appends a column container (flex_direction: Column).
    pub fn column<F>(&mut self, style: Style, f: F)
    where
        F: FnOnce(&mut Ui<'a>),
    {
        self.container(style.flex_col(), f);
    }

    /// Appends a CSS Grid container (display: Grid).
    pub fn grid<F>(&mut self, style: Style, f: F)
    where
        F: FnOnce(&mut Ui<'a>),
    {
        self.container(style.display_grid(), f);
    }

    /// Appends a leaf node with custom painting logic.
    pub fn leaf<F>(&mut self, style: Style, paint: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>) + 'a,
    {
        let id = self.tree.new_leaf(style).expect("new taffy leaf");
        let node_idx = self.nodes.len();
        self.nodes.push(LayoutNode {
            id,
            paint: Some(Box::new(paint)),
            children: Vec::new(),
        });

        if let Some(parent_idx) = self.current_parent {
            self.nodes[parent_idx].children.push(node_idx);
        }
    }

    /// Appends a text label leaf node.
    pub fn text(&mut self, style: Style, content: &str, fg: Color, bg: Color, modifier: Modifier) {
        let text_owned = content.to_string();
        self.leaf(style, move |subview| {
            subview.set_string(0, 0, &text_owned, fg, bg, modifier);
        });
    }

    /// Appends a block with a styled border, optional title, and nested children.
    pub fn block<F>(&mut self, style: Style, title: Option<&str>, border: BorderStyle, fg: Color, bg: Color, f: F)
    where
        F: FnOnce(&mut Ui<'a>),
    {
        let title_owned = title.map(|s| s.to_string());
        // Block container with 1-cell border padding
        let block_style = style.padding_all(1.0);
        let id = self.tree.new_leaf(block_style).expect("new block node");
        let node_idx = self.nodes.len();

        let paint_box: PaintFn<'a> = Box::new(move |subview| {
            if let Some(t) = title_owned {
                draw_border_with_title(subview, border, &t, TextOverflow::Ellipsis, fg, bg);
            } else {
                draw_border(subview, border, fg, bg);
            }
        });

        self.nodes.push(LayoutNode {
            id,
            paint: Some(paint_box),
            children: Vec::new(),
        });

        if let Some(parent_idx) = self.current_parent {
            self.nodes[parent_idx].children.push(node_idx);
        }

        let prev_parent = self.current_parent;
        self.current_parent = Some(node_idx);
        f(self);
        self.current_parent = prev_parent;
    }

    /// Resolves Taffy layout over the tree and renders each node into the target buffer.
    pub fn render_to_buffer(mut self, buffer: &mut Buffer) {
        if self.nodes.is_empty() {
            return;
        }

        // Connect taffy parent-child relationships
        for i in 0..self.nodes.len() {
            let child_ids: Vec<NodeId> = self.nodes[i].children.iter().map(|&idx| self.nodes[idx].id).collect();
            if !child_ids.is_empty() {
                let parent_id = self.nodes[i].id;
                let _ = self.tree.set_children(parent_id, &child_ids);
            }
        }

        let root_id = self.nodes[0].id;

        // Set root style to 100% width and height
        let mut root_style = self.tree.style(root_id).cloned().unwrap_or_default();
        root_style.size.width = Dimension::length(buffer.width as f32);
        root_style.size.height = Dimension::length(buffer.height as f32);
        let _ = self.tree.set_style(root_id, root_style);

        let available = Size {
            width: AvailableSpace::Definite(buffer.width as f32),
            height: AvailableSpace::Definite(buffer.height as f32),
        };

        if self.tree.compute_layout(root_id, available).is_err() {
            return;
        }

        // Recursively dispatch paint callbacks
        self.paint_node(0, 0, 0, buffer);
    }

    fn paint_node(&mut self, node_idx: usize, parent_x: u16, parent_y: u16, buffer: &mut Buffer) {
        let node_id = self.nodes[node_idx].id;
        let (node_x, node_y, node_w, node_h) = match self.tree.layout(node_id) {
            Ok(layout) => {
                let gx = parent_x.saturating_add(layout.location.x.max(0.0).round() as u16);
                let gy = parent_y.saturating_add(layout.location.y.max(0.0).round() as u16);
                let gw = layout.size.width.max(0.0).round() as u16;
                let gh = layout.size.height.max(0.0).round() as u16;
                (gx, gy, gw, gh)
            }
            Err(_) => return,
        };

        let node_rect = Rect::new(node_x, node_y, node_w, node_h);

        // Execute paint callback if present
        if let Some(paint) = self.nodes[node_idx].paint.take() {
            let mut subview = buffer.subview_mut(node_rect);
            paint(&mut subview);
        }

        let children = self.nodes[node_idx].children.clone();
        for child_idx in children {
            self.paint_node(child_idx, node_x, node_y, buffer);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::style::StyleExt;

    fn cell_char(buf: &Buffer, x: u16, y: u16) -> &str {
        buf.get(x, y).map(|c| c.symbol.as_str()).unwrap_or("")
    }

    #[test]
    fn empty_ui_renders_nothing() {
        let mut buf = Buffer::new(20, 10);
        let ui = Ui::new();
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), " ");
    }

    #[test]
    fn single_leaf_paints_at_origin() {
        let mut buf = Buffer::new(40, 10);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.leaf(Style::default().width_cells(10.0).height_cells(3.0), |sv| {
                sv.set_string(0, 0, "ABC", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "A");
        assert_eq!(cell_char(&buf, 1, 0), "B");
        assert_eq!(cell_char(&buf, 2, 0), "C");
    }

    #[test]
    fn row_lays_out_horizontally() {
        let mut buf = Buffer::new(40, 10);
        let mut ui = Ui::new();
        ui.row(Style::default(), |ui| {
            ui.leaf(Style::default().width_cells(10.0).height_cells(5.0), |sv| {
                sv.set_string(0, 0, "L", Color::Reset, Color::Reset, Modifier::empty());
            });
            ui.leaf(Style::default().width_cells(10.0).height_cells(5.0), |sv| {
                sv.set_string(0, 0, "R", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "L");
        assert_eq!(cell_char(&buf, 10, 0), "R");
    }

    #[test]
    fn column_lays_out_vertically() {
        let mut buf = Buffer::new(40, 10);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.leaf(Style::default().width_cells(10.0).height_cells(3.0), |sv| {
                sv.set_string(0, 0, "T", Color::Reset, Color::Reset, Modifier::empty());
            });
            ui.leaf(Style::default().width_cells(10.0).height_cells(3.0), |sv| {
                sv.set_string(0, 0, "B", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "T");
        assert_eq!(cell_char(&buf, 0, 3), "B");
    }

    #[test]
    fn nested_column_with_rows() {
        let mut buf = Buffer::new(40, 10);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.row(Style::default().height_cells(3.0), |ui| {
                ui.leaf(Style::default().width_cells(20.0).height_cells(3.0), |sv| {
                    sv.set_string(0, 0, "A", Color::Reset, Color::Reset, Modifier::empty());
                });
                ui.leaf(Style::default().width_cells(20.0).height_cells(3.0), |sv| {
                    sv.set_string(0, 0, "B", Color::Reset, Color::Reset, Modifier::empty());
                });
            });
            ui.leaf(Style::default().width_cells(40.0).height_cells(3.0), |sv| {
                sv.set_string(0, 0, "C", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "A");
        assert_eq!(cell_char(&buf, 20, 0), "B");
        assert_eq!(cell_char(&buf, 0, 3), "C");
    }

    #[test]
    fn text_leaf_writes_string() {
        let mut buf = Buffer::new(30, 5);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.text(
                Style::default().width_cells(20.0).height_cells(1.0),
                "Hello",
                Color::Cyan,
                Color::Reset,
                Modifier::BOLD,
            );
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "H");
        assert_eq!(cell_char(&buf, 4, 0), "o");
        let cell = buf.get(0, 0).unwrap();
        assert_eq!(cell.fg, Color::Cyan);
        assert!(cell.modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn grid_two_columns() {
        let mut buf = Buffer::new(20, 10);
        let mut ui = Ui::new();
        ui.grid(Style::default().grid_columns(2), |ui| {
            ui.leaf(Style::default().height_cells(3.0), |sv| {
                sv.set_string(0, 0, "G1", Color::Reset, Color::Reset, Modifier::empty());
            });
            ui.leaf(Style::default().height_cells(3.0), |sv| {
                sv.set_string(0, 0, "G2", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "G");
        let g2_x = 10;
        assert_eq!(cell_char(&buf, g2_x, 0), "G");
    }

    #[test]
    fn leaf_with_percent_sizing() {
        let mut buf = Buffer::new(40, 10);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.leaf(Style::default().width_percent(0.5).height_cells(2.0), |sv| {
                sv.set_string(0, 0, "X", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "X");
        assert_eq!(cell_char(&buf, 20, 0), " ", "50% width means no paint at midpoint");
    }

    #[test]
    fn block_draws_border_corners() {
        let mut buf = Buffer::new(20, 6);
        let mut ui = Ui::new();
        ui.column(Style::default(), |ui| {
            ui.block(
                Style::default().width_cells(20.0).height_cells(6.0),
                None,
                BorderStyle::PLAIN,
                Color::Reset,
                Color::Reset,
                |_ui| {},
            );
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "┌");
        assert_eq!(cell_char(&buf, 19, 0), "┐");
        assert_eq!(cell_char(&buf, 0, 5), "└");
        assert_eq!(cell_char(&buf, 19, 5), "┘");
        assert_eq!(cell_char(&buf, 1, 0), "─");
        assert_eq!(cell_char(&buf, 0, 1), "│");
    }

    #[test]
    fn flex_grow_shares_space() {
        let mut buf = Buffer::new(30, 5);
        let mut ui = Ui::new();
        ui.row(Style::default(), |ui| {
            ui.leaf(Style::default().grow(1.0).height_cells(5.0), |sv| {
                sv.set_string(0, 0, "A", Color::Reset, Color::Reset, Modifier::empty());
            });
            ui.leaf(Style::default().grow(1.0).height_cells(5.0), |sv| {
                sv.set_string(0, 0, "B", Color::Reset, Color::Reset, Modifier::empty());
            });
        });
        ui.render_to_buffer(&mut buf);
        assert_eq!(cell_char(&buf, 0, 0), "A");
        assert_eq!(cell_char(&buf, 15, 0), "B");
    }
}
