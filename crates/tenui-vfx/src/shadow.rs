use tenui_core::{canvas::CanvasSubviewMut, color::Color, geometry::Rect};

/// Directional drop shadow configuration with aspect-ratio-corrected convolution.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DropShadow {
    pub offset_x: i16,
    pub offset_y: i16,
    pub blur_radius: f32,
    pub max_opacity: f32,
    pub shadow_color: Color,
    pub enable_subpixel_aa: bool,
}

impl DropShadow {
    pub const ELEVATED: Self = Self {
        offset_x: 2,
        offset_y: 1,
        blur_radius: 2.0,
        max_opacity: 0.55,
        shadow_color: Color::BLACK,
        enable_subpixel_aa: true,
    };

    pub const SUBTLE: Self = Self {
        offset_x: 1,
        offset_y: 1,
        blur_radius: 1.2,
        max_opacity: 0.35,
        shadow_color: Color::BLACK,
        enable_subpixel_aa: true,
    };

    pub const DEEP: Self = Self {
        offset_x: 3,
        offset_y: 2,
        blur_radius: 3.0,
        max_opacity: 0.70,
        shadow_color: Color::BLACK,
        enable_subpixel_aa: true,
    };

    /// Renders an aspect-ratio-corrected drop shadow around a source rectangle.
    pub fn render_shadow(&self, surface: &mut CanvasSubviewMut, content_bounds: Rect) {
        let k = 2.0f32; // Cell aspect ratio correction factor
        let w = surface.bounds.width as i16;
        let h = surface.bounds.height as i16;

        let shadow_x0 = content_bounds.x as f32 + self.offset_x as f32;
        let shadow_y0 = content_bounds.y as f32 + self.offset_y as f32;
        let shadow_x1 = shadow_x0 + content_bounds.width as f32;
        let shadow_y1 = shadow_y0 + content_bounds.height as f32;

        let sigma = self.blur_radius.max(0.5);

        for y in 0..h {
            for x in 0..w {
                let fx = x as f32;
                let fy = y as f32;

                // Inside the original content bounds, do not overwrite content
                if fx >= content_bounds.x as f32
                    && fx < (content_bounds.x + content_bounds.width) as f32
                    && fy >= content_bounds.y as f32
                    && fy < (content_bounds.y + content_bounds.height) as f32
                {
                    continue;
                }

                // Signed orthogonal distance to the shadow rectangle
                let dx = if fx < shadow_x0 {
                    shadow_x0 - fx
                } else if fx >= shadow_x1 {
                    fx - (shadow_x1 - 1.0)
                } else {
                    0.0
                };

                let dy = if fy < shadow_y0 {
                    shadow_y0 - fy
                } else if fy >= shadow_y1 {
                    fy - (shadow_y1 - 1.0)
                } else {
                    0.0
                };

                // Anisotropic distance metric dk
                let dk = (dx * dx + (k * dy) * (k * dy)).sqrt();

                if dk <= (self.blur_radius * 2.5) {
                    let alpha = (self.max_opacity * (-dk / sigma).exp()).clamp(0.0, self.max_opacity);
                    if alpha > 0.02
                        && let Some(cell) = surface.get_cell_mut(x as u16, y as u16)
                    {
                        cell.bg = cell.bg.lerp(self.shadow_color, alpha);

                        // Quarter-block subpixel corner smoothing for outer boundaries
                        if self.enable_subpixel_aa && dk > (self.blur_radius * 1.2) && cell.symbol.is_empty() {
                            let frac_x = fx - fx.floor();
                            let frac_y = fy - fy.floor();
                            let bitmask = match (frac_x >= 0.5, frac_y >= 0.5) {
                                (false, false) => '▘',
                                (true, false) => '▝',
                                (false, true) => '▖',
                                (true, true) => '▗',
                            };
                            if alpha > 0.15 {
                                cell.symbol = tenui_core::cell::CompactSymbol::from_char(bitmask);
                                cell.fg = cell.bg.lerp(self.shadow_color, alpha * 0.5);
                            }
                        }
                    }
                }
            }
        }
    }
}

/// Non-invasive radial focus glow halo emitted into surrounding margin cells.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FocusHalo {
    pub color: Color,
    pub intensity: f32,
    pub radius: f32,
    pub pulse_frequency_hz: Option<f32>,
}

impl FocusHalo {
    pub const CYAN_PULSE: Self = Self {
        color: Color::CYAN,
        intensity: 0.45,
        radius: 2.0,
        pulse_frequency_hz: Some(1.5),
    };

    /// Renders the radial glow halo into margin cells around content_bounds.
    pub fn render_halo(&self, surface: &mut CanvasSubviewMut, content_bounds: Rect, elapsed_secs: f32) {
        let k = 2.0f32;
        let w = surface.bounds.width as i16;
        let h = surface.bounds.height as i16;

        let effective_intensity = match self.pulse_frequency_hz {
            Some(freq) => {
                let osc = (elapsed_secs * freq * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                self.intensity * (0.6 + 0.4 * osc)
            }
            None => self.intensity,
        };

        let bx0 = content_bounds.x as f32;
        let by0 = content_bounds.y as f32;
        let bx1 = (content_bounds.x + content_bounds.width) as f32;
        let by1 = (content_bounds.y + content_bounds.height) as f32;

        let max_r = self.radius.max(1.0);

        for y in 0..h {
            for x in 0..w {
                let fx = x as f32;
                let fy = y as f32;

                // Inside the content box, do not mutate
                if fx >= bx0 && fx < bx1 && fy >= by0 && fy < by1 {
                    continue;
                }

                let dx = if fx < bx0 {
                    bx0 - fx
                } else if fx >= bx1 {
                    fx - (bx1 - 1.0)
                } else {
                    0.0
                };

                let dy = if fy < by0 {
                    by0 - fy
                } else if fy >= by1 {
                    fy - (by1 - 1.0)
                } else {
                    0.0
                };

                let dk = (dx * dx + (k * dy) * (k * dy)).sqrt();

                if dk <= max_r * 2.0 {
                    let alpha = (effective_intensity * (-dk / max_r).exp()).clamp(0.0, 1.0);
                    if alpha > 0.01
                        && let Some(cell) = surface.get_cell_mut(x as u16, y as u16)
                    {
                        cell.bg = cell.bg.lerp(self.color, alpha);
                    }
                }
            }
        }
    }
}
