use tenui_core::{Modifier, buffer::CanvasSubviewMut, color::Color};

/// Mathematical AST node representing sub-cell mathematical expressions based on the TeX box model.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MathNode {
    Symbol(char),
    Text(String),
    Fraction(Box<MathNode>, Box<MathNode>),
    SuperscriptSubscript {
        base: Box<MathNode>,
        sub: Option<Box<MathNode>>,
        sup: Option<Box<MathNode>>,
    },
    Integral {
        lower: Option<Box<MathNode>>,
        upper: Option<Box<MathNode>>,
        integrand: Box<MathNode>,
    },
    Matrix {
        rows: Vec<Vec<MathNode>>,
    },
    Row(Vec<MathNode>),
}

impl MathNode {
    pub fn symbol(c: char) -> Self {
        MathNode::Symbol(c)
    }

    pub fn text(s: impl Into<String>) -> Self {
        MathNode::Text(s.into())
    }

    pub fn fraction(num: MathNode, den: MathNode) -> Self {
        MathNode::Fraction(Box::new(num), Box::new(den))
    }

    pub fn supsub(base: MathNode, sup: Option<MathNode>, sub: Option<MathNode>) -> Self {
        MathNode::SuperscriptSubscript {
            base: Box::new(base),
            sub: sub.map(Box::new),
            sup: sup.map(Box::new),
        }
    }

    pub fn integral(lower: Option<MathNode>, upper: Option<MathNode>, integrand: MathNode) -> Self {
        MathNode::Integral {
            lower: lower.map(Box::new),
            upper: upper.map(Box::new),
            integrand: Box::new(integrand),
        }
    }

    pub fn matrix(rows: Vec<Vec<MathNode>>) -> Self {
        MathNode::Matrix { rows }
    }

    pub fn row(items: Vec<MathNode>) -> Self {
        MathNode::Row(items)
    }

    /// Parses a LaTeX mathematical expression into a `MathNode` AST.
    pub fn from_latex(input: &str) -> Self {
        crate::parse::parse_latex(input)
    }
}

/// TeX-inspired mathematical box layout typesetter for monospaced terminal grids.
pub struct MathTypesetter;

impl MathTypesetter {
    /// Measures the dimensions (width, height) required for a single-level fraction.
    pub fn measure_fraction(num_w: u16, den_w: u16) -> (u16, u16) {
        let width = num_w.max(den_w) + 2;
        let height = 3; // 1 row numerator, 1 bar, 1 row denominator
        (width, height)
    }

    /// Renders a simple text-based fraction with horizontal fraction bar '─', filling glyph
    /// cells with background `bg` (use [`Color::TRANSPARENT`] to leave the canvas showing).
    pub fn render_fraction(
        surface: &mut CanvasSubviewMut,
        x: u16,
        y: u16,
        num_str: &str,
        den_str: &str,
        color: Color,
        bg: Color,
    ) {
        let num_w = num_str.chars().count() as u16;
        let den_w = den_str.chars().count() as u16;
        let bar_w = num_w.max(den_w) + 2;

        let num_offset = (bar_w.saturating_sub(num_w)) / 2;
        let den_offset = (bar_w.saturating_sub(den_w)) / 2;

        // Numerator
        surface.write_str_clipped(x + num_offset, y, num_str, color, bg);
        // Fraction Bar
        for i in 0..bar_w {
            surface.set_char(x + i, y + 1, '─', color, bg, Modifier::empty());
        }
        // Denominator
        surface.write_str_clipped(x + den_offset, y + 2, den_str, color, bg);
    }

    /// Renders an integral sign with optional top and bottom glyphs ('⌠', '⌡', '│'), filling
    /// glyph cells with background `bg`.
    pub fn render_integral(surface: &mut CanvasSubviewMut, x: u16, y: u16, height: u16, color: Color, bg: Color) {
        if height <= 1 {
            surface.set_char(x, y, '∫', color, bg, Modifier::empty());
            return;
        }

        // Top arc
        surface.set_char(x, y, '⌠', color, bg, Modifier::empty());
        // Middle extension bars
        for r in 1..height.saturating_sub(1) {
            surface.set_char(x, y + r, '│', color, bg, Modifier::empty());
        }
        // Bottom arc
        surface.set_char(x, y + height - 1, '⌡', color, bg, Modifier::empty());
    }

    /// Measures the bounding box (width, height) of any arbitrary `MathNode`.
    pub fn measure(&self, node: &MathNode) -> (u16, u16) {
        match node {
            MathNode::Symbol(_) => (1, 1),
            MathNode::Text(s) => (s.chars().count() as u16, 1),
            MathNode::Fraction(num, den) => {
                let (nw, nh) = self.measure(num);
                let (dw, dh) = self.measure(den);
                let width = nw.max(dw) + 2;
                let height = nh + 1 + dh;
                (width, height)
            }
            MathNode::SuperscriptSubscript { base, sub, sup } => {
                let (bw, bh) = self.measure(base);
                let (sup_w, sup_h) = sup.as_ref().map(|s| self.measure(s)).unwrap_or((0, 0));
                let (sub_w, sub_h) = sub.as_ref().map(|s| self.measure(s)).unwrap_or((0, 0));
                let width = bw + sup_w.max(sub_w);
                let height = bh.max(sup_h + sub_h).max(1);
                (width, height)
            }
            MathNode::Integral {
                lower,
                upper,
                integrand,
            } => {
                let (iw, ih) = self.measure(integrand);
                let (lw, _) = lower.as_ref().map(|l| self.measure(l)).unwrap_or((0, 0));
                let (uw, _) = upper.as_ref().map(|u| self.measure(u)).unwrap_or((0, 0));
                let has_limits = lower.is_some() || upper.is_some();
                let int_w = if has_limits { 1 + lw.max(uw) } else { 1 };
                let height = if has_limits { ih.max(3) } else { ih.max(1) };
                (int_w + 1 + iw, height)
            }
            MathNode::Matrix { rows } => {
                if rows.is_empty() {
                    return (2, 1);
                }
                let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let mut col_widths = vec![0u16; num_cols];
                let mut total_height = 0u16;

                for row in rows {
                    let mut row_h = 1u16;
                    for (c_idx, cell) in row.iter().enumerate() {
                        let (w, h) = self.measure(cell);
                        col_widths[c_idx] = col_widths[c_idx].max(w);
                        row_h = row_h.max(h);
                    }
                    total_height += row_h;
                }

                // 2 brackets + column widths + spaces between columns
                let total_width = 2 + col_widths.iter().sum::<u16>() + num_cols.saturating_sub(1) as u16;
                (total_width, total_height)
            }
            MathNode::Row(nodes) => {
                let mut total_w = 0;
                let mut max_h = 1;
                for n in nodes {
                    let (w, h) = self.measure(n);
                    total_w += w;
                    max_h = max_h.max(h);
                }
                (total_w, max_h)
            }
        }
    }

    /// Renders a `MathNode` hierarchically into the canvas subview at (x, y) with a transparent
    /// background (the canvas shows through the glyph cells).
    ///
    /// This is a convenience wrapper over [`render_themed`](Self::render_themed); prefer that
    /// when drawing onto a themed surface so the formula's cell backgrounds match the panel and
    /// don't paint as an opaque black box on a non-black theme.
    pub fn render(&self, surface: &mut CanvasSubviewMut, x: u16, y: u16, node: &MathNode, color: Color) {
        self.render_themed(surface, x, y, node, color, Color::TRANSPARENT);
    }

    /// Renders a `MathNode` with an explicit foreground `fg` and background `bg` for every glyph
    /// cell. Pass the surrounding panel's background as `bg` (e.g. `palette.bg`) so the typeset
    /// formula blends into a Catppuccin/Gruvbox/Solarized surface instead of rendering as a
    /// black rectangle — the bug that `bg: Color::TRANSPARENT` on a `Color::Reset` buffer
    /// produces on non-black themes.
    pub fn render_themed(&self, surface: &mut CanvasSubviewMut, x: u16, y: u16, node: &MathNode, fg: Color, bg: Color) {
        match node {
            MathNode::Symbol(c) => {
                surface.set_char(x, y, *c, fg, bg, Modifier::empty());
            }
            MathNode::Text(s) => {
                surface.write_str_clipped(x, y, s, fg, bg);
            }
            MathNode::Fraction(num, den) => {
                let (nw, nh) = self.measure(num);
                let (dw, _dh) = self.measure(den);
                let bar_w = nw.max(dw) + 2;

                let num_offset = (bar_w.saturating_sub(nw)) / 2;
                let den_offset = (bar_w.saturating_sub(dw)) / 2;

                self.render_themed(surface, x + num_offset, y, num, fg, bg);
                for i in 0..bar_w {
                    surface.set_char(x + i, y + nh, '─', fg, bg, Modifier::empty());
                }
                self.render_themed(surface, x + den_offset, y + nh + 1, den, fg, bg);
            }
            MathNode::SuperscriptSubscript { base, sub, sup } => {
                let (bw, bh) = self.measure(base);
                self.render_themed(surface, x, y, base, fg, bg);

                if let Some(sup_node) = sup {
                    self.render_themed(surface, x + bw, y, sup_node, fg, bg);
                }
                if let Some(sub_node) = sub {
                    let sub_y = if bh > 1 { y + bh - 1 } else { y + 1 };
                    self.render_themed(surface, x + bw, sub_y, sub_node, fg, bg);
                }
            }
            MathNode::Integral {
                lower,
                upper,
                integrand,
            } => {
                let (_, ih) = self.measure(integrand);
                let has_limits = lower.is_some() || upper.is_some();
                let height = if has_limits { ih.max(3) } else { ih.max(1) };
                Self::render_integral(surface, x, y, height, fg, bg);

                let mut offset_x = x + 1;
                if let Some(up) = upper {
                    self.render_themed(surface, x + 1, y, up, fg, bg);
                    let (uw, _) = self.measure(up);
                    offset_x = offset_x.max(x + 1 + uw);
                }
                if let Some(low) = lower {
                    self.render_themed(surface, x + 1, y + height - 1, low, fg, bg);
                    let (lw, _) = self.measure(low);
                    offset_x = offset_x.max(x + 1 + lw);
                }

                let integrand_y = if has_limits {
                    y + (height.saturating_sub(ih)) / 2
                } else {
                    y
                };
                self.render_themed(surface, offset_x + 1, integrand_y, integrand, fg, bg);
            }
            MathNode::Matrix { rows } => {
                if rows.is_empty() {
                    return;
                }
                let (mat_w, mat_h) = self.measure(node);

                // Draw brackets '[' and ']'
                for row_y in 0..mat_h {
                    surface.set_char(x, y + row_y, '│', fg, bg, Modifier::empty());
                    surface.set_char(x + mat_w - 1, y + row_y, '│', fg, bg, Modifier::empty());
                }
                surface.set_char(x, y, '⎡', fg, bg, Modifier::empty());
                surface.set_char(x, y + mat_h - 1, '⎣', fg, bg, Modifier::empty());
                surface.set_char(x + mat_w - 1, y, '⎤', fg, bg, Modifier::empty());
                surface.set_char(x + mat_w - 1, y + mat_h - 1, '⎦', fg, bg, Modifier::empty());

                let num_cols = rows.iter().map(|r| r.len()).max().unwrap_or(0);
                let mut col_widths = vec![0u16; num_cols];
                for r in rows {
                    for (c_idx, cell) in r.iter().enumerate() {
                        let (w, _) = self.measure(cell);
                        col_widths[c_idx] = col_widths[c_idx].max(w);
                    }
                }

                let mut curr_y = y;
                for row in rows {
                    let mut curr_x = x + 1;
                    let mut row_h = 1u16;
                    for (c_idx, cell) in row.iter().enumerate() {
                        let (_, h) = self.measure(cell);
                        row_h = row_h.max(h);
                        self.render_themed(surface, curr_x, curr_y, cell, fg, bg);
                        curr_x += col_widths[c_idx] + 1;
                    }
                    curr_y += row_h;
                }
            }
            MathNode::Row(nodes) => {
                let (_, max_h) = self.measure(node);
                let mut curr_x = x;
                for n in nodes {
                    let (w, h) = self.measure(n);
                    let offset_y = (max_h.saturating_sub(h)) / 2;
                    self.render_themed(surface, curr_x, y + offset_y, n, fg, bg);
                    curr_x += w;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn test_fraction_rendering() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 5));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 10, 5));

        MathTypesetter::render_fraction(&mut subview, 0, 0, "sin(x)", "x", Color::White, Color::TRANSPARENT);

        // Numerator at y=0, bar at y=1, denominator at y=2
        // bar width is max(6, 1) + 2 = 8
        let bar_cell = subview.get(0, 1).expect("bar cell exists");
        assert_eq!(bar_cell.symbol.as_str(), "─");

        let den_cell = subview.get(3, 2).expect("den cell exists");
        assert_eq!(den_cell.symbol.as_str(), "x");
    }

    #[test]
    fn test_math_ast_integral() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 6));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 20, 6));

        let expr = MathNode::integral(
            Some(MathNode::symbol('0')),
            Some(MathNode::symbol('∞')),
            MathNode::fraction(MathNode::text("sin(x)"), MathNode::text("x")),
        );

        let typesetter = MathTypesetter;
        let (w, h) = typesetter.measure(&expr);
        assert!(w > 5);
        assert!(h >= 3);

        typesetter.render(&mut subview, 0, 0, &expr, Color::Cyan);

        // Verify integral top and bottom symbols
        let top_cell = subview.get(0, 0).expect("top cell");
        assert_eq!(top_cell.symbol.as_str(), "⌠");

        let bot_cell = subview.get(0, h - 1).expect("bot cell");
        assert_eq!(bot_cell.symbol.as_str(), "⌡");
    }

    #[test]
    fn test_math_ast_indefinite_integral() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 20, 3));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 20, 3));

        let expr = MathNode::integral(None, None, MathNode::text("x dx"));

        let typesetter = MathTypesetter;
        let (w, h) = typesetter.measure(&expr);
        assert_eq!(h, 1);
        assert!(w >= 5);

        typesetter.render(&mut subview, 0, 0, &expr, Color::Cyan);

        let int_cell = subview.get(0, 0).expect("integral cell");
        assert_eq!(int_cell.symbol.as_str(), "∫");
    }

    #[test]
    fn test_render_themed_applies_background() {
        let mut buffer = Buffer::empty(Rect::new(0, 0, 10, 3));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, 10, 3));
        let bg = Color::Rgb(30, 30, 46);

        let expr = MathNode::text("E=mc");
        MathTypesetter.render_themed(&mut subview, 0, 0, &expr, Color::White, bg);

        // Glyph cells carry the themed background instead of a transparent (black-box) one.
        let cell = subview.get(0, 0).expect("glyph cell");
        assert_eq!(cell.bg, bg);
        assert_eq!(cell.fg, Color::White);
    }
}
