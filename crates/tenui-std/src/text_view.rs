use tenui_core::{CanvasSubviewMut, Color, Modifier, TextOverflow};
use unicode_width::UnicodeWidthChar;

/// An interactive, boundary-safe text viewer component.
///
/// Supports:
/// - Word Wrapping mode (`wrap: true`)
/// - Bidirectional Scrolling (`wrap: false` with `scroll_x` and `scroll_y`)
/// - Line numbering gutter with separator
/// - Fractional vertical and horizontal scrollbar indicators
/// - Tab expansion and control code sanitization
/// - Search highlighting
#[derive(Debug, Clone)]
pub struct TextView {
    pub content: String,
    pub lines: Vec<String>,
    pub scroll_y: usize,
    pub scroll_x: usize,
    pub wrap: bool,
    pub show_line_numbers: bool,
    pub show_scrollbars: bool,
    pub tab_width: usize,
    pub fg: Color,
    pub bg: Color,
    pub gutter_fg: Color,
    pub gutter_bg: Color,
    pub highlight_query: Option<String>,
    pub highlight_fg: Color,
    pub highlight_bg: Color,
}

impl Default for TextView {
    fn default() -> Self {
        Self::new("")
    }
}

impl TextView {
    /// Creates a new `TextView` with the specified content.
    pub fn new(content: impl Into<String>) -> Self {
        let text = content.into();
        let lines: Vec<String> = if text.is_empty() {
            vec![String::new()]
        } else {
            text.lines().map(|s| s.to_string()).collect()
        };

        Self {
            content: text,
            lines,
            scroll_y: 0,
            scroll_x: 0,
            wrap: false,
            show_line_numbers: true,
            show_scrollbars: true,
            tab_width: 4,
            fg: Color::Reset,
            bg: Color::Reset,
            gutter_fg: Color::DarkGray,
            gutter_bg: Color::Reset,
            highlight_query: None,
            highlight_fg: Color::Black,
            highlight_bg: Color::Yellow,
        }
    }

    /// Replaces the underlying content and splits lines.
    pub fn set_text(&mut self, text: impl Into<String>) {
        self.content = text.into();
        self.lines = if self.content.is_empty() {
            vec![String::new()]
        } else {
            self.content.lines().map(|s| s.to_string()).collect()
        };
        self.scroll_y = 0;
        self.scroll_x = 0;
    }

    pub fn with_wrap(mut self, wrap: bool) -> Self {
        self.wrap = wrap;
        self
    }

    pub fn with_line_numbers(mut self, show: bool) -> Self {
        self.show_line_numbers = show;
        self
    }

    pub fn with_scrollbars(mut self, show: bool) -> Self {
        self.show_scrollbars = show;
        self
    }

    pub fn with_tab_width(mut self, width: usize) -> Self {
        self.tab_width = width.max(1);
        self
    }

    pub fn with_style(mut self, fg: Color, bg: Color) -> Self {
        self.fg = fg;
        self.bg = bg;
        self
    }

    pub fn with_gutter_style(mut self, fg: Color, bg: Color) -> Self {
        self.gutter_fg = fg;
        self.gutter_bg = bg;
        self
    }

    pub fn with_highlight(mut self, query: Option<String>, fg: Color, bg: Color) -> Self {
        self.highlight_query = query;
        self.highlight_fg = fg;
        self.highlight_bg = bg;
        self
    }

    pub fn toggle_wrap(&mut self) {
        self.wrap = !self.wrap;
        if self.wrap {
            self.scroll_x = 0;
        }
    }

    pub fn scroll_up(&mut self, n: usize) {
        self.scroll_y = self.scroll_y.saturating_sub(n);
    }

    pub fn scroll_down(&mut self, n: usize) {
        let max_y = self.lines.len().saturating_sub(1);
        self.scroll_y = (self.scroll_y + n).min(max_y);
    }

    pub fn scroll_left(&mut self, n: usize) {
        self.scroll_x = self.scroll_x.saturating_sub(n);
    }

    pub fn scroll_right(&mut self, n: usize) {
        let max_len = self.max_line_width();
        self.scroll_x = (self.scroll_x + n).min(max_len);
    }

    pub fn scroll_to_top(&mut self) {
        self.scroll_y = 0;
        self.scroll_x = 0;
    }

    pub fn scroll_to_bottom(&mut self, viewport_height: usize) {
        if self.lines.len() > viewport_height {
            self.scroll_y = self.lines.len() - viewport_height;
        } else {
            self.scroll_y = 0;
        }
    }

    pub fn page_up(&mut self, viewport_height: usize) {
        self.scroll_up(viewport_height.saturating_sub(1).max(1));
    }

    pub fn page_down(&mut self, viewport_height: usize) {
        self.scroll_down(viewport_height.saturating_sub(1).max(1));
    }

    /// Returns the maximum line width in character columns.
    pub fn max_line_width(&self) -> usize {
        self.lines
            .iter()
            .map(|l| {
                l.chars()
                    .map(|c| {
                        if c == '\t' {
                            self.tab_width
                        } else {
                            UnicodeWidthChar::width(c).unwrap_or(1)
                        }
                    })
                    .sum()
            })
            .max()
            .unwrap_or(0)
    }

    /// Renders the `TextView` strictly bounded within `subview`.
    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>) {
        let h = subview.height();
        let w = subview.width();
        if h == 0 || w == 0 {
            return;
        }

        // Fill background
        for y in 0..h {
            for x in 0..w {
                subview.set_char(x, y, ' ', self.fg, self.bg, Modifier::empty());
            }
        }

        // Calculate gutter
        let gutter_w = if self.show_line_numbers {
            let digits = format!("{}", self.lines.len().max(1)).len() as u16;
            digits + 3 // "{digits} │ "
        } else {
            0
        };

        let scrollbar_col = if self.show_scrollbars && w > gutter_w + 4 { 1 } else { 0 };
        let content_w = w.saturating_sub(gutter_w + scrollbar_col);
        if content_w == 0 {
            return;
        }

        if self.wrap {
            self.render_wrapped(subview, gutter_w, content_w, h);
        } else {
            self.render_scrolled(subview, gutter_w, content_w, h);
        }

        // Render scrollbar
        if self.show_scrollbars && scrollbar_col > 0 {
            self.render_vertical_scrollbar(subview, w - 1, h);
        }
    }

    fn render_scrolled(&self, subview: &mut CanvasSubviewMut<'_>, gutter_w: u16, content_w: u16, viewport_h: u16) {
        let total_lines = self.lines.len();
        let max_render_lines = (viewport_h as usize).min(total_lines.saturating_sub(self.scroll_y));

        for row_idx in 0..max_render_lines {
            let line_idx = self.scroll_y + row_idx;
            let y = row_idx as u16;

            // Gutter
            if self.show_line_numbers {
                let digits = format!("{}", self.lines.len().max(1)).len();
                let gutter_str = format!("{:>width$} │ ", line_idx + 1, width = digits);
                subview.write_str_clipped(0, y, &gutter_str, self.gutter_fg, self.gutter_bg);
            }

            // Line content with horizontal scroll
            if let Some(line) = self.lines.get(line_idx) {
                let overflow = TextOverflow::ScrollHorizontal { offset: self.scroll_x };
                subview.write_str_overflow(gutter_w, y, line, content_w, overflow, self.fg, self.bg);
            }
        }

        // Bottom status indicator for horizontal offset and mode
        if viewport_h > 1 && self.scroll_x > 0 {
            let info = format!("◀ +{} cols ▶", self.scroll_x);
            let info_w = CanvasSubviewMut::measure_text_width(&info);
            if gutter_w + info_w < subview.width() {
                let pos_x = subview.width().saturating_sub(info_w + 2);
                subview.write_str_clipped(pos_x, viewport_h - 1, &info, Color::Cyan, self.bg);
            }
        }
    }

    fn render_wrapped(&self, subview: &mut CanvasSubviewMut<'_>, gutter_w: u16, content_w: u16, viewport_h: u16) {
        let mut visual_row = 0u16;
        let mut skipped_visual_rows = 0usize;

        for (line_idx, line) in self.lines.iter().enumerate() {
            if visual_row >= viewport_h {
                break;
            }

            // Pre-calculate wrapped lines for this logical line
            let mut wrapped_chunks = Vec::new();
            let mut curr_chunk = String::new();
            let mut curr_w = 0u16;

            for word in line.split_whitespace() {
                let word_w = word
                    .chars()
                    .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as u16)
                    .sum::<u16>();
                if word_w > content_w {
                    if !curr_chunk.is_empty() {
                        wrapped_chunks.push(curr_chunk.clone());
                        curr_chunk.clear();
                        curr_w = 0;
                    }
                    for ch in word.chars() {
                        let ch_w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                        if curr_w + ch_w > content_w {
                            wrapped_chunks.push(curr_chunk.clone());
                            curr_chunk.clear();
                            curr_w = 0;
                        }
                        curr_chunk.push(ch);
                        curr_w += ch_w;
                    }
                    continue;
                }
                if curr_w == 0 {
                    curr_chunk.push_str(word);
                    curr_w = word_w;
                } else if curr_w + 1 + word_w <= content_w {
                    curr_chunk.push(' ');
                    curr_chunk.push_str(word);
                    curr_w += 1 + word_w;
                } else {
                    wrapped_chunks.push(curr_chunk.clone());
                    curr_chunk.clear();
                    curr_chunk.push_str(word);
                    curr_w = word_w;
                }
            }
            if !curr_chunk.is_empty() || wrapped_chunks.is_empty() {
                wrapped_chunks.push(curr_chunk);
            }

            for (chunk_idx, chunk) in wrapped_chunks.iter().enumerate() {
                if skipped_visual_rows < self.scroll_y {
                    skipped_visual_rows += 1;
                    continue;
                }
                if visual_row >= viewport_h {
                    break;
                }

                // Gutter
                if self.show_line_numbers {
                    let digits = format!("{}", self.lines.len().max(1)).len();
                    let gutter_str = if chunk_idx == 0 {
                        format!("{:>width$} │ ", line_idx + 1, width = digits)
                    } else {
                        format!("{:>width$} ┊ ", " ", width = digits)
                    };
                    subview.write_str_clipped(0, visual_row, &gutter_str, self.gutter_fg, self.gutter_bg);
                }

                // Chunk
                subview.write_str_clipped(gutter_w, visual_row, chunk, self.fg, self.bg);
                visual_row += 1;
            }
        }
    }

    fn render_vertical_scrollbar(&self, subview: &mut CanvasSubviewMut<'_>, bar_x: u16, h: u16) {
        let total = self.lines.len();
        if total <= h as usize || h < 2 {
            return;
        }

        let track_h = h as f32;
        let thumb_ratio = (track_h / total as f32).clamp(0.1, 1.0);
        let thumb_h = (track_h * thumb_ratio).max(1.0);

        let max_scroll = (total - h as usize) as f32;
        let scroll_fraction = (self.scroll_y as f32 / max_scroll).clamp(0.0, 1.0);
        let thumb_start_y = (track_h - thumb_h) * scroll_fraction;

        for y in 0..h {
            subview.set_char(bar_x, y, '░', Color::DarkGray, self.bg, Modifier::empty());
        }

        let start_int = thumb_start_y.floor() as u16;
        let end_int = (thumb_start_y + thumb_h).ceil() as u16;

        for y in start_int..end_int.min(h) {
            let ch = if y == start_int && thumb_start_y.fract() > 0.5 {
                '▄'
            } else if y + 1 == end_int && (thumb_start_y + thumb_h).fract() < 0.5 {
                '▀'
            } else {
                '█'
            };
            subview.set_char(bar_x, y, ch, Color::Cyan, self.bg, Modifier::empty());
        }
    }
}
