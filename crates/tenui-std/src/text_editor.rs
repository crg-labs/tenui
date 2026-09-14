//! `TextEditor` — a multi-line editing widget that is a thin view over the shared
//! [`TextBuffer`] editing core (selection, branching undo, clipboard), per the spec's
//! "grapheme-aware, rope-ready" input widgets.

use tenui_core::{
    CanvasSubviewMut, Color, Modifier,
    clipboard::{Clipboard, ClipboardTarget},
};
use tenui_text::TextBuffer;
use unicode_width::UnicodeWidthChar;

use crate::theme::ThemePalette;

/// A soft-wrapped visual line: a byte range `[start, end)` into the buffer text. In non-wrap
/// mode there is exactly one per logical line; in wrap mode long logical lines split into
/// several.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct VisLine {
    pub start: usize,
    pub end: usize,
}

/// The foreground/background pairs a [`TextEditor`] draws with. [`TextEditor::render`] uses a
/// terminal-default set; [`TextEditor::render_themed`] derives one from a [`ThemePalette`].
#[derive(Clone, Copy, Debug)]
struct EditorColors {
    fg: Color,
    bg: Color,
    sel_fg: Color,
    sel_bg: Color,
    caret_fg: Color,
    caret_bg: Color,
}

/// A multi-line text editor view bound to a [`TextBuffer`].
#[derive(Debug, Clone, Default)]
pub struct TextEditor {
    buffer: TextBuffer,
    pub scroll_top: usize,
    /// Soft-wrap long logical lines to the view width instead of clipping them.
    pub word_wrap: bool,
    /// Viewport wrap width in columns used when word_wrap is enabled.
    pub wrap_width: Option<u16>,
}

impl TextEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn with_word_wrap(mut self, wrap: bool) -> Self {
        self.word_wrap = wrap;
        self
    }

    pub fn with_wrap_width(mut self, width: u16) -> Self {
        self.wrap_width = Some(width);
        self
    }

    pub fn set_wrap_width(&mut self, width: u16) {
        self.wrap_width = Some(width);
    }

    pub fn from_text(text: &str) -> Self {
        Self {
            buffer: TextBuffer::from_text(text),
            scroll_top: 0,
            word_wrap: false,
            wrap_width: None,
        }
    }

    pub fn text(&self) -> &str {
        self.buffer.text()
    }

    pub fn buffer(&self) -> &TextBuffer {
        &self.buffer
    }

    pub fn buffer_mut(&mut self) -> &mut TextBuffer {
        &mut self.buffer
    }

    // editing (delegates to the shared core)

    pub fn insert_char(&mut self, ch: char) {
        self.buffer.insert_char(ch);
    }

    pub fn insert_str(&mut self, s: &str) {
        self.buffer.insert(s);
    }

    pub fn newline(&mut self) {
        self.buffer.insert("\n");
    }

    pub fn backspace(&mut self) {
        self.buffer.backspace();
    }

    pub fn delete_forward(&mut self) {
        self.buffer.delete_forward();
    }

    pub fn move_left(&mut self, extend: bool) {
        self.buffer.move_left(extend);
    }
    pub fn move_right(&mut self, extend: bool) {
        self.buffer.move_right(extend);
    }
    pub fn move_up(&mut self, extend: bool) {
        if self.word_wrap && self.wrap_width.is_some() {
            self.move_up_wrapped(self.wrap_width.unwrap(), extend);
        } else {
            self.buffer.move_up(extend);
        }
    }
    pub fn move_down(&mut self, extend: bool) {
        if self.word_wrap && self.wrap_width.is_some() {
            self.move_down_wrapped(self.wrap_width.unwrap(), extend);
        } else {
            self.buffer.move_down(extend);
        }
    }
    pub fn move_home(&mut self, extend: bool) {
        if self.word_wrap && self.wrap_width.is_some() {
            self.move_home_wrapped(self.wrap_width.unwrap(), extend);
        } else {
            self.buffer.move_home(extend);
        }
    }
    pub fn move_end(&mut self, extend: bool) {
        if self.word_wrap && self.wrap_width.is_some() {
            self.move_end_wrapped(self.wrap_width.unwrap(), extend);
        } else {
            self.buffer.move_end(extend);
        }
    }

    /// Moves the cursor up one visual row in a wrapped viewport of `width` columns.
    pub fn move_up_wrapped(&mut self, width: u16, extend: bool) {
        let vis = self.visual_lines(width as usize);
        if vis.is_empty() {
            return;
        }
        let text = self.buffer.text();
        let head = self.buffer.cursor().head.min(text.len());

        let mut caret_row = 0usize;
        for (i, l) in vis.iter().enumerate() {
            if l.start <= head {
                caret_row = i;
            } else {
                break;
            }
        }

        if caret_row == 0 {
            return;
        }

        let cur_line = vis[caret_row];
        let clamped_head = head.clamp(cur_line.start, cur_line.end);
        let cur_col: usize = text[cur_line.start..clamped_head]
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
            .sum();

        let target_row = caret_row - 1;
        let target_line = vis[target_row];
        let max_target_head = if target_row + 1 < vis.len() && target_line.end == vis[target_row + 1].start {
            target_line.end.saturating_sub(1).max(target_line.start)
        } else {
            target_line.end
        };

        let mut col = 0usize;
        let mut target_head = target_line.start;
        for ch in text[target_line.start..max_target_head].chars() {
            let chw = UnicodeWidthChar::width(ch).unwrap_or(1);
            if col + chw > cur_col {
                break;
            }
            col += chw;
            target_head += ch.len_utf8();
        }

        self.buffer.set_head(target_head, extend);
    }

    /// Moves the cursor down one visual row in a wrapped viewport of `width` columns.
    pub fn move_down_wrapped(&mut self, width: u16, extend: bool) {
        let vis = self.visual_lines(width as usize);
        if vis.is_empty() {
            return;
        }
        let text = self.buffer.text();
        let head = self.buffer.cursor().head.min(text.len());

        let mut caret_row = 0usize;
        for (i, l) in vis.iter().enumerate() {
            if l.start <= head {
                caret_row = i;
            } else {
                break;
            }
        }

        if caret_row + 1 >= vis.len() {
            return;
        }

        let cur_line = vis[caret_row];
        let clamped_head = head.clamp(cur_line.start, cur_line.end);
        let cur_col: usize = text[cur_line.start..clamped_head]
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
            .sum();

        let target_row = caret_row + 1;
        let target_line = vis[target_row];
        let max_target_head = if target_row + 1 < vis.len() && target_line.end == vis[target_row + 1].start {
            target_line.end.saturating_sub(1).max(target_line.start)
        } else {
            target_line.end
        };

        let mut col = 0usize;
        let mut target_head = target_line.start;
        for ch in text[target_line.start..max_target_head].chars() {
            let chw = UnicodeWidthChar::width(ch).unwrap_or(1);
            if col + chw > cur_col {
                break;
            }
            col += chw;
            target_head += ch.len_utf8();
        }

        self.buffer.set_head(target_head, extend);
    }

    /// Moves the cursor to the beginning of the current visual line in a wrapped viewport.
    pub fn move_home_wrapped(&mut self, width: u16, extend: bool) {
        let vis = self.visual_lines(width as usize);
        if vis.is_empty() {
            return;
        }
        let text = self.buffer.text();
        let head = self.buffer.cursor().head.min(text.len());

        let mut caret_row = 0usize;
        for (i, l) in vis.iter().enumerate() {
            if l.start <= head {
                caret_row = i;
            } else {
                break;
            }
        }

        let target = vis[caret_row].start;
        if head == target {
            self.buffer.move_home(extend);
        } else {
            self.buffer.set_head(target, extend);
        }
    }

    /// Moves the cursor to the end of the current visual line in a wrapped viewport.
    pub fn move_end_wrapped(&mut self, width: u16, extend: bool) {
        let vis = self.visual_lines(width as usize);
        if vis.is_empty() {
            return;
        }
        let text = self.buffer.text();
        let head = self.buffer.cursor().head.min(text.len());

        let mut caret_row = 0usize;
        for (i, l) in vis.iter().enumerate() {
            if l.start <= head {
                caret_row = i;
            } else {
                break;
            }
        }

        let cur_line = vis[caret_row];
        let max_target_head = if caret_row + 1 < vis.len() && cur_line.end == vis[caret_row + 1].start {
            cur_line.end.saturating_sub(1).max(cur_line.start)
        } else {
            cur_line.end
        };

        if head == max_target_head {
            self.buffer.move_end(extend);
        } else {
            self.buffer.set_head(max_target_head, extend);
        }
    }
    pub fn select_all(&mut self) {
        self.buffer.select_all();
    }

    // clipboard + undo

    pub fn copy(&self, clip: &mut Clipboard) -> String {
        self.buffer.copy(clip)
    }
    pub fn cut(&mut self, clip: &mut Clipboard) -> String {
        self.buffer.cut(clip)
    }
    pub fn paste(&mut self, clip: &Clipboard) {
        self.buffer.paste(clip);
    }
    pub fn undo(&mut self) -> bool {
        self.buffer.undo()
    }
    pub fn redo(&mut self, branch: usize) -> bool {
        self.buffer.redo(branch)
    }

    /// Computes the visual (soft-wrapped) lines for a `width`-column viewport as byte ranges
    /// into the buffer text. In non-wrap mode this is one entry per logical line; in wrap mode
    /// over-long logical lines are split at the last space that fits (hard-breaking mid-word
    /// when there is no space), measured in display columns so wide glyphs count as two.
    ///
    /// Shared by [`render`](Self::render), [`desired_height`](Self::desired_height), and
    /// [`hit_test`](Self::hit_test) so all three agree on where lines break.
    pub fn visual_lines(&self, width: usize) -> Vec<VisLine> {
        let text = self.buffer.text();
        let width = width.max(1);

        // Split into logical lines by byte offset (keeping empty lines).
        let mut logical: Vec<(usize, usize)> = Vec::new();
        let mut line_start = 0usize;
        for (i, ch) in text.char_indices() {
            if ch == '\n' {
                logical.push((line_start, i));
                line_start = i + ch.len_utf8();
            }
        }
        logical.push((line_start, text.len()));

        let mut out: Vec<VisLine> = Vec::new();
        for (ls, le) in logical {
            if !self.word_wrap || ls == le {
                out.push(VisLine { start: ls, end: le });
                continue;
            }
            let mut seg_start = ls;
            let mut cur_w = 0usize;
            let mut last_space_end: Option<usize> = None; // byte offset just past a space
            let mut idx = ls;
            while idx < le {
                let ch = text[idx..].chars().next().unwrap();
                let chw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if cur_w + chw > width && idx > seg_start {
                    let brk = match last_space_end {
                        Some(s) if s > seg_start => s,
                        _ => idx,
                    };
                    out.push(VisLine {
                        start: seg_start,
                        end: brk,
                    });
                    seg_start = brk;
                    last_space_end = None;
                    cur_w = text[seg_start..idx]
                        .chars()
                        .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
                        .sum();
                }
                if ch == ' ' {
                    last_space_end = Some(idx + 1);
                }
                cur_w += chw;
                idx += ch.len_utf8();
            }
            out.push(VisLine {
                start: seg_start,
                end: le,
            });
        }
        out
    }

    /// The number of visual rows the content needs at `width` columns, clamped to
    /// `[min, max]`. Lets a host container auto-grow a wrapping editor (e.g. a chat composer
    /// expanding from 1 to 6 rows as the user types) instead of the editor scrolling internally.
    pub fn desired_height(&self, width: u16, min: u16, max: u16) -> u16 {
        let lines = self.visual_lines(width as usize).len() as u16;
        let lo = min.max(1);
        let hi = max.max(lo);
        lines.clamp(lo, hi)
    }

    /// Maps a click at cell (`rel_x`, `rel_y`) — relative to the editor's top-left, honoring the
    /// current `scroll_top` and wrap `width` — to a byte offset in the buffer text, or `None`
    /// when the click lands below the last line. Place the caret there with
    /// `editor.buffer_mut().select_range(off, off)`.
    pub fn hit_test(&self, rel_x: u16, rel_y: u16, width: u16) -> Option<usize> {
        let vis = self.visual_lines(width as usize);
        let row = self.scroll_top + rel_y as usize;
        let line = vis.get(row)?;
        let text = self.buffer.text();
        let mut col = 0usize;
        let mut off = line.start;
        for ch in text[line.start..line.end].chars() {
            let chw = UnicodeWidthChar::width(ch).unwrap_or(1);
            if col + chw > rel_x as usize {
                break;
            }
            col += chw;
            off += ch.len_utf8();
        }
        Some(off)
    }

    /// Renders the editor into `subview` with terminal-default colors. When `focused`, draws the
    /// caret and any selection. Prefer [`render_themed`](Self::render_themed) on a themed UI.
    pub fn render(&mut self, subview: &mut CanvasSubviewMut<'_>, focused: bool) {
        let colors = EditorColors {
            fg: Color::White,
            bg: Color::Reset,
            sel_fg: Color::Black,
            sel_bg: Color::Cyan,
            caret_fg: Color::Black,
            caret_bg: Color::White,
        };
        self.render_with(subview, focused, colors);
    }

    /// Renders the editor using colors drawn from `palette`, so text, selection, and caret match
    /// the surrounding theme instead of hardcoded white-on-black.
    pub fn render_themed(&mut self, subview: &mut CanvasSubviewMut<'_>, focused: bool, palette: &ThemePalette) {
        let colors = EditorColors {
            fg: palette.fg,
            bg: palette.bg,
            sel_fg: palette.bg,
            sel_bg: palette.primary,
            caret_fg: palette.bg,
            caret_bg: palette.fg,
        };
        self.render_with(subview, focused, colors);
    }

    /// Shared rendering core for [`render`](Self::render) and
    /// [`render_themed`](Self::render_themed).
    fn render_with(&mut self, subview: &mut CanvasSubviewMut<'_>, focused: bool, colors: EditorColors) {
        let height = subview.height() as usize;
        let width = subview.width() as usize;
        if height == 0 || width == 0 {
            return;
        }

        self.wrap_width = Some(width as u16);
        let text = self.buffer.text().to_string();
        let vis = self.visual_lines(width);
        let head = self.buffer.cursor().head.min(text.len());

        // Selection as a sorted byte range, if any.
        let selection = {
            let (a, b) = self.buffer.selection();
            if a == b { None } else { Some((a.min(b), a.max(b))) }
        };

        // Caret visual row: the last line whose start is <= head (so a caret at a soft-wrap
        // boundary shows at column 0 of the following line, not trailing the previous one).
        let mut caret_row = 0usize;
        for (i, l) in vis.iter().enumerate() {
            if l.start <= head {
                caret_row = i;
            } else {
                break;
            }
        }
        let caret_col: usize = text[vis[caret_row].start..head]
            .chars()
            .map(|c| UnicodeWidthChar::width(c).unwrap_or(1))
            .sum();

        // Scrolling: don't scroll when everything fits (fixes premature independent scrolling);
        // otherwise keep the caret row within the viewport.
        if vis.len() <= height {
            self.scroll_top = 0;
        } else if caret_row < self.scroll_top {
            self.scroll_top = caret_row;
        } else if caret_row >= self.scroll_top + height {
            self.scroll_top = caret_row + 1 - height;
        }

        for y in 0..height {
            let idx = self.scroll_top + y;
            let Some(line) = vis.get(idx) else { break };
            let mut col = 0usize;
            let mut off = line.start;
            for ch in text[line.start..line.end].chars() {
                let chw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if col + chw > width {
                    break;
                }
                let selected = selection.is_some_and(|(s, e)| off >= s && off < e);
                let (fg, bg) = if selected {
                    (colors.sel_fg, colors.sel_bg)
                } else {
                    (colors.fg, colors.bg)
                };
                subview.set_char(col as u16, y as u16, ch, fg, bg, Modifier::empty());
                col += chw;
                off += ch.len_utf8();
            }
            // Pad the rest of the row with the (possibly themed) background.
            while col < width {
                subview.set_char(col as u16, y as u16, ' ', colors.fg, colors.bg, Modifier::empty());
                col += 1;
            }

            // Caret on this row.
            if focused && idx == caret_row {
                let draw_x = caret_col.min(width.saturating_sub(1));
                let ch = if head < line.end {
                    text[head..].chars().next().unwrap_or(' ')
                } else {
                    ' '
                };
                subview.set_char(
                    draw_x as u16,
                    y as u16,
                    ch,
                    colors.caret_fg,
                    colors.caret_bg,
                    Modifier::REVERSE,
                );
            }
        }
    }
}

impl ClipboardTarget for TextEditor {
    fn clipboard_copy(&self, clip: &mut Clipboard) -> String {
        self.buffer.copy(clip)
    }
    fn clipboard_cut(&mut self, clip: &mut Clipboard) -> String {
        self.buffer.cut(clip)
    }
    fn clipboard_paste(&mut self, clip: &Clipboard) {
        self.buffer.paste(clip)
    }
    fn clipboard_select_all(&mut self) {
        self.buffer.select_all()
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn test_editor_edit_and_clipboard() {
        let mut ed = TextEditor::from_text("hello world");
        ed.move_end(false);
        ed.insert_str("!");
        assert_eq!(ed.text(), "hello world!");

        // Select "world!" and cut it.
        for _ in 0..6 {
            ed.move_left(true);
        }
        let mut clip = Clipboard::new();
        ed.cut(&mut clip);
        assert_eq!(ed.text(), "hello ");
        assert_eq!(clip.register(), "world!");

        // Undo the cut.
        assert!(ed.undo());
        assert_eq!(ed.text(), "hello world!");
    }

    #[test]
    fn test_editor_renders_without_panic_and_scrolls() {
        let mut ed = TextEditor::from_text("l0\nl1\nl2\nl3\nl4\nl5");
        // Move caret to last line, then render a 3-row viewport.
        for _ in 0..5 {
            ed.move_down(false);
        }
        let mut buf = Buffer::new(10, 3);
        let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 3));
        ed.render(&mut sub, true);
        // Scrolled so the last line is visible.
        assert!(ed.scroll_top >= 3);
        assert_eq!(buf.get(0, 2).unwrap().symbol.as_str(), "l");
    }

    #[test]
    fn test_desired_height_wraps_and_clamps() {
        let ed = TextEditor::from_text("one two three four five six").with_word_wrap(true);
        // At width 10 this wraps to several rows; grows past min, capped at max.
        assert!(ed.desired_height(10, 1, 6) > 1);
        assert_eq!(ed.desired_height(10, 1, 2), 2, "clamped to max");
        // A single short line stays at the minimum.
        let one = TextEditor::from_text("hi").with_word_wrap(true);
        assert_eq!(one.desired_height(40, 3, 6), 3, "clamped up to min");
    }

    #[test]
    fn test_no_scroll_when_content_fits() {
        let mut ed = TextEditor::from_text("a\nb");
        ed.scroll_top = 5; // stale value
        let mut buf = Buffer::new(10, 6);
        let mut sub = buf.subview_mut(Rect::new(0, 0, 10, 6));
        ed.render(&mut sub, true);
        assert_eq!(ed.scroll_top, 0, "content fits the viewport, so no scrolling");
    }

    #[test]
    fn test_hit_test_maps_click_to_offset() {
        let ed = TextEditor::from_text("hello\nworld");
        // Row 1, column 2 → 'r' in "world": byte offset 6 ("hello\n") + 2 = 8.
        assert_eq!(ed.hit_test(2, 1, 40), Some(8));
        // Clicking past the last row yields nothing.
        assert_eq!(ed.hit_test(0, 9, 40), None);
    }

    #[test]
    fn test_render_themed_uses_palette_background() {
        let mut ed = TextEditor::from_text("hi");
        let palette = ThemePalette::catppuccin_mocha();
        let mut buf = Buffer::new(6, 1);
        let mut sub = buf.subview_mut(Rect::new(0, 0, 6, 1));
        ed.render_themed(&mut sub, false, &palette);
        // Text and padding carry the theme background, not Color::Reset.
        assert_eq!(buf.get(0, 0).unwrap().bg, palette.bg);
        assert_eq!(buf.get(5, 0).unwrap().bg, palette.bg);
        assert_eq!(buf.get(0, 0).unwrap().fg, palette.fg);
    }

    #[test]
    fn test_wrapped_navigation_up_down_home_end() {
        let mut ed = TextEditor::from_text("one two three four five six")
            .with_word_wrap(true)
            .with_wrap_width(10);

        let vis = ed.visual_lines(10);
        assert!(vis.len() >= 3);

        // Put cursor at the very end
        let end_pos = ed.text().len();
        ed.buffer_mut().select_range(end_pos, end_pos);
        assert_eq!(ed.buffer().cursor().head, end_pos);

        // Move up visual lines
        ed.move_up(false);
        assert!(ed.buffer().cursor().head < end_pos);
        let head_prev = ed.buffer().cursor().head;

        ed.move_up(false);
        assert!(ed.buffer().cursor().head < head_prev);

        // Move home on this visual line
        ed.move_home(false);
        let cur_head = ed.buffer().cursor().head;
        // Moving down goes to next visual line
        ed.move_down(false);
        assert!(ed.buffer().cursor().head > cur_head);

        // Move end goes to visual line end
        ed.move_end(false);
        assert!(ed.buffer().cursor().head > cur_head);
    }
}
