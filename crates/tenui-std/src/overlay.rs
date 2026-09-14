use tenui_core::{CanvasSubviewMut, Color, Modifier, Rect, TextOverflow};
use tenui_layout::{BorderStyle, draw_border, draw_border_with_header};

use crate::{input::TextInput, theme::readable_on};

/// A modal dialog establishing a Focus Scope Barrier.
pub struct Modal {
    pub title: String,
    pub right_badge: Option<String>,
    pub width: u16,
    pub height: u16,
    pub scope_id: u64,
    pub transparent: bool,
    pub auto_width: bool,
    pub auto_grow: bool,
    pub max_width_percent: f32,
    pub overflow: TextOverflow,
    pub min_width: u16,
    pub min_height: u16,
    pub bg: Color,
    pub border_color: Color,
    pub border_style: BorderStyle,
    pub client_clipping: bool,
}

impl Modal {
    pub fn new(title: &str, width: u16, height: u16, scope_id: u64) -> Self {
        Self {
            title: title.to_string(),
            right_badge: None,
            width,
            height,
            scope_id,
            transparent: false,
            auto_width: false,
            auto_grow: false,
            max_width_percent: 0.85,
            overflow: TextOverflow::Ellipsis,
            min_width: width,
            min_height: height,
            bg: Color::Reset,
            border_color: Color::Cyan,
            border_style: BorderStyle::ROUNDED,
            client_clipping: true,
        }
    }

    pub fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    pub fn with_border_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn with_border_style(mut self, style: BorderStyle) -> Self {
        self.border_style = style;
        self
    }

    pub fn with_client_clipping(mut self, clip: bool) -> Self {
        self.client_clipping = clip;
        self
    }

    pub fn with_transparent(mut self, transparent: bool) -> Self {
        self.transparent = transparent;
        self
    }

    pub fn with_right_badge(mut self, badge: &str) -> Self {
        self.right_badge = Some(badge.to_string());
        self
    }

    pub fn with_auto_width(mut self, max_percent: f32) -> Self {
        self.auto_width = true;
        self.max_width_percent = max_percent.clamp(0.1, 1.0);
        self
    }

    pub fn with_auto_grow(mut self, auto_grow: bool) -> Self {
        self.auto_grow = auto_grow;
        self
    }

    pub fn with_overflow(mut self, overflow: TextOverflow) -> Self {
        self.overflow = overflow;
        self
    }

    pub fn with_min_size(mut self, min_w: u16, min_h: u16) -> Self {
        self.min_width = min_w;
        self.min_height = min_h;
        self
    }

    /// Computes the centered bounding rect of the modal within the parent bounds,
    /// auto-growing to accommodate title, badges, and minimum dimensions when configured.
    pub fn bounds(&self, parent: Rect) -> Rect {
        let max_w = ((parent.width as f32) * self.max_width_percent).round() as u16;
        let mut target_w = self.width.max(self.min_width);
        if self.auto_grow || self.auto_width {
            let left_w = self.title.chars().count() as u16;
            let right_w = self.right_badge.as_ref().map(|b| b.chars().count() as u16).unwrap_or(0);
            let needed_w = left_w + right_w + 10;
            target_w = target_w.max(needed_w);
            if self.auto_width {
                target_w = target_w.max(max_w.min(parent.width));
            }
        }
        let w = target_w.min(parent.width);
        let h = self.height.max(self.min_height).min(parent.height);
        let x = parent.x + (parent.width.saturating_sub(w)) / 2;
        let y = parent.y + (parent.height.saturating_sub(h)) / 2;
        Rect::new(x, y, w, h)
    }

    /// Content rect inside the 1-cell border.
    pub fn content_rect(&self, modal_rect: Rect) -> Rect {
        Rect::new(
            modal_rect.x + 1,
            modal_rect.y + 1,
            modal_rect.width.saturating_sub(2),
            modal_rect.height.saturating_sub(2),
        )
    }

    /// Helper to render responsive key-value text within modal with wrap protection.
    pub fn render_key_value(
        subview: &mut CanvasSubviewMut<'_>,
        x: u16,
        y: u16,
        key: &str,
        val: &str,
        key_color: Color,
        val_color: Color,
    ) {
        if y >= subview.height() || x >= subview.width() {
            return;
        }
        let max_w = subview.width().saturating_sub(x);
        let key_len = CanvasSubviewMut::measure_text_width(key);
        let val_len = CanvasSubviewMut::measure_text_width(val);
        let total_needed = key_len + 1 + val_len;
        if total_needed <= max_w {
            subview.write_str_clipped(x, y, key, key_color, Color::TRANSPARENT);
            let val_x = x + key_len + 1;
            subview.write_str_clipped(val_x, y, val, val_color, Color::TRANSPARENT);
        } else {
            // Two-line hanging indent with clean ellipsis on value
            subview.write_str_clipped(x, y, key, key_color, Color::TRANSPARENT);
            if y + 1 < subview.height() {
                let val_avail = subview.width().saturating_sub(x + 2);
                subview.write_str_overflow(
                    x + 2,
                    y + 1,
                    val,
                    val_avail,
                    TextOverflow::Ellipsis,
                    val_color,
                    Color::TRANSPARENT,
                );
            }
        }
    }

    /// Renders the modal frame and delegates content rendering to the provided callback.
    /// Content is scissor-clipped to inner client bounds by default to prevent border destruction.
    pub fn render<F>(&self, subview: &mut CanvasSubviewMut<'_>, f: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        if !self.transparent {
            // Pre-fill entire modal bounds with solid background color so underlying content never bleeds through
            for y in 0..subview.height() {
                for x in 0..subview.width() {
                    subview.set_char(x, y, ' ', Color::Reset, self.bg, Modifier::empty());
                }
            }
        }

        let border_bg = if self.transparent { Color::TRANSPARENT } else { self.bg };
        draw_border_with_header(
            subview,
            self.border_style,
            &self.title,
            self.right_badge.as_deref(),
            self.overflow,
            self.border_color,
            border_bg,
        );

        let inner_rect = Rect::new(
            1,
            1,
            subview.width().saturating_sub(2),
            subview.height().saturating_sub(2),
        );

        if self.client_clipping && !inner_rect.is_empty() {
            let mut inner = subview.subview_mut(inner_rect);
            f(&mut inner);
        } else {
            f(subview);
        }
    }

    /// Explicitly renders the modal with full outer subview access (unclipped to borders).
    pub fn render_raw<F>(&self, subview: &mut CanvasSubviewMut<'_>, f: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        if !self.transparent {
            for y in 0..subview.height() {
                for x in 0..subview.width() {
                    subview.set_char(x, y, ' ', Color::Reset, self.bg, Modifier::empty());
                }
            }
        }

        let border_bg = if self.transparent { Color::TRANSPARENT } else { self.bg };
        draw_border_with_header(
            subview,
            self.border_style,
            &self.title,
            self.right_badge.as_deref(),
            self.overflow,
            self.border_color,
            border_bg,
        );

        f(subview);
    }

    /// Renders the modal with inner client subview clipping.
    pub fn render_content<F>(&self, subview: &mut CanvasSubviewMut<'_>, f: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        self.render(subview, f);
    }
}

/// A command item for the command palette.
#[derive(Debug, Clone)]
pub struct CommandItem {
    pub id: String,
    pub title: String,
    pub category: String,
}

/// Command palette overlay featuring interactive fuzzy search.
pub struct CommandPalette {
    pub query: TextInput,
    pub items: Vec<CommandItem>,
    pub selected: usize,
    pub bg: Color,
}

impl CommandPalette {
    pub fn new(items: Vec<CommandItem>) -> Self {
        Self {
            query: TextInput::new(),
            items,
            selected: 0,
            bg: Color::Rgb(30, 30, 46),
        }
    }

    pub fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    /// Returns items filtered and ranked by fuzzy query matching.
    pub fn filtered_items(&self) -> Vec<&CommandItem> {
        let q = self.query.content.to_lowercase();
        if q.is_empty() {
            return self.items.iter().collect();
        }

        let mut matched: Vec<(&CommandItem, usize)> = self
            .items
            .iter()
            .filter_map(|item| {
                let score = fuzzy_score(&q, &item.title.to_lowercase());
                score.map(|s| (item, s))
            })
            .collect();

        // Sort by highest score first
        matched.sort_by_key(|a| std::cmp::Reverse(a.1));
        matched.into_iter().map(|(item, _)| item).collect()
    }

    pub fn select_next(&mut self) {
        let count = self.filtered_items().len();
        if count > 0 {
            self.selected = (self.selected + 1).min(count - 1);
        }
    }

    pub fn select_prev(&mut self) {
        self.selected = self.selected.saturating_sub(1);
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>) {
        let w = subview.width();
        let h = subview.height();
        if w < 4 || h < 4 {
            return;
        }

        // 1. Clear modal surface to guarantee 100% opacity over underlying content
        subview.clear(self.bg);

        // 2. Draw border and header
        draw_border(subview, BorderStyle::PLAIN, Color::Yellow, self.bg);
        subview.set_string(2, 0, " Command Palette ", Color::Yellow, self.bg, Modifier::BOLD);

        // 3. Search input row at y = 1
        subview.set_string(1, 1, "> ", Color::Cyan, self.bg, Modifier::BOLD);
        let search_text = &self.query.content;
        // Contrast-safe against `self.bg`: white on a dark palette, black on a light one
        // (e.g. Catppuccin Latte) — never white-on-white.
        subview.set_string(3, 1, search_text, readable_on(self.bg), self.bg, Modifier::empty());

        // 4. Divider at y = 2
        for x in 1..w.saturating_sub(1) {
            subview.set_char(x, 2, '─', Color::DarkGray, self.bg, Modifier::empty());
        }

        // 5. Render matched commands
        // Available rows between divider (y=2) and bottom border (y=h-1) is h - 4
        let available_rows = (h.saturating_sub(4)) as usize;
        if available_rows == 0 {
            return;
        }

        let filtered = self.filtered_items();
        let scroll_offset = if self.selected >= available_rows {
            self.selected - available_rows + 1
        } else {
            0
        };

        let inner_width = (w.saturating_sub(2)) as usize;

        for (i, item) in filtered.iter().skip(scroll_offset).take(available_rows).enumerate() {
            let y = 3 + i as u16;
            let is_sel = (i + scroll_offset) == self.selected;
            let bg = if is_sel { Color::Rgb(69, 71, 90) } else { self.bg };
            let fg = if is_sel { Color::Yellow } else { readable_on(self.bg) };
            let modifier = if is_sel { Modifier::BOLD } else { Modifier::empty() };

            let display = format!("[{}] {}", item.category, item.title);
            let display_chars = display.chars().count();
            let line_str = if display_chars >= inner_width {
                display.chars().take(inner_width).collect()
            } else {
                let pad = inner_width - display_chars;
                format!("{}{}", display, " ".repeat(pad))
            };
            subview.set_string(1, y, &line_str, fg, bg, modifier);
        }
    }
}

fn fuzzy_score(pattern: &str, target: &str) -> Option<usize> {
    let mut pattern_chars = pattern.chars().peekable();
    let mut score = 0;
    let mut consecutive = 0;

    for ch in target.chars() {
        if let Some(&p) = pattern_chars.peek() {
            if ch == p {
                pattern_chars.next();
                score += 10 + (consecutive * 5);
                consecutive += 1;
            } else {
                consecutive = 0;
            }
        }
    }

    if pattern_chars.peek().is_none() {
        Some(score)
    } else {
        None
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;
    use crate::theme::{ThemePalette, contrast_ratio};

    #[test]
    fn command_palette_text_is_legible_on_a_light_theme() {
        // Regression: on Catppuccin Latte (light) the palette drew white-on-white for the
        // unselected rows and the search field. Every cell must contrast with its OWN bg.
        let latte = ThemePalette::catppuccin_latte();
        let cp = CommandPalette::new(vec![
            CommandItem {
                id: "open".into(),
                title: "Open File".into(),
                category: "File".into(),
            },
            CommandItem {
                id: "save".into(),
                title: "Save File".into(),
                category: "File".into(),
            },
        ])
        .with_bg(latte.surface);

        let mut buf = Buffer::new(30, 8);
        {
            let mut sv = buf.subview_mut(Rect::new(0, 0, 30, 8));
            cp.render(&mut sv);
        }
        // Row y=3 is selected (dark bar), y=4 is an unselected item on the light surface — the
        // case that used to be white-on-white. Both must be legible against their own cell bg.
        for y in [3u16, 4u16] {
            let cell = buf.get(2, y).expect("item cell");
            assert!(
                contrast_ratio(cell.fg, cell.bg) >= 3.0,
                "palette item text unreadable at y={y}: fg={:?} bg={:?}",
                cell.fg,
                cell.bg
            );
        }
        // The unselected row specifically must not be white-on-light.
        assert_eq!(buf.get(2, 4).unwrap().fg, Color::Black);
    }
}
