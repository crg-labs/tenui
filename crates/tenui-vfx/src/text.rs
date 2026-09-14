use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Optical typography effects for monospaced character grids.
#[derive(Clone, Debug, PartialEq)]
pub enum TextEffect {
    /// Plain unadorned text
    Plain,
    /// Animated Gaussian beam luminance shimmer wave sweeping across glyphs
    Shimmer { speed: f32, peak_color: Color },
    /// Deterministic pseudo-hash glitch decryption from high-entropy noise to plaintext
    GlitchDecryption { progress: f32, seed: u64 },
    /// Kinetic vertical floating wave using 1/8th sub-cell baseline offsets
    KineticWave { amplitude: f32, frequency: f32, speed: f32 },
    /// Per-character directional bloom shadow cast beneath glyphs
    DirectionalBloom { shadow_color: Color },
    /// Per-glyph radial glow that bleeds into neighboring cells' backgrounds.
    Glow {
        glow_color: Color,
        radius: f32,
        intensity: f32,
    },
}

pub struct TextOpticsCompositor;

impl TextOpticsCompositor {
    /// Renders text with advanced per-character optical effects.
    pub fn render_text(
        surface: &mut CanvasSubviewMut,
        local_x: u16,
        local_y: u16,
        text: &str,
        base_color: Color,
        effect: TextEffect,
        elapsed_secs: f32,
    ) {
        let chars: Vec<char> = text.chars().collect();
        let len = chars.len();
        if len == 0 || local_y >= surface.bounds.height {
            return;
        }

        match effect {
            TextEffect::Plain => {
                surface.write_str_clipped(local_x, local_y, text, base_color, Color::TRANSPARENT);
            }
            TextEffect::Shimmer { speed, peak_color } => {
                for (i, &ch) in chars.iter().enumerate() {
                    let u = i as f32 / len.max(1) as f32;
                    let phase = (u - elapsed_secs * speed).rem_euclid(1.0);
                    // Gaussian highlight beam with sharp peak
                    let intensity = (-((phase - 0.5) * (phase - 0.5)) / 0.04).exp();
                    let glyph_color = base_color.lerp(peak_color, intensity);
                    surface.set_cell_styled(local_x + i as u16, local_y, ch, glyph_color, Color::TRANSPARENT);
                }
            }
            TextEffect::GlitchDecryption { progress, seed } => {
                const NOISE_CHARS: &[char] = &[
                    '#', '@', '$', '%', '&', '*', '!', '?', '░', '▒', '1', '0', 'Δ', 'Ω', '§',
                ];
                let clamped_progress = progress.clamp(0.0, 1.0);

                for (i, &ch) in chars.iter().enumerate() {
                    let rand_val = Self::pseudo_hash(i as u64, seed);
                    let threshold = rand_val as f32 / u32::MAX as f32;

                    if clamped_progress >= threshold {
                        // Plaintext locked
                        surface.set_cell_styled(local_x + i as u16, local_y, ch, base_color, Color::TRANSPARENT);
                    } else if clamped_progress >= threshold - 0.08 {
                        // Peak energy flare block
                        surface.set_cell_styled(local_x + i as u16, local_y, '█', Color::WHITE, Color::TRANSPARENT);
                    } else {
                        // High-entropy noise character
                        let noise_idx = (rand_val as usize + (elapsed_secs * 30.0) as usize) % NOISE_CHARS.len();
                        let noise_char = NOISE_CHARS[noise_idx];
                        let muted_color = base_color.dim(0.5);
                        surface.set_cell_styled(
                            local_x + i as u16,
                            local_y,
                            noise_char,
                            muted_color,
                            Color::TRANSPARENT,
                        );
                    }
                }
            }
            TextEffect::KineticWave {
                amplitude,
                frequency,
                speed,
            } => {
                const BLOCKS: &[char] = &[' ', ' ', '▂', '▃', '▄', '▅', '▆', '▇'];
                for (i, &ch) in chars.iter().enumerate() {
                    let wave = ((i as f32 * frequency) + (elapsed_secs * speed)).sin();
                    let norm = ((wave * amplitude).clamp(-1.0, 1.0) + 1.0) * 0.5;
                    let block_idx = (norm * (BLOCKS.len() - 1) as f32).round() as usize;

                    // If at baseline, render standard character; otherwise modulate with sub-cell glyph
                    if norm > 0.8 && local_y > 0 {
                        surface.set_cell_styled(
                            local_x + i as u16,
                            local_y - 1,
                            BLOCKS[block_idx],
                            base_color,
                            Color::TRANSPARENT,
                        );
                    }
                    surface.set_cell_styled(local_x + i as u16, local_y, ch, base_color, Color::TRANSPARENT);
                }
            }
            TextEffect::DirectionalBloom { shadow_color } => {
                // First pass: project subtle shadow one cell below
                if local_y + 1 < surface.bounds.height {
                    for (i, _) in chars.iter().enumerate() {
                        if let Some(cell) = surface.get_cell_mut(local_x + i as u16, local_y + 1) {
                            cell.bg = cell.bg.lerp(shadow_color, 0.45);
                        }
                    }
                }
                // Second pass: render text
                surface.write_str_clipped(local_x, local_y, text, base_color, Color::TRANSPARENT);
            }
            TextEffect::Glow {
                glow_color,
                radius,
                intensity,
            } => {
                let k = 2.0f32; // cell aspect ratio correction (columns ~half the height of rows)
                let sigma = radius.max(0.5);
                let max_reach = (radius * 2.5).ceil() as i16;
                let sw = surface.bounds.width as i16;
                let sh = surface.bounds.height as i16;

                // Pass 1: radial glow bleed into neighboring background cells per glyph.
                for (i, _) in chars.iter().enumerate() {
                    let gx = local_x as i16 + i as i16;
                    let gy = local_y as i16;

                    for dy in -max_reach..=max_reach {
                        for dx in -max_reach..=max_reach {
                            let nx = gx + dx;
                            let ny = gy + dy;
                            if nx < 0 || ny < 0 || nx >= sw || ny >= sh {
                                continue;
                            }
                            if dx == 0 && dy == 0 {
                                continue;
                            }
                            let dist = ((dx as f32) * (dx as f32) + (k * dy as f32) * (k * dy as f32)).sqrt();
                            if dist > radius * 2.5 {
                                continue;
                            }
                            let alpha = intensity * (-(dist * dist) / (2.0 * sigma * sigma)).exp();
                            if alpha < 0.01 {
                                continue;
                            }
                            if let Some(cell) = surface.get_cell_mut(nx as u16, ny as u16) {
                                cell.bg = cell.bg.lerp(glow_color, alpha.clamp(0.0, 1.0));
                            }
                        }
                    }
                }

                // Pass 2: render glyphs with brightened foreground.
                let bright_fg = base_color.lerp(glow_color, 0.3);
                for (i, &ch) in chars.iter().enumerate() {
                    let cx = local_x + i as u16;
                    if cx < surface.bounds.width {
                        if let Some(cell) = surface.get_cell_mut(cx, local_y) {
                            cell.bg = cell.bg.lerp(glow_color, intensity * 0.4);
                        }
                        surface.set_cell_styled(cx, local_y, ch, bright_fg, Color::TRANSPARENT);
                    }
                }
            }
        }
    }

    /// Fast 64-bit to 32-bit analytical hash function for deterministic pseudorandomness.
    pub fn pseudo_hash(idx: u64, seed: u64) -> u32 {
        let mut x = idx.wrapping_mul(0x9E3779B97F4A7C15).wrapping_add(seed);
        x = (x ^ (x >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
        x = (x ^ (x >> 27)).wrapping_mul(0x94D049BB133111EB);
        (x ^ (x >> 31)) as u32
    }
}
