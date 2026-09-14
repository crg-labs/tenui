use tenui_core::{CanvasSubviewMut, Color};

/// Fixed-capacity ring buffer log viewer with viewport slicing and tail -f smart pause.
pub struct LogStreamer {
    capacity: usize,
    buffer: Vec<String>,
    write_idx: usize,
    total_lines: usize,
    scroll_offset: usize,
    pub is_auto_scroll: bool,
}

impl LogStreamer {
    pub fn new(capacity: usize) -> Self {
        let cap = capacity.max(1);
        Self {
            capacity: cap,
            buffer: Vec::with_capacity(cap),
            write_idx: 0,
            total_lines: 0,
            scroll_offset: 0,
            is_auto_scroll: true,
        }
    }

    /// Appends a new log line into the pre-allocated circular ring arena.
    pub fn push_line(&mut self, line: impl Into<String>) {
        let s = line.into();
        if self.buffer.len() < self.capacity {
            self.buffer.push(s);
        } else {
            self.buffer[self.write_idx] = s;
            self.write_idx = (self.write_idx + 1) % self.capacity;
        }
        self.total_lines += 1;

        if self.is_auto_scroll {
            self.scroll_to_bottom();
        }
    }

    /// Number of lines currently in memory.
    pub fn len(&self) -> usize {
        self.buffer.len()
    }

    pub fn is_empty(&self) -> bool {
        self.buffer.is_empty()
    }

    pub fn get_line(&self, idx: usize) -> Option<&str> {
        if idx >= self.buffer.len() {
            return None;
        }
        if self.buffer.len() < self.capacity {
            self.buffer.get(idx).map(|s| s.as_str())
        } else {
            let actual_idx = (self.write_idx + idx) % self.capacity;
            self.buffer.get(actual_idx).map(|s| s.as_str())
        }
    }

    pub fn scroll_up(&mut self, delta: usize) {
        if self.scroll_offset > delta {
            self.scroll_offset -= delta;
        } else {
            self.scroll_offset = 0;
        }
        self.is_auto_scroll = false;
    }

    pub fn scroll_down(&mut self, delta: usize) {
        let max_scroll = self.buffer.len().saturating_sub(1);
        self.scroll_offset = (self.scroll_offset + delta).min(max_scroll);
        if self.scroll_offset >= max_scroll {
            self.is_auto_scroll = true;
        }
    }

    pub fn scroll_to_bottom(&mut self) {
        self.scroll_offset = self.buffer.len().saturating_sub(1);
        self.is_auto_scroll = true;
    }

    /// Renders the visible viewport slice into the canvas subview.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = surface.width();
        let h = surface.height();
        if w == 0 || h == 0 || self.buffer.is_empty() {
            return;
        }

        let total = self.buffer.len();
        let end_idx = (self.scroll_offset + 1).min(total);
        let start_idx = end_idx.saturating_sub(h as usize);

        // Render viewport rows
        for row in 0..h {
            let line_idx = start_idx + row as usize;
            if line_idx < end_idx
                && let Some(line) = self.get_line(line_idx)
            {
                // Line number gutter
                let gutter = format!("{:4} │ ", line_idx + 1);
                surface.write_str_clipped(0, row, &gutter, Color::DarkGray, bg);

                let gutter_w = gutter.len() as u16;
                surface.write_str_clipped(gutter_w, row, line, fg, bg);
            }
        }

        // Status Pill: [⏸ Autoscroll Paused] in top-right corner if paused
        if !self.is_auto_scroll {
            let pill = " [⏸ Autoscroll Paused] ";
            let pill_len = pill.len() as u16;
            if w > pill_len + 2 {
                let px = w - pill_len - 1;
                surface.write_str_clipped(px, 0, pill, Color::BLACK, Color::YELLOW);
            }
        }
    }
}
