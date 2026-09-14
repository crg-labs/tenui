use tenui_core::{CanvasSubviewMut, Color, Modifier};

const FRACTIONAL_BLOCKS: [char; 9] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉', '█'];
const VERTICAL_BARS: [char; 8] = [' ', '▂', '▃', '▄', '▅', '▆', '▇', '█'];

/// A smooth fractional progress bar utilizing Unicode 1/8th sub-character blocks.
pub struct SmoothProgressBar {
    pub progress: f32, // 0.0 to 1.0
}

impl SmoothProgressBar {
    pub fn new(progress: f32) -> Self {
        Self {
            progress: progress.clamp(0.0, 1.0),
        }
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = subview.width();
        if w == 0 {
            return;
        }

        let total_eighths = (self.progress * (w as f32 * 8.0)).round() as usize;
        let full_blocks = total_eighths / 8;
        let remainder = total_eighths % 8;

        for x in 0..w {
            let ch = if (x as usize) < full_blocks {
                FRACTIONAL_BLOCKS[8] // '█'
            } else if (x as usize) == full_blocks && remainder > 0 {
                FRACTIONAL_BLOCKS[remainder]
            } else {
                ' '
            };
            subview.set_char(x, 0, ch, fg, bg, Modifier::empty());
        }
    }
}

/// Mini inline sparkline chart for numerical sequences.
pub struct Sparkline<'a> {
    pub data: &'a [u64],
}

impl<'a> Sparkline<'a> {
    pub fn new(data: &'a [u64]) -> Self {
        Self { data }
    }

    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = subview.width() as usize;
        if w == 0 || self.data.is_empty() {
            return;
        }

        let max_val = self.data.iter().copied().max().unwrap_or(1).max(1);

        // Take the latest `w` data points
        let start = self.data.len().saturating_sub(w);
        let slice = &self.data[start..];

        for (x, &val) in slice.iter().enumerate() {
            let normalized = (val as f32 / max_val as f32).clamp(0.0, 1.0);
            let bar_idx = ((normalized * 7.0).round() as usize).min(7);
            let ch = VERTICAL_BARS[bar_idx];
            subview.set_char(x as u16, 0, ch, fg, bg, Modifier::empty());
        }
    }
}

pub mod spinner;
pub use spinner::{Spinner, SpinnerStyle};
