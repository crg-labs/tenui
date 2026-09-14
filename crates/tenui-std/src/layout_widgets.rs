use tenui_core::{CanvasSubviewMut, Color, Modifier, Rect};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SplitOrientation {
    Horizontal,
    Vertical,
}

/// An interactive split pane with an adjustable divider.
pub struct SplitPane {
    pub orientation: SplitOrientation,
    pub split_ratio: f32, // 0.0 to 1.0
}

impl Default for SplitPane {
    fn default() -> Self {
        Self {
            orientation: SplitOrientation::Horizontal,
            split_ratio: 0.5,
        }
    }
}

impl SplitPane {
    pub fn new(orientation: SplitOrientation, split_ratio: f32) -> Self {
        Self {
            orientation,
            split_ratio: split_ratio.clamp(0.1, 0.9),
        }
    }

    pub fn render<F1, F2>(&self, subview: &mut CanvasSubviewMut<'_>, first_child: F1, second_child: F2)
    where
        F1: FnOnce(&mut CanvasSubviewMut<'_>),
        F2: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        let w = subview.width();
        let h = subview.height();

        match self.orientation {
            SplitOrientation::Horizontal => {
                let split_col = (w as f32 * self.split_ratio).round() as u16;
                let split_col = split_col.clamp(1, w.saturating_sub(2));

                // Draw vertical divider
                for y in 0..h {
                    subview.set_char(split_col, y, '│', Color::DarkGray, Color::Reset, Modifier::empty());
                }

                // Render left pane
                let left_rect = Rect::new(0, 0, split_col, h);
                let mut left_subview = subview.subview_mut(left_rect);
                first_child(&mut left_subview);

                // Render right pane
                let right_w = w.saturating_sub(split_col + 1);
                let right_rect = Rect::new(split_col + 1, 0, right_w, h);
                let mut right_subview = subview.subview_mut(right_rect);
                second_child(&mut right_subview);
            }
            SplitOrientation::Vertical => {
                let split_row = (h as f32 * self.split_ratio).round() as u16;
                let split_row = split_row.clamp(1, h.saturating_sub(2));

                // Draw horizontal divider
                for x in 0..w {
                    subview.set_char(x, split_row, '─', Color::DarkGray, Color::Reset, Modifier::empty());
                }

                // Render top pane
                let top_rect = Rect::new(0, 0, w, split_row);
                let mut top_subview = subview.subview_mut(top_rect);
                first_child(&mut top_subview);

                // Render bottom pane
                let bot_h = h.saturating_sub(split_row + 1);
                let bot_rect = Rect::new(0, split_row + 1, w, bot_h);
                let mut bot_subview = subview.subview_mut(bot_rect);
                second_child(&mut bot_subview);
            }
        }
    }
}

/// A scroll container with fractional sub-cell scrollbar thumbs.
pub struct ScrollArea {
    pub content_height: usize,
    pub scroll_offset: usize,
}

impl ScrollArea {
    pub fn new(content_height: usize, scroll_offset: usize) -> Self {
        Self {
            content_height,
            scroll_offset,
        }
    }

    /// Renders the fractional scrollbar on the rightmost column of the subview.
    pub fn render_scrollbar(&self, subview: &mut CanvasSubviewMut<'_>) {
        let h = subview.height();
        let w = subview.width();
        if h < 2 || w < 2 || self.content_height <= h as usize {
            return;
        }

        let bar_x = w - 1;
        let track_h = h as f32;
        let thumb_ratio = (track_h / self.content_height as f32).clamp(0.1, 1.0);
        let thumb_h = (track_h * thumb_ratio).max(1.0);

        let max_scroll = (self.content_height - h as usize) as f32;
        let scroll_fraction = (self.scroll_offset as f32 / max_scroll).clamp(0.0, 1.0);
        let thumb_start_y = (track_h - thumb_h) * scroll_fraction;

        // Draw track
        for y in 0..h {
            subview.set_char(bar_x, y, '░', Color::DarkGray, Color::Reset, Modifier::empty());
        }

        // Draw thumb with fractional block boundaries
        let start_int = thumb_start_y.floor() as u16;
        let end_int = (thumb_start_y + thumb_h).ceil() as u16;

        for y in start_int..end_int.min(h) {
            let ch = if y == start_int && thumb_start_y.fract() > 0.5 {
                '▄' // Bottom half for top transition
            } else if y + 1 == end_int && (thumb_start_y + thumb_h).fract() < 0.5 {
                '▀' // Top half for bottom transition
            } else {
                '█' // Full block
            };
            subview.set_char(bar_x, y, ch, Color::Reset, Color::Reset, Modifier::empty());
        }
    }
}

/// A multi-dock layout container with tabs.
pub struct DockContainer {
    pub tabs: Vec<String>,
    pub active_tab: usize,
}

impl DockContainer {
    pub fn new(tabs: Vec<String>) -> Self {
        Self { tabs, active_tab: 0 }
    }

    pub fn select_next(&mut self) {
        if !self.tabs.is_empty() {
            self.active_tab = (self.active_tab + 1) % self.tabs.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.tabs.is_empty() {
            if self.active_tab == 0 {
                self.active_tab = self.tabs.len() - 1;
            } else {
                self.active_tab -= 1;
            }
        }
    }

    pub fn render_tabs(&self, subview: &mut CanvasSubviewMut<'_>) {
        let mut x = 0;
        for (i, tab) in self.tabs.iter().enumerate() {
            let is_active = i == self.active_tab;
            let fg = if is_active { Color::Cyan } else { Color::DarkGray };
            let modifier = if is_active { Modifier::BOLD } else { Modifier::empty() };

            let label = format!(" [{}] ", tab);
            subview.set_string(x, 0, &label, fg, Color::Reset, modifier);
            x += label.chars().count() as u16;
            if x >= subview.width() {
                break;
            }
        }

        // Divider
        for cx in 0..subview.width() {
            subview.set_char(cx, 1, '─', Color::DarkGray, Color::Reset, Modifier::empty());
        }
    }
}
