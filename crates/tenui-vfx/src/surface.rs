use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Volumetric surface shaders: inset neumorphic shadows, dithered gradients, and ambient textures.
pub struct SurfaceVolumetrics;

impl SurfaceVolumetrics {
    pub const BAYER_4X4: [[f32; 4]; 4] = [
        [0.0 / 16.0, 8.0 / 16.0, 2.0 / 16.0, 10.0 / 16.0],
        [12.0 / 16.0, 4.0 / 16.0, 14.0 / 16.0, 6.0 / 16.0],
        [3.0 / 16.0, 11.0 / 16.0, 1.0 / 16.0, 9.0 / 16.0],
        [15.0 / 16.0, 7.0 / 16.0, 13.0 / 16.0, 5.0 / 16.0],
    ];

    /// Inset / Sunken Neumorphic Shadow across internal container boundaries.
    pub fn render_inset_shadow(surface: &mut CanvasSubviewMut, depth_radius: f32, max_alpha: f32) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;
        let k = 2.0f32; // Aspect ratio correction

        for y in 0..h {
            for x in 0..w {
                let d_top = y as f32;
                let d_left = x as f32 / k;

                let alpha_top = (1.0 - d_top / depth_radius).max(0.0).powi(2);
                let alpha_left = (1.0 - d_left / depth_radius).max(0.0).powi(2);
                let total_alpha = ((alpha_top + alpha_left) * max_alpha).clamp(0.0, max_alpha);

                if total_alpha > 0.005
                    && let Some(cell) = surface.get_cell_mut(x, y)
                {
                    cell.bg = cell.bg.lerp(Color::BLACK, total_alpha);
                }
            }
        }
    }

    /// Aspect-ratio-corrected linear gradient fill with Bayer ordered dithering.
    pub fn fill_linear_gradient(surface: &mut CanvasSubviewMut, from_color: Color, to_color: Color, angle_rad: f32) {
        let w = surface.bounds.width as f32;
        let h = surface.bounds.height as f32;
        let k = 2.0f32;
        let cos_a = angle_rad.cos();
        let sin_a = angle_rad.sin() / k;
        let max_proj = (w * cos_a.abs()) + (h * k * sin_a.abs());

        for y in 0..surface.bounds.height {
            for x in 0..surface.bounds.width {
                let proj = (x as f32 * cos_a) + (y as f32 * k * sin_a);
                let t_clean = (proj / max_proj.max(1.0)).clamp(0.0, 1.0);

                // Modulate by Bayer matrix to prevent 256-color banding
                let dither_offset = (Self::BAYER_4X4[(x % 4) as usize][(y % 4) as usize] - 0.5) * 0.06;
                let t_dithered = (t_clean + dither_offset).clamp(0.0, 1.0);

                let color = from_color.lerp(to_color, t_dithered);
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    cell.bg = color;
                }
            }
        }
    }

    /// Aspect-ratio-corrected radial gradient fill with Bayer ordered dithering.
    pub fn fill_radial_gradient(
        surface: &mut CanvasSubviewMut,
        center_x: f32,
        center_y: f32,
        inner_color: Color,
        outer_color: Color,
        radius: f32,
    ) {
        let k = 2.0f32;
        let max_r = radius.max(1.0);

        for y in 0..surface.bounds.height {
            for x in 0..surface.bounds.width {
                let dx = x as f32 - center_x;
                let dy = y as f32 - center_y;
                let dist = (dx * dx + (k * dy) * (k * dy)).sqrt();

                let t_clean = (dist / max_r).clamp(0.0, 1.0);
                let dither_offset = (Self::BAYER_4X4[(x % 4) as usize][(y % 4) as usize] - 0.5) * 0.06;
                let t_dithered = (t_clean + dither_offset).clamp(0.0, 1.0);

                let color = inner_color.lerp(outer_color, t_dithered);
                if let Some(cell) = surface.get_cell_mut(x, y) {
                    cell.bg = color;
                }
            }
        }
    }
}
