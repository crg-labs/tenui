use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Optical backdrop filter for frosted glass and acrylic effects (Pass A).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BackdropFilter {
    /// Dimming coefficient (0.0 = no dimming, 1.0 = fully dark)
    pub dim_opacity: f32,
    /// Desaturation factor (0.0 = full color, 1.0 = full grayscale)
    pub desaturation: f32,
    /// Diffusion intensity for glyph blur approximation (0.0 = keep text, 1.0 = heavy stipple)
    pub diffusion_intensity: f32,
    /// Contrast edge threshold to trigger glyph diffusion
    pub edge_threshold: f32,
    /// Optional subtle acrylic tint color
    pub tint_color: Option<Color>,
}

impl BackdropFilter {
    /// Standard frosted acrylic preset
    pub const ACRYLIC: Self = Self {
        dim_opacity: 0.40,
        desaturation: 0.60,
        diffusion_intensity: 0.70,
        edge_threshold: 0.15,
        tint_color: Some(Color::Rgb(15, 23, 42)),
    };

    /// Subtle dim preset without diffusion
    pub const SUBTLE_DIM: Self = Self {
        dim_opacity: 0.25,
        desaturation: 0.20,
        diffusion_intensity: 0.0,
        edge_threshold: 0.50,
        tint_color: None,
    };

    /// Heavy frosted blur preset for high-contrast modals
    pub const HEAVY_FROST: Self = Self {
        dim_opacity: 0.60,
        desaturation: 0.85,
        diffusion_intensity: 0.90,
        edge_threshold: 0.10,
        tint_color: Some(Color::Rgb(10, 15, 30)),
    };

    /// Applies the optical filter directly over an intercepted back-buffer slice.
    pub fn apply(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;

        for y in 0..h {
            for x in 0..w {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    // 1. Process Background Color
                    cell.bg = self.transform_color(cell.bg);

                    // 2. Process Foreground Color
                    let orig_fg = cell.fg;
                    cell.fg = self.transform_color(cell.fg);

                    // 3. Glyph Diffusion (Terminal Blur Approximation)
                    if self.diffusion_intensity > 0.01 && cell.symbol.as_str() != " " && !cell.symbol.is_empty() {
                        let contrast = self.compute_contrast(orig_fg, cell.bg);
                        if contrast > self.edge_threshold {
                            cell.symbol = tenui_core::cell::CompactSymbol::from_char('░');
                            // Tone down diffused glyph foreground
                            cell.fg = cell.bg.lerp(cell.fg, 0.4);
                        }
                    }
                }
            }
        }
    }

    /// Transforms an RGB color through luminance attenuation, desaturation, and acrylic tint.
    fn transform_color(&self, color: Color) -> Color {
        if color == Color::Reset {
            return color;
        }
        let (r_u8, g_u8, b_u8) = color.to_rgb();
        let (r, g, b) = (r_u8 as f32, g_u8 as f32, b_u8 as f32);

        // 1. ITU-R BT.709 Relative Luminance Y
        let y_lum = 0.2126 * r + 0.7152 * g + 0.0722 * b;

        // 2. Perceptual Desaturation
        let r_desat = r * (1.0 - self.desaturation) + y_lum * self.desaturation;
        let g_desat = g * (1.0 - self.desaturation) + y_lum * self.desaturation;
        let b_desat = b * (1.0 - self.desaturation) + y_lum * self.desaturation;

        // 3. Luminance Attenuation
        let dim_factor = 1.0 - self.dim_opacity.clamp(0.0, 1.0);
        let r_final = (r_desat * dim_factor).clamp(0.0, 255.0) as u8;
        let g_final = (g_desat * dim_factor).clamp(0.0, 255.0) as u8;
        let b_final = (b_desat * dim_factor).clamp(0.0, 255.0) as u8;

        // 4. Acrylic Tint
        if let Some(tint) = self.tint_color {
            let processed = Color::Rgb(r_final, g_final, b_final);
            return processed.lerp(tint, 0.35);
        }

        Color::Rgb(r_final, g_final, b_final)
    }

    /// Computes perceptual contrast between two colors [0.0..1.0]
    fn compute_contrast(&self, c1: Color, c2: Color) -> f32 {
        let y1 = c1.relative_luminance();
        let y2 = c2.relative_luminance();
        (y1 - y2).abs()
    }
}

/// Precision optical acrylic filter for frosted glass modals and floating overlays.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct AcrylicFilter {
    pub dim_factor: f32,       // e.g., 0.40
    pub desaturation: f32,     // e.g., 0.60
    pub diffuse_stipple: bool, // e.g., true -> '░'
}

impl Default for AcrylicFilter {
    fn default() -> Self {
        Self {
            dim_factor: 0.40,
            desaturation: 0.60,
            diffuse_stipple: true,
        }
    }
}

impl AcrylicFilter {
    pub const STANDARD: Self = Self {
        dim_factor: 0.40,
        desaturation: 0.60,
        diffuse_stipple: true,
    };

    pub fn new(dim_factor: f32, desaturation: f32, diffuse_stipple: bool) -> Self {
        Self {
            dim_factor,
            desaturation,
            diffuse_stipple,
        }
    }

    /// Applies the optical acrylic transform directly over an intercepted back-buffer slice.
    pub fn apply(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;

        for y in 0..h {
            for x in 0..w {
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    // 1. Attenuate and desaturate background
                    let (bg_r, bg_g, bg_b) = cell.bg.to_rgb();
                    let dimmed_bg_r = bg_r as f32 * (1.0 - self.dim_factor);
                    let dimmed_bg_g = bg_g as f32 * (1.0 - self.dim_factor);
                    let dimmed_bg_b = bg_b as f32 * (1.0 - self.dim_factor);
                    let lum_bg = 0.2126 * dimmed_bg_r + 0.7152 * dimmed_bg_g + 0.0722 * dimmed_bg_b;
                    let desat_bg_r =
                        (dimmed_bg_r * (1.0 - self.desaturation) + lum_bg * self.desaturation).clamp(0.0, 255.0) as u8;
                    let desat_bg_g =
                        (dimmed_bg_g * (1.0 - self.desaturation) + lum_bg * self.desaturation).clamp(0.0, 255.0) as u8;
                    let desat_bg_b =
                        (dimmed_bg_b * (1.0 - self.desaturation) + lum_bg * self.desaturation).clamp(0.0, 255.0) as u8;
                    cell.bg = Color::Rgb(desat_bg_r, desat_bg_g, desat_bg_b);

                    // 2. Attenuate and desaturate foreground
                    let (fg_r, fg_g, fg_b) = cell.fg.to_rgb();
                    let dimmed_fg_r = fg_r as f32 * (1.0 - self.dim_factor);
                    let dimmed_fg_g = fg_g as f32 * (1.0 - self.dim_factor);
                    let dimmed_fg_b = fg_b as f32 * (1.0 - self.dim_factor);
                    let lum_fg = 0.2126 * dimmed_fg_r + 0.7152 * dimmed_fg_g + 0.0722 * dimmed_fg_b;
                    let desat_fg_r =
                        (dimmed_fg_r * (1.0 - self.desaturation) + lum_fg * self.desaturation).clamp(0.0, 255.0) as u8;
                    let desat_fg_g =
                        (dimmed_fg_g * (1.0 - self.desaturation) + lum_fg * self.desaturation).clamp(0.0, 255.0) as u8;
                    let desat_fg_b =
                        (dimmed_fg_b * (1.0 - self.desaturation) + lum_fg * self.desaturation).clamp(0.0, 255.0) as u8;
                    cell.fg = Color::Rgb(desat_fg_r, desat_fg_g, desat_fg_b);

                    // 3. Diffuse sharp characters into stipple masks
                    if self.diffuse_stipple && cell.symbol.as_str() != " " && !cell.symbol.is_empty() {
                        cell.symbol = tenui_core::cell::CompactSymbol::from_char('░');
                    }
                }
            }
        }
    }
}
