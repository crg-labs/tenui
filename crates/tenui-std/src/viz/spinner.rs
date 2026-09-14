use std::time::Duration;

use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Discrete frame sequence for animated spinner archetypes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SpinnerStyle {
    Frames(&'static [char]),
}

impl SpinnerStyle {
    pub fn total_frames(&self) -> usize {
        match self {
            SpinnerStyle::Frames(f) => f.len(),
        }
    }

    pub fn resolve_glyph(&self, idx: usize) -> char {
        match self {
            SpinnerStyle::Frames(f) => {
                if f.is_empty() {
                    ' '
                } else {
                    f[idx % f.len()]
                }
            }
        }
    }
}

/// Discrete frame-based kinetic spinner engine.
#[derive(Clone, Copy, Debug)]
pub struct Spinner {
    pub style: SpinnerStyle,
    pub color: Color,
    pub interval: Duration,
}

impl Spinner {
    pub const BRAILLE_ORBIT: Self = Self {
        style: SpinnerStyle::Frames(&['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏']),
        color: Color::CYAN,
        interval: Duration::from_millis(80),
    };

    pub const MICRO_BLOCK: Self = Self {
        style: SpinnerStyle::Frames(&['▖', '▘', '▝', '▗']),
        color: Color::GREEN,
        interval: Duration::from_millis(100),
    };

    pub const PULSE_DOT: Self = Self {
        style: SpinnerStyle::Frames(&['⠂', '⠒', '⠲', '⠴', '⠤', '⠄']),
        color: Color::YELLOW,
        interval: Duration::from_millis(120),
    };

    pub const BOUNCING_BAR: Self = Self {
        style: SpinnerStyle::Frames(&['▏', '▎', '▍', '▌', '▋', '▊', '▉', '█', '▉', '▊', '▋', '▌', '▍', '▎']),
        color: Color::MAGENTA,
        interval: Duration::from_millis(60),
    };

    pub fn paint(&self, surface: &mut CanvasSubviewMut<'_>, elapsed: Duration) {
        let total = self.style.total_frames();
        if total == 0 {
            return;
        }
        let frame_idx = ((elapsed.as_millis() / self.interval.as_millis().max(1)) % total as u128) as usize;
        let glyph = self.style.resolve_glyph(frame_idx);
        surface.set_char(0, 0, glyph, self.color, Color::TRANSPARENT, Modifier::empty());
    }

    /// Resolves the character glyph for a specific discrete frame index.
    pub fn glyph_for_frame(&self, frame_idx: usize) -> char {
        self.style.resolve_glyph(frame_idx)
    }
}
