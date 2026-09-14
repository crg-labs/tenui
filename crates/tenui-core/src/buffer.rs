use unicode_width::UnicodeWidthChar;

use crate::{
    cell::{Cell, CompactSymbol, Modifier},
    color::Color,
};

/// A 2D rectangle in terminal cell coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Rect {
    pub x: u16,
    pub y: u16,
    pub width: u16,
    pub height: u16,
}

/// Strategies for handling text that exceeds available box, rectangle, or container width.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TextOverflow {
    /// Strict clipping at boundary with no visual indicator.
    #[default]
    Clip,
    /// Truncates overflowing text cleanly and appends an ellipsis character ('…').
    Ellipsis,
    /// Wraps overflowing text onto subsequent lines within available vertical height.
    Wrap,
    /// Like [`Wrap`](Self::Wrap), but every continuation line (the 2nd onward) is
    /// prefixed with `marker` at the left edge and its text inset by `indent` columns —
    /// so wrapped code or log lines read as visibly-continued rather than as new entries
    /// (e.g. `indent: 2, marker: "↳ "`).
    WrapWithContinuation { indent: u16, marker: &'static str },
    /// Horizontally scrolled view starting from a character/column offset.
    ScrollHorizontal { offset: usize },
}

impl Rect {
    pub const ZERO: Self = Self {
        x: 0,
        y: 0,
        width: 0,
        height: 0,
    };

    pub fn new(x: u16, y: u16, width: u16, height: u16) -> Self {
        Self { x, y, width, height }
    }

    pub fn area(&self) -> u32 {
        self.width as u32 * self.height as u32
    }

    pub fn is_empty(&self) -> bool {
        self.width == 0 || self.height == 0
    }

    pub fn left(&self) -> u16 {
        self.x
    }

    pub fn right(&self) -> u16 {
        self.x.saturating_add(self.width)
    }

    pub fn top(&self) -> u16 {
        self.y
    }

    pub fn bottom(&self) -> u16 {
        self.y.saturating_add(self.height)
    }

    pub fn contains(&self, x: u16, y: u16) -> bool {
        x >= self.x && x < self.right() && y >= self.y && y < self.bottom()
    }

    pub fn contains_point(&self, pt: crate::geometry::Point) -> bool {
        self.contains(pt.x, pt.y)
    }

    pub fn intersects(&self, other: &Rect) -> bool {
        self.x < other.right() && self.right() > other.x && self.y < other.bottom() && self.bottom() > other.y
    }

    pub fn intersection(&self, other: &Rect) -> Self {
        let x1 = self.x.max(other.x);
        let y1 = self.y.max(other.y);
        let x2 = self.right().min(other.right());
        let y2 = self.bottom().min(other.bottom());

        if x2 > x1 && y2 > y1 {
            Self {
                x: x1,
                y: y1,
                width: x2 - x1,
                height: y2 - y1,
            }
        } else {
            Self::ZERO
        }
    }
}

/// A 2D grid of terminal cells.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Buffer {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
}

impl Buffer {
    pub fn new(width: u16, height: u16) -> Self {
        let len = width as usize * height as usize;
        Self {
            width,
            height,
            cells: vec![Cell::default(); len],
        }
    }

    pub fn empty(rect: Rect) -> Self {
        Self::new(rect.width, rect.height)
    }

    pub fn rect(&self) -> Rect {
        Rect::new(0, 0, self.width, self.height)
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.width == width && self.height == height {
            return;
        }
        self.width = width;
        self.height = height;
        self.cells.resize(width as usize * height as usize, Cell::default());
        self.clear();
    }

    pub fn clear(&mut self) {
        for cell in &mut self.cells {
            cell.reset();
        }
    }

    /// Clears the entire buffer and sets every cell's background to `bg`.
    pub fn clear_with_bg(&mut self, bg: Color) {
        self.cells.fill(Cell::empty_with_bg(bg));
    }

    /// Overwrites all cells with the poison sentinel, guaranteeing that any
    /// subsequent differential rendering pass diffs every single cell as dirty.
    pub fn poison_all(&mut self) {
        self.cells.fill(Cell::POISON);
    }

    #[inline]
    pub fn index_of(&self, x: u16, y: u16) -> Option<usize> {
        if x < self.width && y < self.height {
            Some(y as usize * self.width as usize + x as usize)
        } else {
            None
        }
    }

    #[inline]
    pub fn get(&self, x: u16, y: u16) -> Option<&Cell> {
        self.index_of(x, y).and_then(|idx| self.cells.get(idx))
    }

    #[inline]
    pub fn get_mut(&mut self, x: u16, y: u16) -> Option<&mut Cell> {
        self.index_of(x, y).and_then(|idx| self.cells.get_mut(idx))
    }

    pub fn set_char(&mut self, x: u16, y: u16, ch: char, fg: Color, bg: Color, modifier: Modifier) {
        let char_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
        if let Some(cell) = self.get_mut(x, y) {
            cell.set_char(ch);
            cell.set_fg(fg);
            cell.set_bg(bg);
            cell.set_modifier(modifier);
        }

        // Handle wide characters (width == 2)
        if char_width > 1
            && x + 1 < self.width
            && let Some(cont_cell) = self.get_mut(x + 1, y)
        {
            cont_cell.symbol = CompactSymbol::SPACE;
            cont_cell.fg = fg;
            cont_cell.bg = bg;
            cont_cell.modifier = modifier;
            cont_cell.width = 0;
            cont_cell.is_continuation = true;
        }
    }

    pub fn set_string(&mut self, mut x: u16, y: u16, s: &str, fg: Color, bg: Color, modifier: Modifier) {
        for ch in s.chars() {
            if x >= self.width {
                break;
            }
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            self.set_char(x, y, ch, fg, bg, modifier);
            x = x.saturating_add(w);
        }
    }

    pub fn fill(&mut self, rect: Rect, cell: Cell) {
        let intersection = self.rect().intersection(&rect);
        if intersection.is_empty() {
            return;
        }

        for y in intersection.top()..intersection.bottom() {
            for x in intersection.left()..intersection.right() {
                if let Some(c) = self.get_mut(x, y) {
                    *c = cell;
                }
            }
        }
    }

    /// Provides an isolated, hardware-clipped slice of the buffer.
    pub fn subview_mut(&mut self, clip: Rect) -> CanvasSubviewMut<'_> {
        let valid_clip = self.rect().intersection(&clip);
        CanvasSubviewMut {
            buffer: self,
            clip: valid_clip,
            bounds: valid_clip,
        }
    }
}

/// An isolated, hardware-clipped subview into a `Buffer`.
/// Local coordinates are relative to the subview's top-left bound.
pub struct CanvasSubviewMut<'a> {
    pub buffer: &'a mut Buffer,
    pub clip: Rect,
    pub bounds: Rect,
}

impl<'a> CanvasSubviewMut<'a> {
    pub fn rect(&self) -> Rect {
        self.clip
    }

    #[allow(clippy::misnamed_getters)]
    pub fn bounds(&self) -> Rect {
        self.clip
    }

    pub fn width(&self) -> u16 {
        self.clip.width
    }

    pub fn height(&self) -> u16 {
        self.clip.height
    }

    /// Creates a nested, hardware-clipped subview relative to this subview's local bounds.
    pub fn subview_mut(&mut self, local_clip: Rect) -> CanvasSubviewMut<'_> {
        let global_x = self.clip.x.saturating_add(local_clip.x);
        let global_y = self.clip.y.saturating_add(local_clip.y);
        let global_w = local_clip.width.min(self.clip.width.saturating_sub(local_clip.x));
        let global_h = local_clip.height.min(self.clip.height.saturating_sub(local_clip.y));
        let valid_clip = Rect::new(global_x, global_y, global_w, global_h);
        CanvasSubviewMut {
            buffer: self.buffer,
            clip: valid_clip,
            bounds: valid_clip,
        }
    }

    #[inline]
    fn to_global(&self, local_x: u16, local_y: u16) -> Option<(u16, u16)> {
        if local_x < self.clip.width && local_y < self.clip.height {
            Some((self.clip.x + local_x, self.clip.y + local_y))
        } else {
            None
        }
    }

    pub fn get(&self, local_x: u16, local_y: u16) -> Option<&Cell> {
        let (gx, gy) = self.to_global(local_x, local_y)?;
        self.buffer.get(gx, gy)
    }

    pub fn get_mut(&mut self, local_x: u16, local_y: u16) -> Option<&mut Cell> {
        let (gx, gy) = self.to_global(local_x, local_y)?;
        self.buffer.get_mut(gx, gy)
    }

    #[inline]
    pub fn get_cell_mut(&mut self, local_x: u16, local_y: u16) -> Option<&mut Cell> {
        self.get_mut(local_x, local_y)
    }

    pub fn set_cell(&mut self, local_x: u16, local_y: u16, cell: Cell) {
        if let Some(c) = self.get_mut(local_x, local_y) {
            *c = cell;
        }
    }

    /// Sets a character cell with foreground and background colors.
    pub fn set_cell_styled(&mut self, local_x: u16, local_y: u16, ch: char, fg: Color, bg: Color) {
        self.set_char(local_x, local_y, ch, fg, bg, Modifier::empty());
    }

    pub fn set_char(&mut self, local_x: u16, local_y: u16, ch: char, fg: Color, bg: Color, modifier: Modifier) {
        if let Some((gx, gy)) = self.to_global(local_x, local_y) {
            let char_width = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            if let Some(cell) = self.buffer.get_mut(gx, gy) {
                cell.set_char(ch);
                cell.set_fg(fg);
                if bg != Color::TRANSPARENT {
                    cell.set_bg(bg);
                }
                cell.set_modifier(modifier);
            }
            if char_width > 1
                && local_x + 1 < self.clip.width
                && let Some(cont) = self.buffer.get_mut(gx + 1, gy)
            {
                cont.symbol = CompactSymbol::SPACE;
                cont.fg = fg;
                if bg != Color::TRANSPARENT {
                    cont.bg = bg;
                }
                cont.modifier = modifier;
                cont.width = 0;
                cont.is_continuation = true;
            }
        }
    }

    pub fn set_string(&mut self, mut local_x: u16, local_y: u16, s: &str, fg: Color, bg: Color, modifier: Modifier) {
        for ch in s.chars() {
            if local_x >= self.clip.width {
                break;
            }
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            self.set_char(local_x, local_y, ch, fg, bg, modifier);
            local_x = local_x.saturating_add(w);
        }
    }

    pub fn fill(&mut self, cell: Cell) {
        for y in 0..self.clip.height {
            for x in 0..self.clip.width {
                if let Some(c) = self.get_mut(x, y) {
                    *c = cell;
                }
            }
        }
    }

    /// Zeroes out the subview bounds with the ambient background token.
    pub fn clear(&mut self, bg: Color) {
        for y in 0..self.clip.height {
            for x in 0..self.clip.width {
                if let Some(c) = self.get_mut(x, y) {
                    c.reset();
                    c.bg = bg;
                }
            }
        }
    }

    /// Hard bounds-checked cell write; drops out-of-bounds attempts safely.
    pub fn set_cell_clipped(&mut self, local_x: u16, local_y: u16, cell: Cell) {
        if local_x < self.clip.width
            && local_y < self.clip.height
            && let Some(c) = self.get_mut(local_x, local_y)
        {
            *c = cell;
        }
    }

    /// Truncated string write preventing boundary spillover into adjacent memory.
    /// Safely expands `\t` to next 4-column tab stops and strips unprintable control codes.
    pub fn write_str_clipped(&mut self, local_x: u16, local_y: u16, text: &str, fg: Color, bg: Color) {
        if local_y >= self.clip.height || local_x >= self.clip.width {
            return;
        }

        let mut curr_x = local_x;
        for ch in text.chars() {
            if curr_x >= self.clip.width {
                break;
            }
            if ch == '\t' {
                let next_tab = (curr_x + 4) - (curr_x % 4);
                let target = next_tab.min(self.clip.width);
                while curr_x < target {
                    self.set_char(curr_x, local_y, ' ', fg, bg, Modifier::empty());
                    curr_x = curr_x.saturating_add(1);
                }
                continue;
            }
            if ch < ' ' {
                continue;
            }
            let w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
            if curr_x + w > self.clip.width {
                break;
            }
            self.set_char(curr_x, local_y, ch, fg, bg, Modifier::empty());
            curr_x = curr_x.saturating_add(w);
        }
    }

    /// Writes text with a configurable overflow policy within `max_width`.
    /// Returns the total column width occupied on screen.
    #[allow(clippy::too_many_arguments)]
    pub fn write_str_overflow(
        &mut self,
        local_x: u16,
        local_y: u16,
        text: &str,
        max_width: u16,
        overflow: TextOverflow,
        fg: Color,
        bg: Color,
    ) -> u16 {
        if local_y >= self.clip.height || local_x >= self.clip.width || max_width == 0 {
            return 0;
        }

        let avail = max_width.min(self.clip.width - local_x);
        let total_w: u16 = text
            .chars()
            .map(|c| {
                if c == '\t' {
                    4
                } else {
                    UnicodeWidthChar::width(c).unwrap_or(1) as u16
                }
            })
            .sum();

        if total_w <= avail && !matches!(overflow, TextOverflow::ScrollHorizontal { .. }) {
            self.write_str_clipped(local_x, local_y, text, fg, bg);
            return total_w;
        }

        match overflow {
            TextOverflow::Clip => {
                let mut curr_x = local_x;
                for ch in text.chars() {
                    if curr_x >= local_x + avail {
                        break;
                    }
                    if ch == '\t' {
                        let next_tab = (curr_x + 4) - (curr_x % 4);
                        let target = next_tab.min(local_x + avail);
                        while curr_x < target {
                            self.set_char(curr_x, local_y, ' ', fg, bg, Modifier::empty());
                            curr_x = curr_x.saturating_add(1);
                        }
                        continue;
                    }
                    if ch < ' ' {
                        continue;
                    }
                    let w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    if curr_x + w > local_x + avail {
                        break;
                    }
                    self.set_char(curr_x, local_y, ch, fg, bg, Modifier::empty());
                    curr_x = curr_x.saturating_add(w);
                }
                curr_x - local_x
            }
            TextOverflow::Ellipsis => {
                if avail == 0 {
                    return 0;
                }
                if avail == 1 {
                    self.set_char(local_x, local_y, '…', fg, bg, Modifier::empty());
                    return 1;
                }
                let target_w = avail - 1; // reserve 1 cell for '…'
                let mut curr_x = local_x;
                for ch in text.chars() {
                    if ch == '\t' {
                        let next_tab = (curr_x + 4) - (curr_x % 4);
                        let target = next_tab.min(local_x + target_w);
                        while curr_x < target {
                            self.set_char(curr_x, local_y, ' ', fg, bg, Modifier::empty());
                            curr_x = curr_x.saturating_add(1);
                        }
                        continue;
                    }
                    if ch < ' ' {
                        continue;
                    }
                    let w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    if curr_x + w > local_x + target_w {
                        break;
                    }
                    self.set_char(curr_x, local_y, ch, fg, bg, Modifier::empty());
                    curr_x = curr_x.saturating_add(w);
                }
                self.set_char(curr_x, local_y, '…', fg, bg, Modifier::empty());
                curr_x.saturating_add(1) - local_x
            }
            TextOverflow::Wrap => {
                let max_lines = self.clip.height.saturating_sub(local_y);
                let _ = self.write_str_wrapped(local_x, local_y, text, avail, max_lines, fg, bg);
                avail
            }
            TextOverflow::WrapWithContinuation { indent, marker } => {
                let max_lines = self.clip.height.saturating_sub(local_y);
                let _ = self.write_str_wrapped_cont(local_x, local_y, text, avail, max_lines, indent, marker, fg, bg);
                avail
            }
            TextOverflow::ScrollHorizontal { offset } => {
                let mut curr_x = local_x;
                let mut col_accum = 0usize;
                for ch in text.chars() {
                    let w = if ch == '\t' {
                        4
                    } else {
                        UnicodeWidthChar::width(ch).unwrap_or(1)
                    };
                    if col_accum + w <= offset {
                        col_accum += w;
                        continue;
                    }
                    if curr_x >= local_x + avail {
                        break;
                    }
                    if ch == '\t' {
                        let next_tab = (curr_x + 4) - (curr_x % 4);
                        let target = next_tab.min(local_x + avail);
                        while curr_x < target {
                            self.set_char(curr_x, local_y, ' ', fg, bg, Modifier::empty());
                            curr_x = curr_x.saturating_add(1);
                        }
                        col_accum += w;
                        continue;
                    }
                    if ch < ' ' {
                        continue;
                    }
                    let w16 = w as u16;
                    if curr_x + w16 > local_x + avail {
                        break;
                    }
                    self.set_char(curr_x, local_y, ch, fg, bg, Modifier::empty());
                    curr_x = curr_x.saturating_add(w16);
                    col_accum += w;
                }
                curr_x - local_x
            }
        }
    }

    /// Writes word-wrapped text across multiple lines within `max_width` up to `max_lines`.
    /// Oversized words that exceed `max_width` are wrapped character-by-character.
    /// Returns the number of lines written.
    #[allow(clippy::too_many_arguments)]
    pub fn write_str_wrapped(
        &mut self,
        local_x: u16,
        local_y: u16,
        text: &str,
        max_width: u16,
        max_lines: u16,
        fg: Color,
        bg: Color,
    ) -> u16 {
        if max_width == 0 || max_lines == 0 || local_y >= self.clip.height || local_x >= self.clip.width {
            return 0;
        }
        let avail_w = max_width.min(self.clip.width - local_x);
        let mut line_idx = 0u16;
        let mut curr_line = String::new();
        let mut curr_w = 0u16;

        for word in text.split_whitespace() {
            let word_w: u16 = word
                .chars()
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as u16)
                .sum();
            if word_w > avail_w {
                // Word is wider than entire available line width! Break word character-by-character
                if !curr_line.is_empty() {
                    self.write_str_clipped(local_x, local_y + line_idx, &curr_line, fg, bg);
                    line_idx += 1;
                    if line_idx >= max_lines || local_y + line_idx >= self.clip.height {
                        return line_idx;
                    }
                    curr_line.clear();
                    curr_w = 0;
                }
                for ch in word.chars() {
                    let ch_w = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    if curr_w + ch_w > avail_w {
                        self.write_str_clipped(local_x, local_y + line_idx, &curr_line, fg, bg);
                        line_idx += 1;
                        if line_idx >= max_lines || local_y + line_idx >= self.clip.height {
                            return line_idx;
                        }
                        curr_line.clear();
                        curr_w = 0;
                    }
                    curr_line.push(ch);
                    curr_w += ch_w;
                }
                continue;
            }
            if curr_w == 0 {
                curr_line.push_str(word);
                curr_w = word_w;
            } else if curr_w + 1 + word_w <= avail_w {
                curr_line.push(' ');
                curr_line.push_str(word);
                curr_w += 1 + word_w;
            } else {
                self.write_str_clipped(local_x, local_y + line_idx, &curr_line, fg, bg);
                line_idx += 1;
                if line_idx >= max_lines || local_y + line_idx >= self.clip.height {
                    return line_idx;
                }
                curr_line.clear();
                curr_line.push_str(word);
                curr_w = word_w;
            }
        }
        if !curr_line.is_empty() && line_idx < max_lines && local_y + line_idx < self.clip.height {
            self.write_str_clipped(local_x, local_y + line_idx, &curr_line, fg, bg);
            line_idx += 1;
        }
        line_idx
    }

    /// Word-wraps `text` like [`write_str_wrapped`](Self::write_str_wrapped), but prefixes
    /// every continuation line (2nd onward) with `marker` at `local_x` and insets its text by
    /// `indent` columns. The first line uses the full `max_width`; continuation lines wrap
    /// within `max_width - indent`. Over-long words are hard-broken per line. Returns the
    /// number of lines written.
    ///
    /// This is the renderer for [`TextOverflow::WrapWithContinuation`].
    #[allow(clippy::too_many_arguments)]
    pub fn write_str_wrapped_cont(
        &mut self,
        local_x: u16,
        local_y: u16,
        text: &str,
        max_width: u16,
        max_lines: u16,
        indent: u16,
        marker: &str,
        fg: Color,
        bg: Color,
    ) -> u16 {
        if max_width == 0 || max_lines == 0 || local_y >= self.clip.height || local_x >= self.clip.width {
            return 0;
        }
        let avail_w = max_width.min(self.clip.width - local_x);
        // Guarantee continuation lines keep at least one usable column.
        let indent = indent.min(avail_w.saturating_sub(1));
        let cont_w = avail_w - indent;
        // Only ever produce as many lines as can actually be drawn — bounding the working
        // buffer to the viewport instead of the (untrusted, possibly huge) input length.
        let cap = max_lines.min(self.clip.height - local_y) as usize;
        if cap == 0 {
            return 0;
        }

        // Greedy word-wrap into lines, using the wider first-line width for line 0 and the
        // inset width for the rest. Re-evaluating per line keeps a word that fits line 0 but
        // not a continuation line correctly hard-broken instead of overflowing.
        let width_for = |idx: usize| -> u16 { if idx == 0 { avail_w } else { cont_w } };
        let char_w = |c: char| -> u16 { UnicodeWidthChar::width(c).unwrap_or(1) as u16 };

        let mut lines: Vec<String> = Vec::new();
        let mut curr = String::new();
        let mut curr_w = 0u16;

        // Stop as soon as `cap` completed lines are buffered — the line under construction
        // would be line index `cap`, which is off-screen and never drawn.
        'words: for word in text.split_whitespace() {
            let word_w: u16 = word.chars().map(char_w).sum();
            loop {
                let avail = width_for(lines.len());
                if curr_w == 0 {
                    if word_w <= avail {
                        curr.push_str(word);
                        curr_w = word_w;
                    } else {
                        // Word wider than a whole line: hard-break it character-by-character.
                        for ch in word.chars() {
                            let a = width_for(lines.len());
                            let cw = char_w(ch);
                            if curr_w > 0 && curr_w + cw > a {
                                lines.push(std::mem::take(&mut curr));
                                curr_w = 0;
                                if lines.len() >= cap {
                                    break 'words;
                                }
                            }
                            curr.push(ch);
                            curr_w += cw;
                        }
                    }
                    break;
                } else if curr_w + 1 + word_w <= avail {
                    curr.push(' ');
                    curr.push_str(word);
                    curr_w += 1 + word_w;
                    break;
                } else {
                    // Doesn't fit the current line: flush and re-evaluate on the next line.
                    lines.push(std::mem::take(&mut curr));
                    curr_w = 0;
                    if lines.len() >= cap {
                        break 'words;
                    }
                }
            }
        }
        if !curr.is_empty() && lines.len() < cap {
            lines.push(curr);
        }

        let mut written = 0u16;
        for (i, line) in lines.iter().enumerate() {
            let y = local_y + written;
            if i == 0 {
                self.write_str_clipped(local_x, y, line, fg, bg);
            } else {
                if !marker.is_empty() {
                    self.write_str_clipped(local_x, y, marker, fg, bg);
                }
                self.write_str_clipped(local_x + indent, y, line, fg, bg);
            }
            written += 1;
        }
        written
    }

    /// Measures single-line text width in terminal character cells.
    pub fn measure_text_width(text: &str) -> u16 {
        text.chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as u16)
            .sum()
    }

    /// Measures the dimensions (width, height) that wrapped text would occupy.
    pub fn measure_wrapped(text: &str, max_width: u16) -> (u16, u16) {
        if max_width == 0 || text.is_empty() {
            return (0, 0);
        }
        let mut lines = 0u16;
        let mut max_line_w = 0u16;
        let mut curr_w = 0u16;

        for word in text.split_whitespace() {
            let word_w: u16 = word
                .chars()
                .map(|c| UnicodeWidthChar::width(c).unwrap_or(1) as u16)
                .sum();
            if curr_w == 0 {
                curr_w = word_w;
            } else if curr_w + 1 + word_w <= max_width {
                curr_w += 1 + word_w;
            } else {
                max_line_w = max_line_w.max(curr_w);
                lines += 1;
                curr_w = word_w;
            }
        }
        if curr_w > 0 {
            max_line_w = max_line_w.max(curr_w);
            lines += 1;
        }
        (max_line_w.min(max_width), lines)
    }

    /// Renders a pulsating or static dashed border ('╌', '╎') around the subview bounds.
    pub fn draw_dashed_border(&mut self, color: Color) {
        let w = self.clip.width;
        let h = self.clip.height;
        if w < 2 || h < 2 {
            return;
        }

        self.set_char(0, 0, '┌', color, Color::TRANSPARENT, Modifier::empty());
        self.set_char(w - 1, 0, '┐', color, Color::TRANSPARENT, Modifier::empty());
        self.set_char(0, h - 1, '└', color, Color::TRANSPARENT, Modifier::empty());
        self.set_char(w - 1, h - 1, '┘', color, Color::TRANSPARENT, Modifier::empty());

        for x in 1..(w - 1) {
            self.set_char(x, 0, '╌', color, Color::TRANSPARENT, Modifier::empty());
            self.set_char(x, h - 1, '╌', color, Color::TRANSPARENT, Modifier::empty());
        }
        for y in 1..(h - 1) {
            self.set_char(0, y, '╎', color, Color::TRANSPARENT, Modifier::empty());
            self.set_char(w - 1, y, '╎', color, Color::TRANSPARENT, Modifier::empty());
        }
    }

    /// Additively boosts background cell luminance across all cells in the subview by factor.
    pub fn boost_background_luminance(&mut self, factor: f32) {
        for y in 0..self.clip.height {
            for x in 0..self.clip.width {
                if let Some(cell) = self.get_cell_mut(x, y) {
                    cell.bg = cell.bg.brighten(factor);
                }
            }
        }
    }

    /// Floats an ephemeral text badge centered inside the subview.
    pub fn write_centered_badge(&mut self, badge: &str, fg: Color, bg: Color) {
        let w = self.clip.width;
        let h = self.clip.height;
        let badge_len = badge.chars().count() as u16;
        if w < badge_len || h == 0 {
            return;
        }
        let start_x = (w - badge_len) / 2;
        let start_y = h / 2;
        self.write_str_clipped(start_x, start_y, badge, fg, bg);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{cell::Modifier, color::Color};

    // ---- Rect tests ----

    #[test]
    fn rect_area() {
        assert_eq!(Rect::new(0, 0, 10, 5).area(), 50);
        assert_eq!(Rect::ZERO.area(), 0);
    }

    #[test]
    fn rect_is_empty() {
        assert!(Rect::ZERO.is_empty());
        assert!(Rect::new(5, 5, 0, 10).is_empty());
        assert!(Rect::new(5, 5, 10, 0).is_empty());
        assert!(!Rect::new(0, 0, 1, 1).is_empty());
    }

    #[test]
    fn rect_edges() {
        let r = Rect::new(3, 7, 10, 5);
        assert_eq!(r.left(), 3);
        assert_eq!(r.right(), 13);
        assert_eq!(r.top(), 7);
        assert_eq!(r.bottom(), 12);
    }

    #[test]
    fn rect_contains() {
        let r = Rect::new(2, 2, 4, 4);
        assert!(r.contains(2, 2));
        assert!(r.contains(5, 5));
        assert!(!r.contains(6, 2));
        assert!(!r.contains(2, 6));
        assert!(!r.contains(1, 2));
    }

    #[test]
    fn rect_intersects_overlapping() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        assert!(a.intersects(&b));
        assert!(b.intersects(&a));
    }

    #[test]
    fn rect_intersects_non_overlapping() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(10, 10, 5, 5);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn rect_intersects_edge_touching() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(5, 0, 5, 5);
        assert!(!a.intersects(&b));
    }

    #[test]
    fn rect_intersection_overlap() {
        let a = Rect::new(0, 0, 10, 10);
        let b = Rect::new(5, 5, 10, 10);
        let i = a.intersection(&b);
        assert_eq!(i, Rect::new(5, 5, 5, 5));
    }

    #[test]
    fn rect_intersection_no_overlap() {
        let a = Rect::new(0, 0, 5, 5);
        let b = Rect::new(10, 10, 5, 5);
        assert_eq!(a.intersection(&b), Rect::ZERO);
    }

    #[test]
    fn rect_intersection_zero_size() {
        let a = Rect::ZERO;
        let b = Rect::new(0, 0, 10, 10);
        assert_eq!(a.intersection(&b), Rect::ZERO);
    }

    // Buffer tests

    #[test]
    fn buffer_new_dimensions() {
        let buf = Buffer::new(20, 10);
        assert_eq!(buf.width, 20);
        assert_eq!(buf.height, 10);
        assert_eq!(buf.cells.len(), 200);
    }

    #[test]
    fn buffer_empty_from_rect() {
        let buf = Buffer::empty(Rect::new(5, 5, 8, 4));
        assert_eq!(buf.width, 8);
        assert_eq!(buf.height, 4);
    }

    #[test]
    fn buffer_resize_clears() {
        let mut buf = Buffer::new(10, 10);
        buf.set_char(0, 0, 'X', Color::Red, Color::Blue, Modifier::BOLD);
        buf.resize(5, 5);
        assert_eq!(buf.width, 5);
        assert_eq!(buf.height, 5);
        let cell = buf.get(0, 0).unwrap();
        assert_eq!(cell.fg, Color::Reset);
    }

    #[test]
    fn buffer_resize_noop_same_size() {
        let mut buf = Buffer::new(10, 10);
        buf.set_char(0, 0, 'X', Color::Red, Color::Reset, Modifier::empty());
        buf.resize(10, 10);
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "X");
    }

    #[test]
    fn buffer_clear() {
        let mut buf = Buffer::new(5, 5);
        buf.set_char(2, 2, 'A', Color::Red, Color::Blue, Modifier::empty());
        buf.clear();
        assert_eq!(buf.get(2, 2).unwrap().as_str(), " ");
        assert_eq!(buf.get(2, 2).unwrap().fg, Color::Reset);
    }

    #[test]
    fn buffer_clear_with_bg() {
        let mut buf = Buffer::new(5, 5);
        buf.clear_with_bg(Color::Blue);
        assert_eq!(buf.get(0, 0).unwrap().bg, Color::Blue);
        assert_eq!(buf.get(4, 4).unwrap().bg, Color::Blue);
    }

    #[test]
    fn buffer_poison_all() {
        let mut buf = Buffer::new(3, 3);
        buf.poison_all();
        for cell in &buf.cells {
            assert_eq!(*cell, Cell::POISON);
        }
    }

    #[test]
    fn buffer_get_out_of_bounds() {
        let buf = Buffer::new(10, 10);
        assert!(buf.get(10, 0).is_none());
        assert!(buf.get(0, 10).is_none());
        assert!(buf.get(100, 100).is_none());
    }

    #[test]
    fn buffer_get_mut_out_of_bounds() {
        let mut buf = Buffer::new(10, 10);
        assert!(buf.get_mut(10, 0).is_none());
        assert!(buf.get_mut(0, 10).is_none());
    }

    #[test]
    fn buffer_set_char_ascii() {
        let mut buf = Buffer::new(10, 5);
        buf.set_char(3, 2, 'Z', Color::Green, Color::Reset, Modifier::BOLD);
        let cell = buf.get(3, 2).unwrap();
        assert_eq!(cell.as_str(), "Z");
        assert_eq!(cell.fg, Color::Green);
        assert_eq!(cell.modifier, Modifier::BOLD);
    }

    #[test]
    fn buffer_set_char_wide_cjk() {
        let mut buf = Buffer::new(10, 5);
        buf.set_char(2, 0, '中', Color::Reset, Color::Reset, Modifier::empty());
        let cell = buf.get(2, 0).unwrap();
        assert_eq!(cell.as_str(), "中");
        let cont = buf.get(3, 0).unwrap();
        assert!(cont.is_continuation);
        assert_eq!(cont.width, 0);
    }

    #[test]
    fn buffer_set_char_wide_at_right_edge() {
        let mut buf = Buffer::new(5, 1);
        buf.set_char(4, 0, '中', Color::Reset, Color::Reset, Modifier::empty());
        let cell = buf.get(4, 0).unwrap();
        assert_eq!(cell.as_str(), "中");
    }

    #[test]
    fn buffer_set_string_clips_at_boundary() {
        let mut buf = Buffer::new(5, 1);
        buf.set_string(0, 0, "Hello, World!", Color::Reset, Color::Reset, Modifier::empty());
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "H");
        assert_eq!(buf.get(4, 0).unwrap().as_str(), "o");
    }

    #[test]
    fn buffer_fill_partial_overlap() {
        let mut buf = Buffer::new(10, 10);
        let fill_cell = Cell::from_char('X');
        buf.fill(Rect::new(8, 8, 5, 5), fill_cell);
        assert_eq!(buf.get(8, 8).unwrap().as_str(), "X");
        assert_eq!(buf.get(9, 9).unwrap().as_str(), "X");
        assert_eq!(buf.get(7, 7).unwrap().as_str(), " ");
    }

    #[test]
    fn buffer_fill_no_overlap() {
        let mut buf = Buffer::new(5, 5);
        let fill_cell = Cell::from_char('X');
        buf.fill(Rect::new(10, 10, 5, 5), fill_cell);
        assert_eq!(buf.get(0, 0).unwrap().as_str(), " ");
    }

    // CanvasSubviewMut tests

    #[test]
    fn subview_clips_to_buffer() {
        let mut buf = Buffer::new(10, 10);
        let sub = buf.subview_mut(Rect::new(2, 2, 5, 5));
        assert_eq!(sub.width(), 5);
        assert_eq!(sub.height(), 5);
    }

    #[test]
    fn subview_clips_past_buffer_edge() {
        let mut buf = Buffer::new(10, 10);
        let sub = buf.subview_mut(Rect::new(8, 8, 10, 10));
        assert_eq!(sub.width(), 2);
        assert_eq!(sub.height(), 2);
    }

    #[test]
    fn subview_get_out_of_bounds() {
        let mut buf = Buffer::new(10, 10);
        let sub = buf.subview_mut(Rect::new(0, 0, 5, 5));
        assert!(sub.get(5, 0).is_none());
        assert!(sub.get(0, 5).is_none());
    }

    #[test]
    fn subview_set_char_maps_to_global() {
        let mut buf = Buffer::new(20, 20);
        {
            let mut sub = buf.subview_mut(Rect::new(5, 5, 10, 10));
            sub.set_char(0, 0, 'A', Color::Reset, Color::Reset, Modifier::empty());
            sub.set_char(2, 3, 'B', Color::Reset, Color::Reset, Modifier::empty());
        }
        assert_eq!(buf.get(5, 5).unwrap().as_str(), "A");
        assert_eq!(buf.get(7, 8).unwrap().as_str(), "B");
    }

    #[test]
    fn nested_subview_clipping() {
        let mut buf = Buffer::new(20, 20);
        {
            let mut outer = buf.subview_mut(Rect::new(5, 5, 10, 10));
            let mut inner = outer.subview_mut(Rect::new(2, 2, 4, 4));
            inner.set_char(0, 0, 'X', Color::Reset, Color::Reset, Modifier::empty());
        }
        assert_eq!(buf.get(7, 7).unwrap().as_str(), "X");
    }

    #[test]
    fn subview_write_str_clipped_basic() {
        let mut buf = Buffer::new(10, 3);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 5, 1));
            sub.write_str_clipped(0, 0, "Hello World", Color::Reset, Color::Reset);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "H");
        assert_eq!(buf.get(4, 0).unwrap().as_str(), "o");
        assert_eq!(buf.get(5, 0).unwrap().as_str(), " ");
    }

    #[test]
    fn subview_write_str_clipped_tabs() {
        let mut buf = Buffer::new(20, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 20, 1));
            sub.write_str_clipped(0, 0, "A\tB", Color::Reset, Color::Reset);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "A");
        assert_eq!(buf.get(4, 0).unwrap().as_str(), "B");
    }

    #[test]
    fn subview_write_str_clipped_strips_control_chars() {
        let mut buf = Buffer::new(10, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 1));
            sub.write_str_clipped(0, 0, "A\x01\x02B", Color::Reset, Color::Reset);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "A");
        assert_eq!(buf.get(1, 0).unwrap().as_str(), "B");
    }

    #[test]
    fn subview_write_str_clipped_out_of_bounds_y() {
        let mut buf = Buffer::new(10, 2);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 2));
            sub.write_str_clipped(0, 5, "offscreen", Color::Reset, Color::Reset);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), " ");
    }

    #[test]
    fn subview_overflow_clip() {
        let mut buf = Buffer::new(10, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 1));
            let w = sub.write_str_overflow(0, 0, "Hello World", 5, TextOverflow::Clip, Color::Reset, Color::Reset);
            assert_eq!(w, 5);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "H");
        assert_eq!(buf.get(4, 0).unwrap().as_str(), "o");
        assert_eq!(buf.get(5, 0).unwrap().as_str(), " ");
    }

    #[test]
    fn subview_overflow_ellipsis() {
        let mut buf = Buffer::new(20, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 20, 1));
            let w = sub.write_str_overflow(
                0,
                0,
                "Hello World",
                6,
                TextOverflow::Ellipsis,
                Color::Reset,
                Color::Reset,
            );
            assert!(w <= 6);
        }
        let last_visible = (0..6u16).rev().find(|&x| buf.get(x, 0).unwrap().as_str() == "…");
        assert!(last_visible.is_some(), "ellipsis character should appear");
    }

    #[test]
    fn subview_overflow_ellipsis_single_cell() {
        let mut buf = Buffer::new(10, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 1));
            let w = sub.write_str_overflow(0, 0, "Hello", 1, TextOverflow::Ellipsis, Color::Reset, Color::Reset);
            assert_eq!(w, 1);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "…");
    }

    #[test]
    fn subview_overflow_scroll_horizontal() {
        let mut buf = Buffer::new(20, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 20, 1));
            let w = sub.write_str_overflow(
                0,
                0,
                "Hello World",
                20,
                TextOverflow::ScrollHorizontal { offset: 6 },
                Color::Reset,
                Color::Reset,
            );
            assert!(w > 0);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "W");
    }

    #[test]
    fn subview_overflow_text_fits() {
        let mut buf = Buffer::new(20, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 20, 1));
            let w = sub.write_str_overflow(0, 0, "Hi", 20, TextOverflow::Clip, Color::Reset, Color::Reset);
            assert_eq!(w, 2);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "H");
        assert_eq!(buf.get(1, 0).unwrap().as_str(), "i");
    }

    #[test]
    fn subview_overflow_zero_width() {
        let mut buf = Buffer::new(10, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 1));
            let w = sub.write_str_overflow(0, 0, "Hello", 0, TextOverflow::Clip, Color::Reset, Color::Reset);
            assert_eq!(w, 0);
        }
    }

    #[test]
    fn subview_write_str_wrapped_basic() {
        let mut buf = Buffer::new(10, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 5));
            let lines = sub.write_str_wrapped(0, 0, "hello world foo", 5, 5, Color::Reset, Color::Reset);
            assert!(lines >= 2);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "h");
    }

    #[test]
    fn subview_write_str_wrapped_oversized_word() {
        let mut buf = Buffer::new(10, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 5));
            let lines = sub.write_str_wrapped(0, 0, "abcdefghij", 3, 5, Color::Reset, Color::Reset);
            assert!(lines > 1, "oversized word should be broken");
        }
    }

    #[test]
    fn subview_write_str_wrapped_max_lines_limit() {
        let mut buf = Buffer::new(10, 10);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 10));
            let lines = sub.write_str_wrapped(0, 0, "a b c d e f g h i j", 3, 2, Color::Reset, Color::Reset);
            assert!(lines <= 2);
        }
    }

    #[test]
    fn measure_text_width_ascii() {
        assert_eq!(CanvasSubviewMut::measure_text_width("hello"), 5);
    }

    #[test]
    fn measure_text_width_wide_chars() {
        assert_eq!(CanvasSubviewMut::measure_text_width("中文"), 4);
    }

    #[test]
    fn measure_text_width_empty() {
        assert_eq!(CanvasSubviewMut::measure_text_width(""), 0);
    }

    #[test]
    fn measure_wrapped_basic() {
        let (w, h) = CanvasSubviewMut::measure_wrapped("hello world", 6);
        assert_eq!(h, 2);
        assert!(w <= 6);
    }

    #[test]
    fn measure_wrapped_empty() {
        let (w, h) = CanvasSubviewMut::measure_wrapped("", 10);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
    }

    #[test]
    fn measure_wrapped_zero_width() {
        let (w, h) = CanvasSubviewMut::measure_wrapped("hello", 0);
        assert_eq!(w, 0);
        assert_eq!(h, 0);
    }

    #[test]
    fn subview_fill() {
        let mut buf = Buffer::new(5, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(1, 1, 3, 3));
            sub.fill(Cell::from_char('*'));
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), " ");
        assert_eq!(buf.get(1, 1).unwrap().as_str(), "*");
        assert_eq!(buf.get(3, 3).unwrap().as_str(), "*");
        assert_eq!(buf.get(4, 4).unwrap().as_str(), " ");
    }

    #[test]
    fn subview_clear() {
        let mut buf = Buffer::new(5, 5);
        buf.set_char(2, 2, 'X', Color::Red, Color::Blue, Modifier::BOLD);
        {
            let mut sub = buf.subview_mut(Rect::new(1, 1, 3, 3));
            sub.clear(Color::Green);
        }
        let cell = buf.get(2, 2).unwrap();
        assert_eq!(cell.as_str(), " ");
        assert_eq!(cell.bg, Color::Green);
    }

    #[test]
    fn subview_dashed_border() {
        let mut buf = Buffer::new(10, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 5));
            sub.draw_dashed_border(Color::White);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), "┌");
        assert_eq!(buf.get(9, 0).unwrap().as_str(), "┐");
        assert_eq!(buf.get(0, 4).unwrap().as_str(), "└");
        assert_eq!(buf.get(9, 4).unwrap().as_str(), "┘");
        assert_eq!(buf.get(1, 0).unwrap().as_str(), "╌");
        assert_eq!(buf.get(0, 1).unwrap().as_str(), "╎");
    }

    #[test]
    fn subview_dashed_border_too_small() {
        let mut buf = Buffer::new(1, 1);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 1, 1));
            sub.draw_dashed_border(Color::White);
        }
        assert_eq!(buf.get(0, 0).unwrap().as_str(), " ");
    }

    #[test]
    fn subview_write_centered_badge() {
        let mut buf = Buffer::new(10, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 5));
            sub.write_centered_badge("Hi", Color::White, Color::Reset);
        }
        let center_x = (10 - 2) / 2;
        let center_y = 5 / 2;
        assert_eq!(buf.get(center_x, center_y).unwrap().as_str(), "H");
        assert_eq!(buf.get(center_x + 1, center_y).unwrap().as_str(), "i");
    }

    #[test]
    fn subview_set_cell_clipped_in_bounds() {
        let mut buf = Buffer::new(5, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 5, 5));
            sub.set_cell_clipped(2, 2, Cell::from_char('Q'));
        }
        assert_eq!(buf.get(2, 2).unwrap().as_str(), "Q");
    }

    #[test]
    fn subview_set_cell_clipped_out_of_bounds() {
        let mut buf = Buffer::new(5, 5);
        {
            let mut sub = buf.subview_mut(Rect::new(0, 0, 5, 5));
            sub.set_cell_clipped(10, 10, Cell::from_char('Q'));
        }
        // should not panic, buffer unchanged
        assert_eq!(buf.get(4, 4).unwrap().as_str(), " ");
    }
}
