//! Auto-width text table widget.
//!
//! [`VirtualTable`](crate::VirtualTable) renders a *typed* dataset through fixed-width
//! columns and row extractors — ideal for large scrolling grids. [`TextTable`] is its
//! lightweight complement: you hand it already-stringified header and body cells and it
//! sizes the columns to their content (shrinking the widest to fit the available width),
//! wraps overflowing cells across multiple lines, and draws box-drawing borders with proper
//! column rules. It is the right tool for rendering ad-hoc or Markdown (GFM) tables — e.g.
//! ones streamed from an LLM — where the columns aren't known ahead of time.

use tenui_core::{CanvasSubviewMut, Color, Modifier};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::{
    doc::parse_inline_markdown,
    rich::{RichSpan, wrap_rich_spans},
    theme::ThemePalette,
};

/// Box-drawing glyph set for a [`TextTable`], including the T-junctions and cross that
/// column rules need (which [`tenui_layout::BorderStyle`] does not carry).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TableBorder {
    /// Light square corners: `┌┬┐ ├┼┤ └┴┘`.
    Plain,
    /// Light rounded corners: `╭┬╮ ├┼┤ ╰┴╯`.
    Rounded,
    /// Double lines: `╔╦╗ ╠╬╣ ╚╩╝`.
    Double,
    /// Pure-ASCII fallback (`+ - |`) for terminals without box-drawing.
    Ascii,
}

/// The nine corner/junction glyphs plus the two edges, resolved from a [`TableBorder`].
struct Glyphs {
    tl: char,
    tm: char,
    tr: char, // top row
    ml: char,
    mm: char,
    mr: char, // header-rule row
    bl: char,
    bm: char,
    br: char, // bottom row
    h: char,
    v: char, // edges
}

impl TableBorder {
    fn glyphs(self) -> Glyphs {
        match self {
            TableBorder::Plain => Glyphs {
                tl: '┌',
                tm: '┬',
                tr: '┐',
                ml: '├',
                mm: '┼',
                mr: '┤',
                bl: '└',
                bm: '┴',
                br: '┘',
                h: '─',
                v: '│',
            },
            TableBorder::Rounded => Glyphs {
                tl: '╭',
                tm: '┬',
                tr: '╮',
                ml: '├',
                mm: '┼',
                mr: '┤',
                bl: '╰',
                bm: '┴',
                br: '╯',
                h: '─',
                v: '│',
            },
            TableBorder::Double => Glyphs {
                tl: '╔',
                tm: '╦',
                tr: '╗',
                ml: '╠',
                mm: '╬',
                mr: '╣',
                bl: '╚',
                bm: '╩',
                br: '╝',
                h: '═',
                v: '║',
            },
            TableBorder::Ascii => Glyphs {
                tl: '+',
                tm: '+',
                tr: '+',
                ml: '+',
                mm: '+',
                mr: '+',
                bl: '+',
                bm: '+',
                br: '+',
                h: '-',
                v: '|',
            },
        }
    }
}

/// Column text alignment for [`TextTable`].
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum TableAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// An auto-sizing, cell-wrapping, bordered text table.
///
/// ```
/// use tenui_std::{TextTable, TableBorder, theme::ThemePalette};
/// use tenui_core::{Buffer, Rect};
///
/// let table = TextTable::new(vec!["Name".into(), "Role".into()])
///     .row(vec!["Ada".into(), "Engineer".into()])
///     .row(vec!["Grace".into(), "Admiral".into()])
///     .border(TableBorder::Rounded);
///
/// let (w, h) = table.measure(40);
/// let mut buf = Buffer::new(w, h);
/// let mut sv = buf.subview_mut(Rect::new(0, 0, w, h));
/// let used = table.render(&mut sv, &ThemePalette::catppuccin_mocha());
/// assert_eq!(used, h);
/// ```
#[derive(Debug, Clone)]
pub struct TextTable {
    pub headers: Vec<String>,
    pub rows: Vec<Vec<String>>,
    pub border: TableBorder,
    /// Upper bound on any single column's content width before wrapping kicks in.
    pub max_col_width: u16,
    /// Draw a `├┼┤` rule between adjacent data rows. Off by default; turn it on when cells
    /// wrap across several lines and adjacent rows would otherwise merge into one wall of text.
    pub row_separator: bool,
    /// Per-column alignments (defaults to [`TableAlignment::Left`] for any column not specified).
    pub alignments: Vec<TableAlignment>,
}

impl TextTable {
    /// One space of padding on each side of a cell's content.
    const PAD: u16 = 2;

    /// Creates a table with the given header labels (may be empty for a header-less table).
    pub fn new(headers: Vec<String>) -> Self {
        Self {
            headers,
            rows: Vec::new(),
            border: TableBorder::Rounded,
            max_col_width: 40,
            row_separator: false,
            alignments: Vec::new(),
        }
    }

    /// Appends a data row (builder-style). Rows may be ragged; short rows are padded with
    /// empty cells and over-long rows widen the table.
    pub fn row(mut self, cells: Vec<String>) -> Self {
        self.rows.push(cells);
        self
    }

    /// Sets the border glyph style (default [`TableBorder::Rounded`]).
    pub fn border(mut self, border: TableBorder) -> Self {
        self.border = border;
        self
    }

    /// Sets the per-column content-width cap before wrapping (default 40).
    pub fn max_col_width(mut self, w: u16) -> Self {
        self.max_col_width = w.max(1);
        self
    }

    /// Enables (or disables) a `├┼┤` rule drawn between adjacent data rows (default off).
    pub fn row_separator(mut self, on: bool) -> Self {
        self.row_separator = on;
        self
    }

    /// Sets the column alignments (builder-style).
    pub fn alignments(mut self, alignments: Vec<TableAlignment>) -> Self {
        self.alignments = alignments;
        self
    }

    fn col_count(&self) -> usize {
        self.headers
            .len()
            .max(self.rows.iter().map(Vec::len).max().unwrap_or(0))
    }

    fn cell(row: &[String], i: usize) -> &str {
        row.get(i).map(String::as_str).unwrap_or("")
    }

    /// Resolves each column's content width: the natural width of its widest cell, capped by
    /// [`max_col_width`](Self::max_col_width), then shrunk (widest first) so the whole table
    /// fits `avail_w`. Never smaller than 1 per column.
    fn column_widths(&self, avail_w: u16) -> Vec<u16> {
        let n = self.col_count();
        if n == 0 {
            return Vec::new();
        }
        let dummy = ThemePalette::catppuccin_mocha();
        let mut widths = vec![1u16; n];
        for (i, wref) in widths.iter_mut().enumerate() {
            let mut w = Self::cell_natural_width(Self::cell(&self.headers, i), &dummy);
            for row in &self.rows {
                w = w.max(Self::cell_natural_width(Self::cell(row, i), &dummy));
            }
            *wref = w.max(1).min(self.max_col_width);
        }
        // Shrink the widest column repeatedly until the table fits the available width.
        let overhead = (n as u16 + 1) + Self::PAD * n as u16; // verticals + per-column padding
        let budget = avail_w.saturating_sub(overhead);
        let mut sum: u16 = widths.iter().sum();
        while sum > budget {
            match widths
                .iter()
                .enumerate()
                .filter(|(_, w)| **w > 1)
                .max_by_key(|(_, w)| **w)
            {
                Some((i, _)) => {
                    widths[i] -= 1;
                    sum -= 1;
                }
                None => break, // every column already at the 1-cell floor
            }
        }
        widths
    }

    /// Computes the visual display width of `text` after resolving HTML breaks and inline
    /// Markdown syntax (stripping delimiters and accounting for link/code formatting).
    fn cell_natural_width(text: &str, palette: &ThemePalette) -> u16 {
        let normalized = Self::normalize_breaks(text);
        let mut max_w = 0u16;
        for segment in normalized.split('\n') {
            let spans = parse_inline_markdown(segment, Color::Reset, Modifier::empty(), palette);
            let w: usize = spans.iter().map(|s| UnicodeWidthStr::width(s.text.as_str())).sum();
            max_w = max_w.max(w as u16);
        }
        max_w
    }

    /// Word-wraps a cell into styled [`RichSpan`] lines of at most `width` columns, first
    /// honoring explicit line breaks (`<br>`, `<br/>`, `\n`), then parsing inline Markdown
    /// syntax, and soft-wrapping styled tokens without severing formatting or delimiters.
    fn wrap_rich_cell(
        text: &str,
        width: u16,
        default_fg: Color,
        default_mod: Modifier,
        palette: &ThemePalette,
    ) -> Vec<Vec<RichSpan>> {
        let normalized = Self::normalize_breaks(text);
        let width = (width as usize).max(1);
        let mut lines: Vec<Vec<RichSpan>> = Vec::new();
        for segment in normalized.split('\n') {
            let spans = parse_inline_markdown(segment, default_fg, default_mod, palette);
            let seg_lines = wrap_rich_spans(&spans, width);
            lines.extend(seg_lines);
        }
        if lines.is_empty() {
            lines.push(Vec::new());
        }
        lines
    }

    /// Plain-text backward-compatible cell wrapping helper.
    pub fn wrap_cell(text: &str, width: u16) -> Vec<String> {
        let dummy = ThemePalette::catppuccin_mocha();
        let lines = Self::wrap_rich_cell(text, width, Color::Reset, Modifier::empty(), &dummy);
        lines
            .into_iter()
            .map(|row| row.into_iter().map(|s| s.text).collect::<String>())
            .collect()
    }

    /// Replaces HTML break tags (`<br>`, `<br/>`, `<br />`, case-insensitive) with newlines
    /// so they act as intra-cell line breaks rather than leaking as literal text.
    fn normalize_breaks(text: &str) -> String {
        if !text.contains('<') {
            return text.to_string();
        }
        let mut out = String::with_capacity(text.len());
        let bytes = text.as_bytes();
        let mut i = 0;
        while i < bytes.len() {
            if bytes[i] == b'<' {
                let rest = &text[i..];
                let lower = rest
                    .get(..rest.len().min(6))
                    .map(|s| s.to_ascii_lowercase())
                    .unwrap_or_default();
                if lower.starts_with("<br") {
                    // Match up to the closing '>' of a <br…> tag with no other content.
                    if let Some(close) = rest.find('>') {
                        let inner = rest[3..close].trim();
                        if inner.is_empty() || inner == "/" {
                            out.push('\n');
                            i += close + 1;
                            continue;
                        }
                    }
                }
            }
            let ch = text[i..].chars().next().unwrap();
            out.push(ch);
            i += ch.len_utf8();
        }
        out
    }

    /// Computes the `(width, height)` in cells the table occupies within `avail_w`.
    pub fn measure(&self, avail_w: u16) -> (u16, u16) {
        let widths = self.column_widths(avail_w);
        if widths.is_empty() {
            return (0, 0);
        }
        let table_w: u16 = widths.iter().map(|w| w + Self::PAD).sum::<u16>() + widths.len() as u16 + 1;
        let has_header = !self.headers.is_empty();
        let mut h: u16 = 2; // top + bottom border
        if has_header {
            h += 1 + 1; // one-line header (headers don't wrap) + header rule
        }
        let dummy = ThemePalette::catppuccin_mocha();
        for row in &self.rows {
            let row_h = (0..widths.len())
                .map(|i| {
                    Self::wrap_rich_cell(Self::cell(row, i), widths[i], Color::Reset, Modifier::empty(), &dummy).len()
                        as u16
                })
                .max()
                .unwrap_or(1)
                .max(1);
            h += row_h;
        }
        if self.row_separator {
            h += (self.rows.len().saturating_sub(1)) as u16; // one rule between each data-row pair
        }
        (table_w.min(avail_w.max(1)), h)
    }

    /// Renders the table into `sv` starting at its top-left, returning the number of rows
    /// (height in cells) drawn. Content is clipped to `sv`'s bounds.
    pub fn render(&self, sv: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) -> u16 {
        let avail_w = sv.width();
        let widths = self.column_widths(avail_w);
        if widths.is_empty() || sv.height() == 0 {
            return 0;
        }
        let g = self.border.glyphs();
        let border_fg = palette.border;
        let bg = palette.bg;
        let n = widths.len();

        // Interior span (including padding) of each column, used for horizontal rules.
        let spans: Vec<u16> = widths.iter().map(|w| w + Self::PAD).collect();
        let mut y = 0u16;

        let draw_rule = |sv: &mut CanvasSubviewMut<'_>, y: u16, left: char, mid: char, right: char| {
            let mut x = 0u16;
            sv.set_char(x, y, left, border_fg, bg, Modifier::empty());
            x += 1;
            for (i, span) in spans.iter().enumerate() {
                for _ in 0..*span {
                    sv.set_char(x, y, g.h, border_fg, bg, Modifier::empty());
                    x += 1;
                }
                let junction = if i + 1 == n { right } else { mid };
                sv.set_char(x, y, junction, border_fg, bg, Modifier::empty());
                x += 1;
            }
        };

        // top border
        draw_rule(sv, y, g.tl, g.tm, g.tr);
        y += 1;

        // header (single line; headers are not wrapped)
        if !self.headers.is_empty() {
            if y >= sv.height() {
                return y;
            }
            let header_spans: Vec<Vec<RichSpan>> = (0..n)
                .map(|i| parse_inline_markdown(Self::cell(&self.headers, i), palette.primary, Modifier::BOLD, palette))
                .collect();
            let col_refs: Vec<&[RichSpan]> = header_spans.iter().map(|v| v.as_slice()).collect();
            self.draw_cell_spans(sv, y, &widths, palette, &col_refs);
            y += 1;
            if y >= sv.height() {
                return y;
            }
            draw_rule(sv, y, g.ml, g.mm, g.mr);
            y += 1;
        }

        // data rows (wrapped), optionally separated by a `├┼┤` rule
        for (r, row) in self.rows.iter().enumerate() {
            if self.row_separator && r > 0 {
                if y >= sv.height() {
                    return y;
                }
                draw_rule(sv, y, g.ml, g.mm, g.mr);
                y += 1;
            }
            let wrapped: Vec<Vec<Vec<RichSpan>>> = (0..n)
                .map(|i| Self::wrap_rich_cell(Self::cell(row, i), widths[i], palette.fg, Modifier::empty(), palette))
                .collect();
            let row_h = wrapped.iter().map(Vec::len).max().unwrap_or(1).max(1);
            for line in 0..row_h {
                if y >= sv.height() {
                    return y;
                }
                let col_refs: Vec<&[RichSpan]> = (0..n)
                    .map(|i| wrapped[i].get(line).map(|v| v.as_slice()).unwrap_or(&[]))
                    .collect();
                self.draw_cell_spans(sv, y, &widths, palette, &col_refs);
                y += 1;
            }
        }

        // bottom border
        if y < sv.height() {
            draw_rule(sv, y, g.bl, g.bm, g.br);
            y += 1;
        }
        y
    }

    /// Draws one `│ c0 │ c1 │ …` cell line using styled [`RichSpan`] runs.
    fn draw_cell_spans(
        &self,
        sv: &mut CanvasSubviewMut<'_>,
        y: u16,
        widths: &[u16],
        palette: &ThemePalette,
        cols: &[&[RichSpan]],
    ) {
        let g = self.border.glyphs();
        let border_fg = palette.border;
        let bg = palette.bg;
        let mut x = 0u16;
        sv.set_char(x, y, g.v, border_fg, bg, Modifier::empty());
        x += 1;
        for (i, &w) in widths.iter().enumerate() {
            sv.set_char(x, y, ' ', palette.fg, bg, Modifier::empty());
            x += 1;
            let spans = cols.get(i).copied().unwrap_or(&[]);
            let align = self.alignments.get(i).copied().unwrap_or(TableAlignment::Left);

            // Compute total display width of spans (capped at w)
            let mut total_w = 0u16;
            for span in spans {
                for ch in span.text.chars() {
                    let cw = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    if total_w + cw > w {
                        break;
                    }
                    total_w += cw;
                }
                if total_w >= w {
                    break;
                }
            }

            let pad_left = match align {
                TableAlignment::Left => 0,
                TableAlignment::Right => w.saturating_sub(total_w),
                TableAlignment::Center => w.saturating_sub(total_w) / 2,
            };
            let pad_right = w.saturating_sub(total_w + pad_left);

            for _ in 0..pad_left {
                sv.set_char(x, y, ' ', palette.fg, bg, Modifier::empty());
                x += 1;
            }

            let mut drawn = 0u16;
            for span in spans {
                for ch in span.text.chars() {
                    let cw = UnicodeWidthChar::width(ch).unwrap_or(1) as u16;
                    if drawn + cw > total_w {
                        break;
                    }
                    sv.set_char(x, y, ch, span.fg, bg, span.modifier);
                    x += cw;
                    drawn += cw;
                }
                if drawn >= total_w {
                    break;
                }
            }

            for _ in 0..pad_right {
                sv.set_char(x, y, ' ', palette.fg, bg, Modifier::empty());
                x += 1;
            }

            sv.set_char(x, y, ' ', palette.fg, bg, Modifier::empty());
            x += 1;
            sv.set_char(x, y, g.v, border_fg, bg, Modifier::empty());
            x += 1;
        }
    }

    /// Parses a GitHub-Flavored-Markdown table block (header row, `|---|` delimiter row,
    /// then data rows) into a [`TextTable`]. Returns `None` when `block` is not a valid GFM
    /// table (missing/!matching delimiter row).
    pub fn parse_gfm(block: &str) -> Option<TextTable> {
        let lines: Vec<&str> = block.lines().map(str::trim).filter(|l| !l.is_empty()).collect();
        if lines.len() < 2 {
            return None;
        }
        // Split on unescaped `|` only, honoring GFM's `\|` escape (a literal pipe inside a
        // cell, e.g. in a code span or formula). Without this a stray pipe would spawn phantom
        // columns and crush the real ones.
        let split = |line: &str| -> Vec<String> {
            let t = line.trim();
            let t = t.strip_prefix('|').unwrap_or(t);
            let t = t.strip_suffix('|').unwrap_or(t);
            let mut cells: Vec<String> = Vec::new();
            let mut cur = String::new();
            let mut chars = t.chars().peekable();
            while let Some(ch) = chars.next() {
                match ch {
                    '\\' if chars.peek() == Some(&'|') => {
                        chars.next();
                        cur.push('|');
                    }
                    '|' => cells.push(std::mem::take(&mut cur)),
                    _ => cur.push(ch),
                }
            }
            cells.push(cur);
            cells.iter().map(|c| c.trim().to_string()).collect()
        };
        // The 2nd line must be a delimiter row: each cell like ---, :---, ---:, :--:.
        let delim_cells = split(lines[1]);
        let is_delim = lines[1].contains('-')
            && delim_cells
                .iter()
                .all(|c| !c.is_empty() && c.chars().all(|ch| ch == '-' || ch == ':') && c.contains('-'));
        if !is_delim {
            return None;
        }
        let alignments: Vec<TableAlignment> = delim_cells
            .iter()
            .map(|c| {
                let has_left = c.starts_with(':');
                let has_right = c.ends_with(':');
                match (has_left, has_right) {
                    (true, true) => TableAlignment::Center,
                    (false, true) => TableAlignment::Right,
                    _ => TableAlignment::Left,
                }
            })
            .collect();
        let headers = split(lines[0]);
        let n_cols = headers.len();
        let mut table = TextTable::new(headers).alignments(alignments);
        for line in &lines[2..] {
            // Clamp each data row to the header column count per the GFM spec: extra cells are
            // dropped, missing cells are padded empty. Guards against any residual over-split.
            let mut cells = split(line);
            cells.resize(n_cols, String::new());
            table.rows.push(cells);
        }
        Some(table)
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    fn row_text(sv: &CanvasSubviewMut<'_>, y: u16, w: u16) -> String {
        (0..w)
            .map(|x| sv.get(x, y).map(|c| c.symbol.as_str().to_string()).unwrap_or_default())
            .collect()
    }

    #[test]
    fn test_measure_and_render_basic() {
        let t = TextTable::new(vec!["Name".into(), "Role".into()])
            .row(vec!["Ada".into(), "Engineer".into()])
            .border(TableBorder::Plain);
        let (w, h) = t.measure(40);
        // 2 cols: widths Name/Ada->4, Role/Engineer->8 => 4+2 + 8+2 + 3 borders = 19
        assert_eq!(w, 19);
        assert_eq!(h, 5); // top + header + rule + 1 data row + bottom
        let mut buf = Buffer::new(w, h);
        let mut sv = buf.subview_mut(Rect::new(0, 0, w, h));
        let used = t.render(&mut sv, &ThemePalette::catppuccin_mocha());
        assert_eq!(used, h);
        assert!(row_text(&sv, 0, w).starts_with('┌'));
        assert!(row_text(&sv, 1, w).contains("Name"));
        assert!(row_text(&sv, 1, w).contains("Role"));
        assert!(row_text(&sv, 3, w).contains("Engineer"));
        assert!(row_text(&sv, 4, w).starts_with('└'));
    }

    #[test]
    fn test_wrap_shrinks_and_wraps_wide_cell() {
        let t = TextTable::new(vec!["K".into(), "V".into()]).row(vec!["a".into(), "one two three four five".into()]);
        // Narrow avail forces the wide 2nd column to shrink and wrap onto multiple lines.
        let (w, h) = t.measure(16);
        assert!(w <= 16);
        assert!(
            h > 5,
            "wide cell should wrap to more than a single data line, got h={h}"
        );
    }

    #[test]
    fn test_parse_gfm_roundtrip() {
        let block = "| Lane | Latency |\n|------|--------:|\n| Layout | 120us |\n| Paint | 1.2ms |";
        let t = TextTable::parse_gfm(block).expect("valid GFM table");
        assert_eq!(t.headers, vec!["Lane".to_string(), "Latency".to_string()]);
        assert_eq!(t.rows.len(), 2);
        assert_eq!(t.rows[1], vec!["Paint".to_string(), "1.2ms".to_string()]);

        // Not a table: no delimiter row.
        assert!(TextTable::parse_gfm("just some\nprose here").is_none());
    }

    #[test]
    fn test_row_separator_adds_rules_between_rows() {
        let plain = TextTable::new(vec!["A".into()])
            .row(vec!["x".into()])
            .row(vec!["y".into()]);
        let sep = plain.clone().row_separator(true);
        let (_, h_plain) = plain.measure(20);
        let (w, h_sep) = sep.measure(20);
        assert_eq!(h_sep, h_plain + 1, "one rule between the two data rows");

        let mut buf = Buffer::new(w, h_sep);
        let mut sv = buf.subview_mut(Rect::new(0, 0, w, h_sep));
        sep.render(&mut sv, &ThemePalette::catppuccin_mocha());
        // Row: top, header, header-rule, x, ├──┤ separator, y, bottom.
        assert!(row_text(&sv, 4, w).starts_with('├'), "separator rule between data rows");
    }

    #[test]
    fn test_br_tag_becomes_line_break() {
        let lines = TextTable::wrap_cell("line one<br>line two<br/>three", 40);
        assert_eq!(lines, vec!["line one", "line two", "three"]);
    }

    #[test]
    fn test_gfm_escaped_pipe_and_column_clamp() {
        // A `\|` inside a cell must stay one cell; a stray unescaped `|` overflow is clamped.
        let block = "| Op | Meaning |\n|----|---------|\n| `a \\| b` | bitwise or |\n| x | y | z |";
        let t = TextTable::parse_gfm(block).expect("valid GFM table");
        assert_eq!(t.headers.len(), 2);
        assert_eq!(t.rows[0], vec!["`a | b`".to_string(), "bitwise or".to_string()]);
        // The 3-cell row is clamped back down to 2 columns.
        assert_eq!(t.rows[1].len(), 2);
        assert_eq!(t.rows[1], vec!["x".to_string(), "y".to_string()]);
    }

    #[test]
    fn test_inline_markdown_rendering_and_styling() {
        let palette = ThemePalette::catppuccin_mocha();
        let t = TextTable::new(vec!["Header".into()])
            .row(vec!["**Bold** and `code`".into()])
            .border(TableBorder::Plain);
        let (w, h) = t.measure(40);
        let mut buf = Buffer::new(w, h);
        let mut sv = buf.subview_mut(Rect::new(0, 0, w, h));
        t.render(&mut sv, &palette);

        let row1 = row_text(&sv, 3, w);
        // Markdown asterisks and backticks must be stripped from the rendered view.
        assert!(
            !row1.contains("**"),
            "raw markdown asterisks should not appear in rendered cells"
        );
        assert!(
            !row1.contains('`'),
            "raw markdown backticks should not appear in rendered cells"
        );
        assert!(row1.contains("Bold and code"));

        // Verify cell modifiers and colors
        // Left border is at x=0, pad space at x=1, "Bold" starts at x=2.
        let cell_b = sv.get(2, 3).expect("cell for 'B'");
        assert_eq!(cell_b.symbol.as_str(), "B");
        assert_eq!(cell_b.modifier, Modifier::BOLD);

        // "Bold and " has 9 chars (x=2..10). 'c' in "code" starts at x=11.
        let cell_c = sv.get(11, 3).expect("cell for 'c'");
        assert_eq!(cell_c.symbol.as_str(), "c");
        assert_eq!(cell_c.fg, palette.accent);
    }

    #[test]
    fn test_inline_markdown_column_sizing_does_not_bloat() {
        // Raw "**10**" is 6 chars, but rendered visual width is 2.
        // With 1 padding on each side and 2 borders: 2 + 2 (pad) + 2 (border) = 6.
        let t = TextTable::new(vec!["D".into()])
            .row(vec!["**10**".into()])
            .border(TableBorder::Plain);
        let (w, _) = t.measure(40);
        // Column width should be 2 (for "10"), table width = 2 + 2 + 2 = 6.
        assert_eq!(
            w, 6,
            "column width should size to stripped text '10' (2), not raw '**10**' (6)"
        );
    }

    #[test]
    fn test_inline_markdown_wrapping_preserves_style() {
        let palette = ThemePalette::catppuccin_mocha();
        // Force wrap at width 5: "**Basic rules**" -> "Basic" and "rules"
        let wrapped = TextTable::wrap_rich_cell("**Basic rules**", 5, palette.fg, Modifier::empty(), &palette);
        assert_eq!(wrapped.len(), 2, "should wrap into 2 lines");
        assert_eq!(wrapped[0][0].text, "Basic");
        assert_eq!(wrapped[0][0].modifier, Modifier::BOLD);
        assert_eq!(wrapped[1][0].text, "rules");
        assert_eq!(wrapped[1][0].modifier, Modifier::BOLD);
    }

    #[test]
    fn test_column_alignments_left_center_right() {
        let block = "| Left | Center | Right |\n|:---|:---:|---:|\n| a | b | c |";
        let table = TextTable::parse_gfm(block).expect("valid GFM table");
        assert_eq!(
            table.alignments,
            vec![TableAlignment::Left, TableAlignment::Center, TableAlignment::Right]
        );

        let (w, h) = table.measure(40);
        let mut buf = Buffer::new(w, h);
        let mut sv = buf.subview_mut(Rect::new(0, 0, w, h));
        table.render(&mut sv, &ThemePalette::catppuccin_mocha());

        let data_row = row_text(&sv, 3, w);
        assert!(data_row.contains('a'));
        assert!(data_row.contains('b'));
        assert!(data_row.contains('c'));
    }
}
