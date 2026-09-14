use tenui_core::{CanvasSubviewMut, Color, Modifier};

use crate::theme::ThemePalette;

pub struct StatusBar {
    pub left: Vec<StatusSegment>,
    pub center: Vec<StatusSegment>,
    pub right: Vec<StatusSegment>,
}

pub struct StatusSegment {
    pub text: String,
    pub fg: Option<Color>,
    pub bg: Option<Color>,
    pub bold: bool,
}

impl StatusSegment {
    pub fn text(s: impl Into<String>) -> Self {
        Self {
            text: s.into(),
            fg: None,
            bg: None,
            bold: false,
        }
    }

    pub fn fg(mut self, color: Color) -> Self {
        self.fg = Some(color);
        self
    }

    pub fn bg(mut self, color: Color) -> Self {
        self.bg = Some(color);
        self
    }

    pub fn bold(mut self) -> Self {
        self.bold = true;
        self
    }
}

impl StatusBar {
    pub fn new() -> Self {
        Self {
            left: Vec::new(),
            center: Vec::new(),
            right: Vec::new(),
        }
    }

    pub fn left(mut self, segment: StatusSegment) -> Self {
        self.left.push(segment);
        self
    }

    pub fn center(mut self, segment: StatusSegment) -> Self {
        self.center.push(segment);
        self
    }

    pub fn right(mut self, segment: StatusSegment) -> Self {
        self.right.push(segment);
        self
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        let w = canvas.width() as usize;

        for x in 0..w {
            canvas.set_char(x as u16, 0, ' ', palette.fg, palette.surface, Modifier::empty());
        }

        let mut col = 0usize;
        for seg in &self.left {
            col = Self::write_segment(canvas, col, w, seg, palette);
        }

        let right_total: usize = self.right.iter().map(|s| s.text.len()).sum();
        let right_start = w.saturating_sub(right_total);

        let center_total: usize = self.center.iter().map(|s| s.text.len()).sum();
        let center_start = (w.saturating_sub(center_total)) / 2;

        let mut col = center_start;
        for seg in &self.center {
            col = Self::write_segment(canvas, col, right_start, seg, palette);
        }

        let mut col = right_start;
        for seg in &self.right {
            col = Self::write_segment(canvas, col, w, seg, palette);
        }
        let _ = col;
    }

    fn write_segment(
        canvas: &mut CanvasSubviewMut<'_>,
        start: usize,
        limit: usize,
        seg: &StatusSegment,
        palette: &ThemePalette,
    ) -> usize {
        let fg = seg.fg.unwrap_or(palette.fg);
        let bg = seg.bg.unwrap_or(palette.surface);
        let mods = if seg.bold { Modifier::BOLD } else { Modifier::empty() };
        let mut col = start;
        for ch in seg.text.chars() {
            if col >= limit {
                break;
            }
            canvas.set_char(col as u16, 0, ch, fg, bg, mods);
            col += 1;
        }
        col
    }
}

impl Default for StatusBar {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn status_bar_renders_sections() {
        let bar = StatusBar::new()
            .left(StatusSegment::text(" NORMAL ").bold())
            .center(StatusSegment::text("main.rs"))
            .right(StatusSegment::text("Ln 42 Col 8 "));

        let palette = ThemePalette::catppuccin_mocha();
        let mut buf = Buffer::new(60, 1);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 1));
        bar.render(&mut canvas, &palette);

        assert_eq!(buf.get(1, 0).unwrap().symbol.as_str(), "N");
        let center_start = (60 - 7) / 2;
        assert_eq!(buf.get(center_start, 0).unwrap().symbol.as_str(), "m");
    }

    #[test]
    fn status_bar_empty() {
        let bar = StatusBar::new();
        let palette = ThemePalette::catppuccin_mocha();
        let mut buf = Buffer::new(20, 1);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 20, 1));
        bar.render(&mut canvas, &palette);

        assert_eq!(buf.get(0, 0).unwrap().bg, palette.surface);
    }
}
