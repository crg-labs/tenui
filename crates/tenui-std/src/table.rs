use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Defines a column in a [`VirtualTable`].
pub struct Column<T> {
    /// The column header label displayed in the first row.
    pub title: &'static str,
    /// Width of the column in terminal character cells.
    pub width: u16,
    /// Closure extracting a display string from a row item.
    pub extractor: Box<dyn Fn(&T) -> String>,
}

impl<T> Column<T> {
    /// Creates a new table column with a static title, cell width, and row extractor closure.
    pub fn new<F>(title: &'static str, width: u16, extractor: F) -> Self
    where
        F: Fn(&T) -> String + 'static,
    {
        Self {
            title,
            width,
            extractor: Box::new(extractor),
        }
    }
}

/// Zero-allocation virtualized table rendering datasets of arbitrary scale.
/// Only instantiates and renders rows within the currently visible viewport.
pub struct VirtualTable<'a, T> {
    pub items: &'a [T],
    pub columns: Vec<Column<T>>,
    pub scroll_offset: usize,
    pub selected: Option<usize>,
}

impl<'a, T> VirtualTable<'a, T> {
    pub fn new(items: &'a [T], columns: Vec<Column<T>>) -> Self {
        Self {
            items,
            columns,
            scroll_offset: 0,
            selected: None,
        }
    }

    pub fn scroll_down(&mut self, delta: usize, max_visible: usize) {
        let max_scroll = self.items.len().saturating_sub(max_visible);
        self.scroll_offset = (self.scroll_offset + delta).min(max_scroll);
    }

    pub fn scroll_up(&mut self, delta: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(delta);
    }

    pub fn select_next(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let next = match self.selected {
            Some(curr) => (curr + 1).min(self.items.len() - 1),
            None => 0,
        };
        self.selected = Some(next);
    }

    pub fn select_prev(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let prev = match self.selected {
            Some(curr) => curr.saturating_sub(1),
            None => 0,
        };
        self.selected = Some(prev);
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>) {
        let height = subview.height();
        if height < 3 {
            return;
        }

        let mut x_offset = 0;
        // 1. Render Header
        for col in &self.columns {
            subview.set_string(x_offset, 0, col.title, Color::Cyan, Color::Reset, Modifier::BOLD);
            x_offset += col.width + 1;
            if x_offset < subview.width() {
                subview.set_char(x_offset - 1, 0, '│', Color::DarkGray, Color::Reset, Modifier::empty());
            }
        }

        // 2. Render Divider
        for x in 0..subview.width() {
            subview.set_char(x, 1, '─', Color::DarkGray, Color::Reset, Modifier::empty());
        }

        // 3. Render visible rows
        let available_rows = (height - 2) as usize;
        let visible_items = self
            .items
            .iter()
            .enumerate()
            .skip(self.scroll_offset)
            .take(available_rows);

        for (row_idx, (global_idx, item)) in visible_items.enumerate() {
            let y = 2 + row_idx as u16;
            let is_selected = self.selected == Some(global_idx);

            let bg = if is_selected { Color::DarkGray } else { Color::Reset };
            let modifier = if is_selected { Modifier::BOLD } else { Modifier::empty() };

            let mut col_x = 0;
            for col in &self.columns {
                let text = (col.extractor)(item);
                let truncated: String = text.chars().take(col.width as usize).collect();
                subview.set_string(col_x, y, &truncated, Color::White, bg, modifier);
                col_x += col.width + 1;
            }
        }
    }
}
