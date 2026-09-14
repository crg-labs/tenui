use tenui_core::{CanvasSubviewMut, Color, Modifier};

use crate::theme::ThemePalette;

#[derive(Debug, Clone, Copy)]
pub struct Hsv {
    pub h: f32,
    pub s: f32,
    pub v: f32,
}

impl Hsv {
    pub fn new(h: f32, s: f32, v: f32) -> Self {
        Self {
            h: h.clamp(0.0, 360.0),
            s: s.clamp(0.0, 1.0),
            v: v.clamp(0.0, 1.0),
        }
    }

    pub fn to_rgb(self) -> (u8, u8, u8) {
        let c = self.v * self.s;
        let h_prime = self.h / 60.0;
        let x = c * (1.0 - (h_prime % 2.0 - 1.0).abs());
        let m = self.v - c;

        let (r1, g1, b1) = match h_prime as u32 {
            0 => (c, x, 0.0),
            1 => (x, c, 0.0),
            2 => (0.0, c, x),
            3 => (0.0, x, c),
            4 => (x, 0.0, c),
            _ => (c, 0.0, x),
        };

        (
            ((r1 + m) * 255.0).round() as u8,
            ((g1 + m) * 255.0).round() as u8,
            ((b1 + m) * 255.0).round() as u8,
        )
    }
}

pub struct ColorPicker {
    pub hsv: Hsv,
    pub cursor_x: u16,
    pub cursor_y: u16,
}

impl ColorPicker {
    pub fn new() -> Self {
        Self {
            hsv: Hsv::new(0.0, 1.0, 1.0),
            cursor_x: 0,
            cursor_y: 0,
        }
    }

    pub fn selected_color(&self) -> Color {
        let (r, g, b) = self.hsv.to_rgb();
        Color::Rgb(r, g, b)
    }

    pub fn selected_hex(&self) -> String {
        let (r, g, b) = self.hsv.to_rgb();
        format!("#{:02X}{:02X}{:02X}", r, g, b)
    }

    pub fn set_hue(&mut self, h: f32) {
        self.hsv.h = h.clamp(0.0, 360.0);
    }

    pub fn set_saturation(&mut self, s: f32) {
        self.hsv.s = s.clamp(0.0, 1.0);
    }

    pub fn set_value(&mut self, v: f32) {
        self.hsv.v = v.clamp(0.0, 1.0);
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        let w = canvas.width();
        let h = canvas.height();
        if w < 4 || h < 4 {
            return;
        }

        let gradient_h = h.saturating_sub(3);

        for y in 0..gradient_h {
            let v = 1.0 - (y as f32 / gradient_h.max(1) as f32);
            for x in 0..w.saturating_sub(2) {
                let s = x as f32 / w.saturating_sub(2).max(1) as f32;
                let (r, g, b) = Hsv::new(self.hsv.h, s, v).to_rgb();
                canvas.set_char(x, y, '▀', Color::Rgb(r, g, b), Color::Reset, Modifier::empty());
            }
        }

        let hue_y = gradient_h;
        for x in 0..w.saturating_sub(2) {
            let hue = (x as f32 / w.saturating_sub(2).max(1) as f32) * 360.0;
            let (r, g, b) = Hsv::new(hue, 1.0, 1.0).to_rgb();
            canvas.set_char(x, hue_y, '█', Color::Rgb(r, g, b), Color::Reset, Modifier::empty());
        }

        let hex = self.selected_hex();
        let label = format!(" {} ", hex);
        let label_y = h.saturating_sub(1);
        for (i, ch) in label.chars().enumerate() {
            if (i as u16) < w {
                canvas.set_char(i as u16, label_y, ch, palette.fg, palette.surface, Modifier::empty());
            }
        }
    }
}

impl Default for ColorPicker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn hsv_red() {
        let hsv = Hsv::new(0.0, 1.0, 1.0);
        assert_eq!(hsv.to_rgb(), (255, 0, 0));
    }

    #[test]
    fn hsv_green() {
        let hsv = Hsv::new(120.0, 1.0, 1.0);
        assert_eq!(hsv.to_rgb(), (0, 255, 0));
    }

    #[test]
    fn hsv_blue() {
        let hsv = Hsv::new(240.0, 1.0, 1.0);
        assert_eq!(hsv.to_rgb(), (0, 0, 255));
    }

    #[test]
    fn color_picker_hex() {
        let mut picker = ColorPicker::new();
        picker.set_hue(0.0);
        picker.set_saturation(1.0);
        picker.set_value(1.0);
        assert_eq!(picker.selected_hex(), "#FF0000");
    }

    #[test]
    fn color_picker_renders() {
        let picker = ColorPicker::new();
        let palette = ThemePalette::catppuccin_mocha();
        let mut buf = Buffer::new(20, 10);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 20, 10));
        picker.render(&mut canvas, &palette);

        assert_ne!(buf.get(0, 0).unwrap().symbol.as_str(), " ");
    }
}
