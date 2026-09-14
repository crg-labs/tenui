use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Braille High-Density 2x4 Vector Matrix using Unicode Braille Patterns (U+2800..U+28FF).
/// Multiplies resolution by 2x horizontally and 4x vertically.
pub struct BrailleCanvas {
    pub cell_width: u16,
    pub cell_height: u16,
    cells: Vec<u8>,
}

impl BrailleCanvas {
    pub fn new(cell_width: u16, cell_height: u16) -> Self {
        let count = cell_width as usize * cell_height as usize;
        Self {
            cell_width,
            cell_height,
            cells: vec![0; count],
        }
    }

    pub fn dot_width(&self) -> u16 {
        self.cell_width * 2
    }

    pub fn dot_height(&self) -> u16 {
        self.cell_height * 4
    }

    pub fn clear(&mut self) {
        self.cells.fill(0);
    }

    /// Sets an individual sub-cell dot.
    pub fn set_dot(&mut self, dx: u16, dy: u16) {
        if dx >= self.dot_width() || dy >= self.dot_height() {
            return;
        }

        let cell_x = dx / 2;
        let cell_y = dy / 4;
        let sub_x = dx % 2;
        let sub_y = dy % 4;

        let bit_mask: u8 = match (sub_x, sub_y) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (0, 3) => 0x40,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (1, 3) => 0x80,
            _ => 0,
        };

        let idx = cell_y as usize * self.cell_width as usize + cell_x as usize;
        if let Some(cell) = self.cells.get_mut(idx) {
            *cell |= bit_mask;
        }
    }

    /// Draws a line using the integer-only Bresenham algorithm.
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_dot(x0 as u16, y0 as u16);
            }

            if x0 == x1 && y0 == y1 {
                break;
            }

            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws a circle outline using the integer midpoint circle algorithm.
    pub fn draw_circle(&mut self, cx: i32, cy: i32, radius: i32) {
        let mut x = radius;
        let mut y = 0;
        let mut err = 0;

        while x >= y {
            self.plot_circle_points(cx, cy, x, y);

            if err <= 0 {
                y += 1;
                err += 2 * y + 1;
            }
            if err > 0 {
                x -= 1;
                err -= 2 * x + 1;
            }
        }
    }

    fn plot_circle_points(&mut self, cx: i32, cy: i32, x: i32, y: i32) {
        let points = [
            (cx + x, cy + y),
            (cx + y, cy + x),
            (cx - y, cy + x),
            (cx - x, cy + y),
            (cx - x, cy - y),
            (cx - y, cy - x),
            (cx + y, cy - x),
            (cx + x, cy - y),
        ];

        for (px, py) in points {
            if px >= 0 && py >= 0 {
                self.set_dot(px as u16, py as u16);
            }
        }
    }

    /// Flushes the rasterized Braille pattern into a clipped subview.
    pub fn render_to_subview(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = self.cell_width.min(subview.width());
        let h = self.cell_height.min(subview.height());

        for y in 0..h {
            for x in 0..w {
                let idx = y as usize * self.cell_width as usize + x as usize;
                let pattern = self.cells.get(idx).copied().unwrap_or(0);
                let braille_char = char::from_u32(0x2800 + pattern as u32).unwrap_or(' ');
                subview.set_char(x, y, braille_char, fg, bg, Modifier::empty());
            }
        }
    }
}
