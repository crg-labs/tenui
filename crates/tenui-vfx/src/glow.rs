use tenui_core::{canvas::CanvasSubviewMut, color::Color, geometry::Rect};

/// Falloff curve controlling how glow intensity decays with distance.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum GlowFalloff {
    /// Gaussian: `exp(-d² / (2σ²))` — soft, natural light bleed.
    Gaussian,
    /// Exponential: `exp(-d / σ)` — sharper falloff, tighter halo.
    Exponential,
    /// Linear: `max(0, 1 - d / radius)` — even gradient to hard cutoff.
    Linear,
}

/// A spatial glow effect that radiates colored light outward from, inward into,
/// or both directions around a rectangular region.
///
/// Operates on the same `CanvasSubviewMut` + `Rect` pattern as [`DropShadow`](super::DropShadow)
/// and [`FocusHalo`](super::FocusHalo), blending glow color into cell backgrounds
/// via `Color::lerp`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct GlowEffect {
    pub color: Color,
    pub outer_radius: f32,
    pub inner_radius: f32,
    pub intensity: f32,
    pub falloff: GlowFalloff,
    pub pulse_hz: Option<f32>,
}

impl GlowEffect {
    pub const NEON_CYAN: Self = Self {
        color: Color::Rgb(0, 255, 255),
        outer_radius: 3.0,
        inner_radius: 0.0,
        intensity: 0.6,
        falloff: GlowFalloff::Gaussian,
        pulse_hz: None,
    };

    pub const NEON_MAGENTA: Self = Self {
        color: Color::Rgb(255, 0, 255),
        outer_radius: 3.0,
        inner_radius: 0.0,
        intensity: 0.6,
        falloff: GlowFalloff::Gaussian,
        pulse_hz: None,
    };

    pub const SOFT_GOLD: Self = Self {
        color: Color::Rgb(255, 200, 50),
        outer_radius: 2.0,
        inner_radius: 1.0,
        intensity: 0.4,
        falloff: GlowFalloff::Gaussian,
        pulse_hz: None,
    };

    pub const PULSE_GREEN: Self = Self {
        color: Color::Rgb(0, 255, 128),
        outer_radius: 2.5,
        inner_radius: 0.0,
        intensity: 0.55,
        falloff: GlowFalloff::Exponential,
        pulse_hz: Some(1.2),
    };

    pub const DANGER_RED: Self = Self {
        color: Color::Rgb(255, 50, 50),
        outer_radius: 2.0,
        inner_radius: 0.5,
        intensity: 0.5,
        falloff: GlowFalloff::Exponential,
        pulse_hz: Some(2.0),
    };

    pub fn new(color: Color) -> Self {
        Self {
            color,
            outer_radius: 2.5,
            inner_radius: 0.0,
            intensity: 0.5,
            falloff: GlowFalloff::Gaussian,
            pulse_hz: None,
        }
    }

    pub fn with_outer_radius(mut self, r: f32) -> Self {
        self.outer_radius = r;
        self
    }

    pub fn with_inner_radius(mut self, r: f32) -> Self {
        self.inner_radius = r;
        self
    }

    pub fn with_intensity(mut self, i: f32) -> Self {
        self.intensity = i;
        self
    }

    pub fn with_falloff(mut self, f: GlowFalloff) -> Self {
        self.falloff = f;
        self
    }

    pub fn with_pulse(mut self, hz: f32) -> Self {
        self.pulse_hz = Some(hz);
        self
    }

    /// Renders the glow into cells surrounding and/or within `content_bounds`.
    ///
    /// The cell aspect ratio (columns are ~half the height of rows) is corrected
    /// so the glow appears visually circular, matching the convention in
    /// [`DropShadow`](super::DropShadow) and [`FocusHalo`](super::FocusHalo).
    pub fn render(&self, surface: &mut CanvasSubviewMut, content_bounds: Rect, elapsed_secs: f32) {
        let k = 2.0f32; // cell aspect ratio correction
        let w = surface.bounds.width;
        let h = surface.bounds.height;

        let effective_intensity = match self.pulse_hz {
            Some(freq) => {
                let osc = (elapsed_secs * freq * std::f32::consts::TAU).sin() * 0.5 + 0.5;
                self.intensity * (0.5 + 0.5 * osc)
            }
            None => self.intensity,
        };

        let bx0 = content_bounds.x as f32;
        let by0 = content_bounds.y as f32;
        let bx1 = (content_bounds.x + content_bounds.width) as f32;
        let by1 = (content_bounds.y + content_bounds.height) as f32;

        let outer_max = self.outer_radius.max(0.0);
        let inner_max = self.inner_radius.max(0.0);

        for y in 0..h {
            for x in 0..w {
                let fx = x as f32;
                let fy = y as f32;

                let inside = fx >= bx0 && fx < bx1 && fy >= by0 && fy < by1;

                // Signed distance to the content rectangle boundary (negative = inside).
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

                let outer_dist = (dx * dx + (k * dy) * (k * dy)).sqrt();

                let alpha = if !inside && outer_max > 0.0 {
                    if outer_dist > outer_max * 2.5 {
                        continue;
                    }
                    effective_intensity * self.falloff_factor(outer_dist, outer_max)
                } else if inside && inner_max > 0.0 {
                    let edge_dx = (fx - bx0).min(bx1 - 1.0 - fx).max(0.0);
                    let edge_dy = (fy - by0).min(by1 - 1.0 - fy).max(0.0);
                    let inner_dist = (edge_dx * edge_dx + (k * edge_dy) * (k * edge_dy)).sqrt();
                    if inner_dist > inner_max * 2.5 {
                        continue;
                    }
                    effective_intensity * self.falloff_factor(inner_dist, inner_max)
                } else {
                    continue;
                };

                let alpha = alpha.clamp(0.0, 1.0);
                if alpha < 0.01 {
                    continue;
                }

                if let Some(cell) = surface.get_cell_mut(x, y) {
                    cell.bg = cell.bg.lerp(self.color, alpha);
                    if inside {
                        cell.fg = cell.fg.lerp(self.color, alpha * 0.3);
                    }
                }
            }
        }
    }

    fn falloff_factor(&self, distance: f32, radius: f32) -> f32 {
        let sigma = radius.max(0.5);
        match self.falloff {
            GlowFalloff::Gaussian => (-(distance * distance) / (2.0 * sigma * sigma)).exp(),
            GlowFalloff::Exponential => (-distance / sigma).exp(),
            GlowFalloff::Linear => (1.0 - distance / (sigma * 2.5)).max(0.0),
        }
    }
}

/// Multi-layer bloom compositor that stacks several glow passes for richer lighting.
#[derive(Clone, Debug, PartialEq)]
pub struct BloomStack {
    pub layers: Vec<GlowEffect>,
}

impl BloomStack {
    pub fn new() -> Self {
        Self { layers: Vec::new() }
    }

    pub fn push(mut self, glow: GlowEffect) -> Self {
        self.layers.push(glow);
        self
    }

    /// Renders all glow layers in order.
    pub fn render(&self, surface: &mut CanvasSubviewMut, content_bounds: Rect, elapsed_secs: f32) {
        for layer in &self.layers {
            layer.render(surface, content_bounds, elapsed_secs);
        }
    }

    /// A preset neon bloom: a tight bright core halo plus a wider diffuse outer glow.
    pub fn neon(color: Color) -> Self {
        let (r, g, b) = color.to_rgb();
        let bright = Color::Rgb(r.saturating_add(60), g.saturating_add(60), b.saturating_add(60));
        Self::new()
            .push(
                GlowEffect::new(bright)
                    .with_outer_radius(1.5)
                    .with_intensity(0.7)
                    .with_falloff(GlowFalloff::Gaussian),
            )
            .push(
                GlowEffect::new(color)
                    .with_outer_radius(3.5)
                    .with_intensity(0.35)
                    .with_falloff(GlowFalloff::Gaussian),
            )
    }
}

impl Default for BloomStack {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::buffer::Buffer;

    use super::*;

    fn make_surface(w: u16, h: u16, bg: Color) -> Buffer {
        let mut buf = Buffer::new(w, h);
        for y in 0..h {
            for x in 0..w {
                if let Some(cell) = buf.get_mut(x, y) {
                    cell.bg = bg;
                }
            }
        }
        buf
    }

    #[test]
    fn outer_glow_modifies_surrounding_cells() {
        let mut buf = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let content = Rect {
            x: 5,
            y: 3,
            width: 10,
            height: 4,
        };
        let glow = GlowEffect::NEON_CYAN;
        {
            let bounds = Rect {
                x: 0,
                y: 0,
                width: 20,
                height: 10,
            };
            let mut sv = buf.subview_mut(bounds);
            glow.render(&mut sv, content, 0.0);
        }
        let outside = buf.get(3, 3).unwrap();
        assert_ne!(outside.bg, Color::Rgb(0, 0, 0));

        let far = buf.get(0, 0).unwrap();
        let near = buf.get(4, 3).unwrap();
        let (_, _, b_far) = far.bg.to_rgb();
        let (_, _, b_near) = near.bg.to_rgb();
        assert!(b_near >= b_far, "cells closer to the content should glow more");
    }

    #[test]
    fn inner_glow_modifies_interior_cells() {
        let mut buf = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let content = Rect {
            x: 2,
            y: 2,
            width: 16,
            height: 6,
        };
        let glow = GlowEffect::new(Color::Rgb(255, 100, 0))
            .with_inner_radius(2.0)
            .with_outer_radius(0.0)
            .with_intensity(0.8);
        {
            let bounds = Rect {
                x: 0,
                y: 0,
                width: 20,
                height: 10,
            };
            let mut sv = buf.subview_mut(bounds);
            glow.render(&mut sv, content, 0.0);
        }
        let edge_cell = buf.get(2, 2).unwrap();
        assert_ne!(edge_cell.bg, Color::Rgb(0, 0, 0));
    }

    #[test]
    fn pulse_varies_with_time() {
        let glow = GlowEffect::PULSE_GREEN;
        let mut buf_a = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let mut buf_b = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let content = Rect {
            x: 5,
            y: 3,
            width: 10,
            height: 4,
        };
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        {
            let mut sv = buf_a.subview_mut(bounds);
            glow.render(&mut sv, content, 0.0);
        }
        {
            let mut sv = buf_b.subview_mut(bounds);
            glow.render(&mut sv, content, 0.42);
        }
        let cell_a = buf_a.get(4, 3).unwrap();
        let cell_b = buf_b.get(4, 3).unwrap();
        assert_ne!(cell_a.bg, cell_b.bg);
    }

    #[test]
    fn bloom_stack_neon_creates_two_layers() {
        let bloom = BloomStack::neon(Color::Rgb(0, 200, 255));
        assert_eq!(bloom.layers.len(), 2);
    }

    #[test]
    fn falloff_variants_produce_different_results() {
        let content = Rect {
            x: 5,
            y: 3,
            width: 10,
            height: 4,
        };
        let bounds = Rect {
            x: 0,
            y: 0,
            width: 20,
            height: 10,
        };
        let base = GlowEffect::new(Color::Rgb(255, 0, 0))
            .with_outer_radius(3.0)
            .with_intensity(0.8);

        let mut buf_gauss = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let mut buf_exp = make_surface(20, 10, Color::Rgb(0, 0, 0));
        let mut buf_lin = make_surface(20, 10, Color::Rgb(0, 0, 0));

        {
            let mut sv = buf_gauss.subview_mut(bounds);
            base.with_falloff(GlowFalloff::Gaussian).render(&mut sv, content, 0.0);
        }
        {
            let mut sv = buf_exp.subview_mut(bounds);
            base.with_falloff(GlowFalloff::Exponential)
                .render(&mut sv, content, 0.0);
        }
        {
            let mut sv = buf_lin.subview_mut(bounds);
            base.with_falloff(GlowFalloff::Linear).render(&mut sv, content, 0.0);
        }

        let g = buf_gauss.get(3, 3).unwrap().bg;
        let e = buf_exp.get(3, 3).unwrap().bg;
        let l = buf_lin.get(3, 3).unwrap().bg;
        assert!(
            g != e || e != l,
            "different falloffs should produce different glow intensities"
        );
    }
}
