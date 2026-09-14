use tenui_core::{canvas::CanvasSubviewMut, cell::Cell, color::Color, geometry::Rect};

/// A stateless, deterministic per-cell fragment kernel executed in scalar time.
pub trait CellShader: Send + Sync {
    fn shade_cell(&self, local_x: u16, local_y: u16, bounds: Rect, cell: &mut Cell, elapsed_time: f32);

    /// Applies the shader kernel across all cells in the canvas subview.
    fn apply(&self, surface: &mut CanvasSubviewMut, elapsed_time: f32) {
        let bounds = surface.bounds;
        for y in 0..bounds.height {
            for x in 0..bounds.width {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    self.shade_cell(x, y, bounds, cell, elapsed_time);
                }
            }
        }
    }
}

/// Emulates vintage broadcast monitors and CRT terminals with scanlines and phosphor tint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CrtScanlineShader {
    pub scanline_darkening: f32,
    pub phosphor_tint: Option<Color>,
    pub flicker_intensity: f32,
}

impl Default for CrtScanlineShader {
    fn default() -> Self {
        Self {
            scanline_darkening: 0.25,
            phosphor_tint: Some(Color::Rgb(51, 255, 51)), // Classic P1 green phosphor
            flicker_intensity: 0.05,
        }
    }
}

impl CellShader for CrtScanlineShader {
    fn shade_cell(&self, _local_x: u16, local_y: u16, _bounds: Rect, cell: &mut Cell, elapsed_time: f32) {
        // 1. Scanline Attenuation (alternating rows)
        if local_y % 2 == 1 {
            cell.fg = cell.fg.dim(self.scanline_darkening);
            cell.bg = cell.bg.dim(self.scanline_darkening);
        }

        // 2. Refresh Flicker
        let flicker = (elapsed_time * 60.0).sin() * self.flicker_intensity;
        if flicker > 0.0 {
            cell.fg = cell.fg.brighten(flicker);
        } else {
            cell.fg = cell.fg.dim(-flicker);
        }

        // 3. Phosphor Decay Tint
        if let Some(phosphor) = self.phosphor_tint {
            cell.fg = cell.fg.lerp(phosphor, 0.15);
        }
    }
}

/// Content placeholder skeleton loader sweeping a concentrated diagonal light beam.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SinusoidalShimmerShader {
    pub speed: f32,
    pub beam_width: f32,
    pub highlight_color: Color,
}

impl Default for SinusoidalShimmerShader {
    fn default() -> Self {
        Self {
            speed: 1.5,
            beam_width: 0.05,
            highlight_color: Color::Rgb(255, 255, 255),
        }
    }
}

impl CellShader for SinusoidalShimmerShader {
    fn shade_cell(&self, local_x: u16, local_y: u16, bounds: Rect, cell: &mut Cell, elapsed_time: f32) {
        let max_dim = (bounds.width + bounds.height * 2).max(1) as f32;
        let proj = (local_x + local_y * 2) as f32 / max_dim;
        let phase = (proj - elapsed_time * self.speed).rem_euclid(1.0);

        let dist_from_center = phase - 0.5;
        let intensity = (-((dist_from_center * dist_from_center) / self.beam_width)).exp();

        if intensity > 0.01 {
            cell.bg = cell.bg.lerp(self.highlight_color, intensity * 0.4);
            cell.fg = cell.fg.lerp(self.highlight_color, intensity * 0.7);
        }
    }
}
