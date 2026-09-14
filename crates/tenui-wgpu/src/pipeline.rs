use tenui_core::{buffer::Buffer, color::Color};

pub const TERMINAL_WGSL_SHADER: &str = include_str!("shaders/terminal.wgsl");

#[repr(C)]
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct CellInstance {
    pub grid_pos: [f32; 2],     // (col, row) integer coordinates
    pub fg_color: [f32; 4],     // Linear sRGB foreground color with alpha
    pub bg_color: [f32; 4],     // Linear sRGB background color
    pub glyph_uv_min: [f32; 2], // Texture atlas UV coordinates
    pub glyph_uv_max: [f32; 2],
    pub attributes: u32, // Bitfield: Bold, Underline, Inverted
}

pub struct GpuPipelineConfig {
    pub grid_cols: u16,
    pub grid_rows: u16,
    pub enable_crt_scanlines: bool,
    pub cell_pixel_size: (u32, u32),
}

impl Default for GpuPipelineConfig {
    fn default() -> Self {
        Self {
            grid_cols: 100,
            grid_rows: 32,
            enable_crt_scanlines: true,
            cell_pixel_size: (10, 20),
        }
    }
}

pub struct WgpuCanvasRunner {
    pub config: GpuPipelineConfig,
    pub instance_buffer: Vec<CellInstance>,
}

impl WgpuCanvasRunner {
    pub fn new(config: GpuPipelineConfig) -> Self {
        let capacity = (config.grid_cols as usize) * (config.grid_rows as usize);
        Self {
            config,
            instance_buffer: Vec::with_capacity(capacity),
        }
    }

    /// Converts a Tenui Cell Buffer into an array of CellInstance primitives for GPU upload.
    pub fn update_from_buffer(&mut self, buffer: &Buffer) {
        self.instance_buffer.clear();
        let cols = buffer.width;
        let rows = buffer.height;

        for y in 0..rows {
            for x in 0..cols {
                if let Some(cell) = buffer.get(x, y) {
                    let fg = Self::color_to_rgba(cell.fg);
                    let bg = Self::color_to_rgba(cell.bg);
                    let attrs = cell.modifier.bits() as u32;

                    self.instance_buffer.push(CellInstance {
                        grid_pos: [x as f32, y as f32],
                        fg_color: fg,
                        bg_color: bg,
                        glyph_uv_min: [0.0, 0.0],
                        glyph_uv_max: [1.0, 1.0],
                        attributes: attrs,
                    });
                }
            }
        }
    }

    fn color_to_rgba(c: Color) -> [f32; 4] {
        if c == Color::Reset {
            [0.0, 0.0, 0.0, 0.0]
        } else {
            let (r, g, b) = c.to_rgb();
            [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0]
        }
    }
}

impl Default for WgpuCanvasRunner {
    fn default() -> Self {
        Self::new(GpuPipelineConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shader_compiles_and_contains_entry_points() {
        assert!(TERMINAL_WGSL_SHADER.contains("fn vs_main"));
        assert!(TERMINAL_WGSL_SHADER.contains("fn fs_main"));
        assert!(TERMINAL_WGSL_SHADER.contains("struct InstanceInput"));
    }

    #[test]
    fn test_buffer_to_instance_conversion() {
        use tenui_core::Modifier;
        let mut buffer = Buffer::new(10, 5);
        buffer.set_string(0, 0, "Tenui", Color::Cyan, Color::Black, Modifier::empty());

        let mut runner = WgpuCanvasRunner::new(GpuPipelineConfig {
            grid_cols: 10,
            grid_rows: 5,
            ..Default::default()
        });

        runner.update_from_buffer(&buffer);
        assert_eq!(runner.instance_buffer.len(), 50);

        // First cell should have cyan fg
        let first = runner.instance_buffer[0];
        assert_eq!(first.grid_pos, [0.0, 0.0]);
        let (r, g, b) = Color::Cyan.to_rgb();
        let expected_fg = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
        assert_eq!(first.fg_color, expected_fg);
    }
}
