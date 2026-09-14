/// A character cell coordinate broken down into whole cells and fractional sub-cell glyph.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct MicroStepCell {
    pub whole_cells: u16,
    pub fractional_glyph: Option<char>,
}

/// Fractional sub-cell micro-stepping engine using Unicode 1/8th block character matrices.
///
/// Converts coarse 1-cell hops into 1/8th sub-cell trajectories.
pub struct MicroStepper;

impl MicroStepper {
    /// Horizontal 1/8th Unicode block characters (U+258F .. U+2588).
    pub const HORIZONTAL_LUT: [char; 8] = [' ', '▏', '▎', '▍', '▌', '▋', '▊', '▉'];

    /// Vertical lower 1/8th Unicode block characters (U+2581 .. U+2588).
    pub const VERTICAL_LUT: [char; 8] = [' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇'];

    /// Maps continuous floating-point horizontal coordinate into integer whole cells and fractional glyph.
    pub fn resolve_horizontal(value: f32) -> MicroStepCell {
        if value <= 0.0 {
            return MicroStepCell {
                whole_cells: 0,
                fractional_glyph: None,
            };
        }
        let whole = value.floor() as u16;
        let frac = value - value.floor();
        let step_idx = ((frac * 8.0).floor() as usize).clamp(0, 7);

        MicroStepCell {
            whole_cells: whole,
            fractional_glyph: if step_idx == 0 {
                None
            } else {
                Some(Self::HORIZONTAL_LUT[step_idx])
            },
        }
    }

    /// Maps continuous floating-point vertical coordinate into integer whole cells and fractional lower block glyph.
    pub fn resolve_vertical(value: f32) -> MicroStepCell {
        if value <= 0.0 {
            return MicroStepCell {
                whole_cells: 0,
                fractional_glyph: None,
            };
        }
        let whole = value.floor() as u16;
        let frac = value - value.floor();
        let step_idx = ((frac * 8.0).floor() as usize).clamp(0, 7);

        MicroStepCell {
            whole_cells: whole,
            fractional_glyph: if step_idx == 0 {
                None
            } else {
                Some(Self::VERTICAL_LUT[step_idx])
            },
        }
    }
}
