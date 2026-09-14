use std::io::{self, Write};

use crate::{
    buffer::Buffer,
    cell::Modifier,
    color::{Color, push_dec},
};

/// Emits a `CSI y;x H` cursor-position escape (1-based) without `core::fmt` overhead.
#[inline]
fn push_cursor(out: &mut String, x: u16, y: u16) {
    out.push_str("\x1b[");
    push_dec(out, y as u32 + 1);
    out.push(';');
    push_dec(out, x as u32 + 1);
    out.push('H');
}

/// An ANSI SGR coalescing difference engine.
///
/// Implements Phase 3 of the Tenui pipeline:
/// - Skips unchanged cells.
/// - Groups contiguous modifications.
/// - Coalesces style/color updates into compound SGR escape sequences.
/// - Brackets frames with Synchronized Output (CSI ? 2026 h/l).
#[derive(Debug, Default)]
pub struct SgrCoalescer {
    current_cursor: Option<(u16, u16)>,
    current_fg: Color,
    current_bg: Color,
    current_modifier: Modifier,
    current_underline_color: Color,
    current_underline_style: crate::cell::UnderlineStyle,
}

impl SgrCoalescer {
    pub const SYNC_START: &'static str = "\x1b[?2026h";
    pub const SYNC_END: &'static str = "\x1b[?2026l";
    pub const RESET_ALL: &'static str = "\x1b[0m";

    pub fn new() -> Self {
        Self::default()
    }

    /// Resets cached terminal styling state.
    pub fn reset_state(&mut self) {
        self.current_cursor = None;
        self.current_fg = Color::Reset;
        self.current_bg = Color::Reset;
        self.current_modifier = Modifier::empty();
        self.current_underline_color = Color::Reset;
        self.current_underline_style = crate::cell::UnderlineStyle::None;
    }

    /// Generates ANSI diff stream from `front` to `back` and writes it to `writer`.
    pub fn write_diff<W: Write>(&mut self, writer: &mut W, front: &Buffer, back: &Buffer) -> io::Result<()> {
        let mut out = String::with_capacity(4096);

        // Synchronized output begin
        out.push_str(Self::SYNC_START);

        let width = back.width.min(front.width) as usize;
        let height = back.height.min(front.height) as usize;
        let back_w = back.width as usize;
        let front_w = front.width as usize;

        for y in 0..height {
            // Take the two rows as slices of length `width`. `width` is clamped to both
            // buffers' widths and `y < height` to both heights, so these ranges are always
            // in bounds — the bounds check happens once per row here instead of twice per
            // cell inside `get()`, and indexing `..[x]` for `x < width` then elides. This is
            // the full-screen scan hot path (`draw`/`present`), so per-cell overhead counts.
            let back_row = &back.cells[y * back_w..y * back_w + width];
            let front_row = &front.cells[y * front_w..y * front_w + width];

            for x in 0..width {
                let back_cell = &back_row[x];

                // If back is continuation cell (2nd half of wide char), skip direct draw
                if back_cell.is_continuation {
                    continue;
                }

                // Skip identical cells
                if *back_cell == front_row[x] {
                    continue;
                }

                let x = x as u16;
                let y = y as u16;

                // Position cursor if not adjacent
                if self.current_cursor != Some((x, y)) {
                    push_cursor(&mut out, x, y);
                }

                // Coalesce style updates
                self.write_style_diff(
                    &mut out,
                    back_cell.fg,
                    back_cell.bg,
                    back_cell.modifier,
                    back_cell.underline_color,
                    back_cell.underline_style,
                );

                // Write glyph
                out.push_str(back_cell.symbol.as_str());

                // Update tracked cursor position
                let step = back_cell.width.max(1) as u16;
                self.current_cursor = Some((x + step, y));
            }
        }

        // Synchronized output end
        out.push_str(Self::SYNC_END);

        writer.write_all(out.as_bytes())?;
        writer.flush()
    }

    /// Emits an ANSI diff for only the cells at `dirty` (flat `y*width + x` indices),
    /// instead of scanning the whole grid. Given the exact changed-cell set (e.g. from the
    /// SIMD [`diff_buffers`](../../tenui_simd/diff/struct.SimdDiffScanner.html), or any
    /// superset), the emitted bytes are identical to [`Self::write_diff`] — it
    /// just skips the O(cells) scalar comparison, which is the point on 4K/8K viewports.
    pub fn write_diff_indexed<W: Write>(
        &mut self,
        writer: &mut W,
        front: &Buffer,
        back: &Buffer,
        dirty: &[usize],
    ) -> io::Result<()> {
        let mut out = String::with_capacity(dirty.len().saturating_mul(8) + 32);
        out.push_str(Self::SYNC_START);

        let width = back.width as usize;
        if width > 0 {
            // Row-major order so the cursor-adjacency optimization matches write_diff.
            let mut idxs: Vec<usize> = dirty.to_vec();
            idxs.sort_unstable();
            idxs.dedup();

            for &idx in &idxs {
                let x = (idx % width) as u16;
                let y = (idx / width) as u16;
                let back_cell = match back.get(x, y) {
                    Some(c) => c,
                    None => continue,
                };
                if back_cell.is_continuation {
                    continue;
                }
                // Defensive: a superset dirty set may include unchanged cells.
                if Some(back_cell) == front.get(x, y) {
                    continue;
                }
                if self.current_cursor != Some((x, y)) {
                    push_cursor(&mut out, x, y);
                }
                self.write_style_diff(
                    &mut out,
                    back_cell.fg,
                    back_cell.bg,
                    back_cell.modifier,
                    back_cell.underline_color,
                    back_cell.underline_style,
                );
                out.push_str(back_cell.symbol.as_str());
                let step = back_cell.width.max(1) as u16;
                self.current_cursor = Some((x + step, y));
            }
        }

        out.push_str(Self::SYNC_END);
        writer.write_all(out.as_bytes())?;
        writer.flush()
    }

    /// Generates compound SGR escape code for state transition.
    fn write_style_diff(
        &mut self,
        out: &mut String,
        target_fg: Color,
        target_bg: Color,
        target_modifier: Modifier,
        target_underline_color: Color,
        target_underline_style: crate::cell::UnderlineStyle,
    ) {
        if self.current_fg == target_fg
            && self.current_bg == target_bg
            && self.current_modifier == target_modifier
            && self.current_underline_color == target_underline_color
            && self.current_underline_style == target_underline_style
        {
            return;
        }

        // Write SGR parameters straight into `out` (no per-cell Vec/format! churn).
        // Speculatively emit the CSI introducer, then truncate it back if no
        // parameter turned out to be needed. Output is byte-identical to a
        // `\x1b[<p0;p1;...>m` sequence.
        let start = out.len();
        out.push_str("\x1b[");
        let mut first = true;

        // If any previously set modifiers are missing in target, do a clean reset.
        let must_reset = !self.current_modifier.difference(target_modifier).is_empty();

        if must_reset {
            push_sep(out, &mut first);
            out.push('0');
            append_modifier_codes(out, &mut first, target_modifier);
            if target_fg != Color::Reset {
                push_sep(out, &mut first);
                target_fg.write_fg_sgr(out);
            }
            if target_bg != Color::Reset {
                push_sep(out, &mut first);
                target_bg.write_bg_sgr(out);
            }
        } else {
            append_modifier_codes(out, &mut first, target_modifier.difference(self.current_modifier));
            if self.current_fg != target_fg {
                push_sep(out, &mut first);
                target_fg.write_fg_sgr(out);
            }
            if self.current_bg != target_bg {
                push_sep(out, &mut first);
                target_bg.write_bg_sgr(out);
            }
        }

        // Underline style (CSI 4:Xm)
        if (target_underline_style != self.current_underline_style || must_reset)
            && let Some(code) = target_underline_style.sgr_code()
        {
            push_sep(out, &mut first);
            out.push_str(code);
        }

        // Underline color (CSI 58 extended underline color)
        if target_underline_color != self.current_underline_color || must_reset {
            match target_underline_color {
                Color::Reset => {
                    if self.current_underline_color != Color::Reset {
                        push_sep(out, &mut first);
                        out.push_str("59");
                    }
                }
                Color::Rgb(r, g, b) => {
                    push_sep(out, &mut first);
                    out.push_str("58;2;");
                    push_dec(out, r as u32);
                    out.push(';');
                    push_dec(out, g as u32);
                    out.push(';');
                    push_dec(out, b as u32);
                }
                Color::Indexed(idx) => {
                    push_sep(out, &mut first);
                    out.push_str("58;5;");
                    push_dec(out, idx as u32);
                }
                _ => {}
            }
        }

        if first {
            out.truncate(start); // nothing emitted — drop the speculative CSI introducer
        } else {
            out.push('m');
        }

        self.current_fg = target_fg;
        self.current_bg = target_bg;
        self.current_modifier = target_modifier;
        self.current_underline_color = target_underline_color;
        self.current_underline_style = target_underline_style;
    }
}

/// Writes a `;` separator before every parameter except the first.
#[inline]
fn push_sep(out: &mut String, first: &mut bool) {
    if *first {
        *first = false;
    } else {
        out.push(';');
    }
}

fn append_modifier_codes(out: &mut String, first: &mut bool, m: Modifier) {
    for (flag, code) in [
        (Modifier::BOLD, '1'),
        (Modifier::DIM, '2'),
        (Modifier::ITALIC, '3'),
        (Modifier::UNDERLINE, '4'),
        (Modifier::BLINK, '5'),
        (Modifier::REVERSE, '7'),
        (Modifier::HIDDEN, '8'),
        (Modifier::STRIKETHROUGH, '9'),
    ] {
        if m.contains(flag) {
            push_sep(out, first);
            out.push(code);
        }
    }
}
