use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Individual particle state with analytical kinematic motion.
#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub ax: f32,
    pub ay: f32,
    pub life: f32,
    pub max_life: f32,
    pub color: Color,
    pub active: bool,
}

impl Default for Particle {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            vx: 0.0,
            vy: 0.0,
            ax: 0.0,
            ay: 0.0,
            life: 0.0,
            max_life: 1.0,
            color: Color::WHITE,
            active: false,
        }
    }
}

/// Zero-allocation, stack-allocated pool of 256 sub-cell particles.
pub struct ParticlePool {
    pub particles: [Particle; 256],
    pub count: usize,
}

impl Default for ParticlePool {
    fn default() -> Self {
        Self {
            particles: [Particle::default(); 256],
            count: 0,
        }
    }
}

impl ParticlePool {
    pub fn new() -> Self {
        Self::default()
    }

    /// Spawns a new particle into the pool if capacity allows.
    // Position, velocity, acceleration, lifetime and color are distinct physics inputs;
    // bundling them into a struct would only move the argument list, not reduce it.
    #[allow(clippy::too_many_arguments)]
    pub fn spawn(&mut self, x: f32, y: f32, vx: f32, vy: f32, ax: f32, ay: f32, max_life: f32, color: Color) -> bool {
        for p in self.particles.iter_mut() {
            if !p.active {
                p.x = x;
                p.y = y;
                p.vx = vx;
                p.vy = vy;
                p.ax = ax;
                p.ay = ay;
                p.life = 0.0;
                p.max_life = max_life.max(0.01);
                p.color = color;
                p.active = true;
                self.count += 1;
                return true;
            }
        }
        false
    }

    pub fn spawn_burst(&mut self, x: f32, y: f32, vx: f32, vy: f32, color: Color, max_life: f32) -> bool {
        self.spawn(x, y, vx, vy, 0.0, 0.0, max_life, color)
    }

    /// Advances particle dynamics analytically by delta time dt.
    pub fn update(&mut self, dt: f32) {
        self.count = 0;
        for p in self.particles.iter_mut() {
            if p.active {
                p.life += dt;
                if p.life >= p.max_life {
                    p.active = false;
                    continue;
                }

                // Analytical kinematic integration: p(t) = p0 + v0*t + 0.5*a*t^2
                p.x += p.vx * dt + 0.5 * p.ax * dt * dt;
                p.y += p.vy * dt + 0.5 * p.ay * dt * dt;
                p.vx += p.ax * dt;
                p.vy += p.ay * dt;

                self.count += 1;
            }
        }
    }

    pub fn step(&mut self, dt: f32) {
        self.update(dt);
    }

    pub fn render(&self, surface: &mut CanvasSubviewMut, braille: bool) {
        if braille {
            self.render_braille(surface);
        } else {
            self.render_halfblock(surface);
        }
    }

    /// Renders active particles onto the surface (defaulting to Braille plane).
    pub fn paint(&self, surface: &mut CanvasSubviewMut) {
        self.render_braille(surface);
    }
}

/// ParticleEmitter alias representing the zero-allocation particle system.
pub type ParticleEmitter = ParticlePool;

impl ParticlePool {
    /// Rasterizes particles over a 2x4 Braille matrix plane via bitwise OR.
    pub fn render_braille(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width as i32;
        let h = surface.bounds.height as i32;

        for p in self.particles.iter() {
            if !p.active {
                continue;
            }

            let col = p.x.floor() as i32;
            let row = p.y.floor() as i32;

            if col >= 0 && col < w && row >= 0 && row < h {
                let frac_x = p.x - p.x.floor();
                let frac_y = p.y - p.y.floor();

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

                let alpha = (1.0 - (p.life / p.max_life)).clamp(0.0, 1.0);
                if let Some(cell) = surface.get_cell_mut(col as u16, row as u16) {
                    cell.merge_braille_dot(bit);
                    cell.fg = p.color.with_alpha(alpha);
                }
            }
        }
    }

    /// Rasterizes particles over a 1x2 Half-Block subpixel plane with alpha fade.
    pub fn render_halfblock(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width as i32;
        let h = surface.bounds.height as i32;

        for p in self.particles.iter() {
            if !p.active {
                continue;
            }

            let col = p.x.floor() as i32;
            let row = p.y.floor() as i32;

            if col >= 0 && col < w && row >= 0 && row < h {
                let frac_y = p.y - p.y.floor();
                let alpha = (1.0 - (p.life / p.max_life)).clamp(0.0, 1.0);
                let fade_color = p.color.with_alpha(alpha);

                if let Some(cell) = surface.get_cell_mut(col as u16, row as u16) {
                    if frac_y < 0.5 {
                        // Upper half block
                        cell.symbol = tenui_core::cell::CompactSymbol::from_char('▀');
                        cell.fg = fade_color;
                    } else {
                        // Lower half block
                        cell.symbol = tenui_core::cell::CompactSymbol::from_char('▄');
                        cell.fg = fade_color;
                    }
                }
            }
        }
    }
}
