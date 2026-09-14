use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Half-block subpixel rasterizer using '▀' (U+2580).
/// Doubles vertical resolution (each cell holds 2 vertical pixels).
pub struct HalfBlockCanvas {
    pub cell_width: u16,
    pub cell_height: u16,
    pixels: Vec<Color>,
}

impl HalfBlockCanvas {
    pub fn new(cell_width: u16, cell_height: u16) -> Self {
        let pixel_count = cell_width as usize * (cell_height as usize * 2);
        Self {
            cell_width,
            cell_height,
            pixels: vec![Color::Reset; pixel_count],
        }
    }

    pub fn pixel_width(&self) -> u16 {
        self.cell_width
    }

    pub fn pixel_height(&self) -> u16 {
        self.cell_height * 2
    }

    #[inline]
    fn pixel_index(&self, px: u16, py: u16) -> Option<usize> {
        if px < self.pixel_width() && py < self.pixel_height() {
            Some(py as usize * self.pixel_width() as usize + px as usize)
        } else {
            None
        }
    }

    pub fn get_pixel(&self, px: u16, py: u16) -> Color {
        self.pixel_index(px, py)
            .and_then(|idx| self.pixels.get(idx))
            .copied()
            .unwrap_or(Color::Reset)
    }

    pub fn set_pixel(&mut self, px: u16, py: u16, color: Color) {
        if let Some(idx) = self.pixel_index(px, py) {
            self.pixels[idx] = color;
        }
    }

    /// Blends an RGB color with the existing pixel using linear alpha interpolation.
    pub fn blend_pixel(&mut self, px: u16, py: u16, (r, g, b): (u8, u8, u8), alpha: f32) {
        let alpha = alpha.clamp(0.0, 1.0);
        let existing = self.get_pixel(px, py);
        let (er, eg, eb) = match existing {
            Color::Rgb(er, eg, eb) => (er, eg, eb),
            _ => (0, 0, 0),
        };

        let nr = ((r as f32 * alpha) + (er as f32 * (1.0 - alpha))).round() as u8;
        let ng = ((g as f32 * alpha) + (eg as f32 * (1.0 - alpha))).round() as u8;
        let nb = ((b as f32 * alpha) + (eb as f32 * (1.0 - alpha))).round() as u8;

        self.set_pixel(px, py, Color::Rgb(nr, ng, nb));
    }

    /// Draws an anti-aliased line using Xiaolin Wu's algorithm with 24-bit linear blending.
    pub fn draw_line_aa(&mut self, mut x0: f32, mut y0: f32, mut x1: f32, mut y1: f32, color: (u8, u8, u8)) {
        let steep = (y1 - y0).abs() > (x1 - x0).abs();
        if steep {
            std::mem::swap(&mut x0, &mut y0);
            std::mem::swap(&mut x1, &mut y1);
        }
        if x0 > x1 {
            std::mem::swap(&mut x0, &mut x1);
            std::mem::swap(&mut y0, &mut y1);
        }

        let dx = x1 - x0;
        let dy = y1 - y0;
        let gradient = if dx == 0.0 { 1.0 } else { dy / dx };

        // Handle first endpoint
        let xend = x0.round();
        let yend = y0 + gradient * (xend - x0);
        let xgap = 1.0 - (x0 + 0.5).fract();
        let xpxl1 = xend as i32;
        let ypxl1 = yend.floor() as i32;

        if steep {
            self.plot(ypxl1, xpxl1, color, (1.0 - yend.fract()) * xgap);
            self.plot(ypxl1 + 1, xpxl1, color, yend.fract() * xgap);
        } else {
            self.plot(xpxl1, ypxl1, color, (1.0 - yend.fract()) * xgap);
            self.plot(xpxl1, ypxl1 + 1, color, yend.fract() * xgap);
        }

        let mut intery = yend + gradient;

        // Handle second endpoint
        let xend = x1.round();
        let yend = y1 + gradient * (xend - x1);
        let xgap = (x1 + 0.5).fract();
        let xpxl2 = xend as i32;
        let ypxl2 = yend.floor() as i32;

        if steep {
            self.plot(ypxl2, xpxl2, color, (1.0 - yend.fract()) * xgap);
            self.plot(ypxl2 + 1, xpxl2, color, yend.fract() * xgap);
        } else {
            self.plot(xpxl2, ypxl2, color, (1.0 - yend.fract()) * xgap);
            self.plot(xpxl2, ypxl2 + 1, color, yend.fract() * xgap);
        }

        // Main loop
        if steep {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                self.plot(y, x, color, 1.0 - intery.fract());
                self.plot(y + 1, x, color, intery.fract());
                intery += gradient;
            }
        } else {
            for x in (xpxl1 + 1)..xpxl2 {
                let y = intery.floor() as i32;
                self.plot(x, y, color, 1.0 - intery.fract());
                self.plot(x, y + 1, color, intery.fract());
                intery += gradient;
            }
        }
    }

    fn plot(&mut self, x: i32, y: i32, color: (u8, u8, u8), alpha: f32) {
        if x >= 0 && y >= 0 && (x as u16) < self.pixel_width() && (y as u16) < self.pixel_height() {
            self.blend_pixel(x as u16, y as u16, color, alpha);
        }
    }

    /// Flushes the rasterized half-block pixels into a hardware-clipped subview.
    pub fn render_to_subview(&self, subview: &mut CanvasSubviewMut<'_>) {
        let w = self.cell_width.min(subview.width());
        let h = self.cell_height.min(subview.height());

        for y in 0..h {
            for x in 0..w {
                let top_color = self.get_pixel(x, y * 2);
                let bot_color = self.get_pixel(x, y * 2 + 1);

                if top_color == Color::Reset && bot_color == Color::Reset {
                    subview.set_char(x, y, ' ', Color::Reset, Color::Reset, Modifier::empty());
                } else {
                    subview.set_char(x, y, '▀', top_color, bot_color, Modifier::empty());
                }
            }
        }
    }
}
