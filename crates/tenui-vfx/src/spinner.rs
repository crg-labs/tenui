use std::f32::consts::{PI, TAU};

use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Continuous sub-pixel vector arc spinner rasterized into Braille or Half-Block matrix planes.
#[derive(Clone, Debug)]
pub struct VectorArcSpinner {
    pub radius: f32,
    pub speed: f32,      // Rotations per second
    pub arc_length: f32, // Length in radians (e.g. 1.5 * PI)
    pub color: Color,
}

impl VectorArcSpinner {
    pub const BRAILLE_DOUBLE_ARC: Self = Self {
        radius: 1.6,
        speed: 1.25,
        arc_length: 1.5 * PI,
        color: Color::CYAN,
    };

    pub fn new(radius: f32, color: Color) -> Self {
        Self {
            radius,
            speed: 1.25,
            arc_length: 1.5 * PI,
            color,
        }
    }

    pub fn paint(&self, surface: &mut CanvasSubviewMut, elapsed: std::time::Duration, color: Color) {
        let mut s = self.clone();
        s.color = color;
        s.render_arc(surface, elapsed.as_secs_f32());
    }

    /// Computes the exact phase angle at a given epoch timestamp.
    pub fn phase_at(&self, elapsed_secs: f32) -> f32 {
        (elapsed_secs * self.speed * TAU).rem_euclid(TAU)
    }

    /// Renders a continuous sub-pixel rotational arc over a Braille plane.
    pub fn render_arc(&self, surface: &mut CanvasSubviewMut, elapsed_secs: f32) {
        let k = 2.0f32; // Cell aspect ratio factor
        let center_x = surface.bounds.width as f32 / 2.0;
        let center_y = surface.bounds.height as f32 / 2.0;

        let theta_start = self.phase_at(elapsed_secs);

        let steps = 64;
        let step_size = self.arc_length / steps as f32;

        for i in 0..=steps {
            let phi = theta_start + (i as f32 * step_size);
            let px = center_x + self.radius * phi.cos();
            let py = center_y + (self.radius / k) * phi.sin();

            let col = px.floor() as i32;
            let row = py.floor() as i32;

            if col >= 0 && col < surface.bounds.width as i32 && row >= 0 && row < surface.bounds.height as i32 {
                let frac_x = px - px.floor();
                let frac_y = py - py.floor();

                let dot_x = if frac_x >= 0.5 { 1 } else { 0 };
                let dot_y = (frac_y * 4.0).floor().clamp(0.0, 3.0) as usize;

                let bit = match (dot_x, dot_y) {
                    (0, 0) => 0x01,
                    (0, 1) => 0x02,
                    (0, 2) => 0x04,
                    (1, 0) => 0x08,
                    (1, 1) => 0x10,
                    (1, 2) => 0x20,
                    (0, 3) => 0x40,
                    (1, 3) => 0x80,
                    _ => 0x00,
                };

                if let Some(cell) = surface.get_cell_mut(col as u16, row as u16) {
                    cell.merge_braille_dot(bit);
                    cell.fg = self.color;
                }
            }
        }
    }
}
