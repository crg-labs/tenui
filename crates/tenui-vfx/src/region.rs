use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Regional Signed Distance Field (SDF) stencils and sub-cell geometric masking.
pub struct RegionStencilMask;

impl RegionStencilMask {
    /// Clips a region to an analytical circle using sub-cell quarter blocks (U+2596..U+259F).
    pub fn mask_circle(surface: &mut CanvasSubviewMut, fill_color: Color) {
        let w = surface.bounds.width as f32;
        let h = surface.bounds.height as f32;
        let k = 2.0f32;
        let cx = w / 2.0;
        let cy = h / 2.0;
        let radius = (w.min(h * k) / 2.0) - 0.5;

        for y in 0..surface.bounds.height {
            for x in 0..surface.bounds.width {
                // Sample 4 sub-pixel quadrant points
                let mut bitmask = 0u8;
                let offsets = [
                    (0.25, 0.25, 0x01), // Top-left
                    (0.75, 0.25, 0x02), // Top-right
                    (0.25, 0.75, 0x04), // Bottom-left
                    (0.75, 0.75, 0x08), // Bottom-right
                ];

                for (ox, oy, bit) in offsets {
                    let px = x as f32 + ox;
                    let py = y as f32 + oy;
                    let dist = ((px - cx) * (px - cx) + (k * (py - cy)) * (k * (py - cy))).sqrt();
                    if dist <= radius {
                        bitmask |= bit;
                    }
                }

                match bitmask {
                    0x0F => {
                        // Fully interior: solid block
                        if let Some(cell) = surface.get_cell_mut(x, y) {
                            cell.bg = fill_color;
                        }
                    }
                    0x00 => {
                        // Fully exterior: preserve untouched
                    }
                    _ => {
                        // Fractional boundary quadrant block
                        let glyph = match bitmask {
                            0x01 => '▘',
                            0x02 => '▝',
                            0x03 => '▀',
                            0x04 => '▖',
                            0x05 => '▌',
                            0x06 => '▞',
                            0x07 => '▛',
                            0x08 => '▗',
                            0x09 => '▚',
                            0x0A => '▐',
                            0x0B => '▜',
                            0x0C => '▄',
                            0x0D => '▙',
                            0x0E => '▟',
                            _ => ' ',
                        };
                        surface.set_cell_styled(x, y, glyph, fill_color, Color::TRANSPARENT);
                    }
                }
            }
        }
    }
}

/// Dynamic scene transition operators.
pub struct SceneTransition;

impl SceneTransition {
    /// Venetian blind horizontal wipe transition between two phases (progress: 0.0 -> 1.0).
    pub fn venetian_wipe(surface: &mut CanvasSubviewMut, progress: f32, mask_color: Color) {
        let h = surface.bounds.height;
        let w = surface.bounds.width;
        let band_size = 4u16;

        for y in 0..h {
            let band_phase = ((y % band_size) as f32 / band_size as f32) * 0.3;
            let row_progress = (progress + band_phase).clamp(0.0, 1.0);
            let cutoff_x = (w as f32 * row_progress).round() as u16;

            for x in 0..cutoff_x.min(w) {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    cell.bg = cell.bg.lerp(mask_color, 0.7);
                    cell.fg = cell.fg.dim(0.3);
                }
            }
        }
    }

    /// Radial iris bloom transition centered at (cx, cy).
    pub fn radial_iris(surface: &mut CanvasSubviewMut, cx: u16, cy: u16, progress: f32, fill_color: Color) {
        let k = 2.0f32;
        let max_r = (surface.bounds.width as f32).hypot(surface.bounds.height as f32 * k);
        let current_r = max_r * progress.clamp(0.0, 1.0);

        for y in 0..surface.bounds.height {
            for x in 0..surface.bounds.width {
                let dx = x as f32 - cx as f32;
                let dy = y as f32 - cy as f32;
                let dist = (dx * dx + (k * dy) * (k * dy)).sqrt();

                if dist <= current_r
                    && let Some(cell) = surface.get_cell_mut(x, y)
                {
                    cell.bg = fill_color;
                }
            }
        }
    }
}

/// Refractive displacement and warp field operators.
pub struct WarpField;

impl WarpField {
    /// Applies an animated refractive heatwave displacement across the surface.
    pub fn apply_heatwave(surface: &mut CanvasSubviewMut, amplitude: f32, frequency: f32, elapsed_secs: f32) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;

        // Apply a subtle sinusoidal luminance modulation simulating refractive heat
        for y in 0..h {
            let wave = ((y as f32 * frequency) + (elapsed_secs * 3.0)).sin() * amplitude;
            let factor = 1.0 + wave * 0.15;

            for x in 0..w {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    if factor > 1.0 {
                        cell.fg = cell.fg.brighten((factor - 1.0).min(0.3));
                    } else {
                        cell.fg = cell.fg.dim((1.0 - factor).min(0.3));
                    }
                }
            }
        }
    }
}
