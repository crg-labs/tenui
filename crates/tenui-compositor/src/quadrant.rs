use tenui_core::{CanvasSubviewMut, Color, Modifier};

const QUADRANT_CHARS: [char; 16] = [
    ' ', // 0b0000
    '▖', // 0b0001: Lower-left
    '▗', // 0b0010: Lower-right
    '▄', // 0b0011: Lower-half
    '▘', // 0b0100: Upper-left
    '▌', // 0b0101: Left-half
    '▚', // 0b0110: Upper-left + lower-right
    '▙', // 0b0111: Upper-left + lower-left + lower-right
    '▝', // 0b1000: Upper-right
    '▞', // 0b1001: Upper-right + lower-left
    '▐', // 0b1010: Right-half
    '▟', // 0b1011: Upper-right + lower-left + lower-right
    '▀', // 0b1100: Upper-half
    '▛', // 0b1101: Upper-left + upper-right + lower-left
    '▜', // 0b1110: Upper-left + upper-right + lower-right
    '█', // 0b1111: Full block
];

/// Quadrant subpixel matrix providing a 2x2 graphic plane per character cell
/// using Unicode Block Elements (U+2596..U+259F).
pub struct QuadrantCanvas {
    pub cell_width: u16,
    pub cell_height: u16,
    masks: Vec<u8>,
}

impl QuadrantCanvas {
    pub fn new(cell_width: u16, cell_height: u16) -> Self {
        let count = cell_width as usize * cell_height as usize;
        Self {
            cell_width,
            cell_height,
            masks: vec![0; count],
        }
    }

    pub fn pixel_width(&self) -> u16 {
        self.cell_width * 2
    }

    pub fn pixel_height(&self) -> u16 {
        self.cell_height * 2
    }

    pub fn clear(&mut self) {
        self.masks.fill(0);
    }

    /// Sets a subpixel dot in 2x2 space.
    pub fn set_pixel(&mut self, px: u16, py: u16) {
        if px >= self.pixel_width() || py >= self.pixel_height() {
            return;
        }

        let cell_x = px / 2;
        let cell_y = py / 2;
        let sub_x = px % 2;
        let sub_y = py % 2;

        let bit = match (sub_x, sub_y) {
            (0, 1) => 0b0001, // lower-left
            (1, 1) => 0b0010, // lower-right
            (0, 0) => 0b0100, // upper-left
            (1, 0) => 0b1000, // upper-right
            _ => 0,
        };

        let idx = cell_y as usize * self.cell_width as usize + cell_x as usize;
        if let Some(mask) = self.masks.get_mut(idx) {
            *mask |= bit;
        }
    }

    /// Flushes the quadrant subpixels into a hardware-clipped subview.
    pub fn render_to_subview(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = self.cell_width.min(subview.width());
        let h = self.cell_height.min(subview.height());

        for y in 0..h {
            for x in 0..w {
                let idx = y as usize * self.cell_width as usize + x as usize;
                let mask = (self.masks.get(idx).copied().unwrap_or(0) & 0x0F) as usize;
                let ch = QUADRANT_CHARS[mask];
                subview.set_char(x, y, ch, fg, bg, Modifier::empty());
            }
        }
    }
}
