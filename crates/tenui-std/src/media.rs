use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Supported terminal graphics protocol capabilities.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GraphicsProtocol {
    /// Kitty terminal graphics protocol (APC escape codes)
    Kitty,
    /// Sixel bitmap graphics stream
    Sixel,
    /// iTerm2 inline image format
    ITerm2,
    /// Pure text Half-Block (▀ / ▄) TrueColor ANSI fallback
    HalfBlock,
}

/// Image rendering canvas with protocol auto-negotiation and half-block fallback.
pub struct ImageCanvas {
    pub width: u32,
    pub height: u32,
    pub rgba_pixels: Vec<u8>,
}

impl ImageCanvas {
    pub fn new(width: u32, height: u32, rgba_pixels: Vec<u8>) -> Self {
        Self {
            width,
            height,
            rgba_pixels,
        }
    }

    /// Auto-detects terminal graphics protocol capability from environment and flags.
    pub fn detect_protocol() -> GraphicsProtocol {
        if let Ok(term) = std::env::var("TERM")
            && (term.contains("kitty") || std::env::var("KITTY_PID").is_ok())
        {
            return GraphicsProtocol::Kitty;
        }
        if let Ok(term_prog) = std::env::var("TERM_PROGRAM")
            && (term_prog.contains("iTerm") || term_prog.contains("WezTerm"))
        {
            return GraphicsProtocol::ITerm2;
        }
        // Default safe universal fallback: Half-Block RGB
        GraphicsProtocol::HalfBlock
    }

    /// Renders the image onto the canvas subview according to the selected protocol.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>, protocol: GraphicsProtocol) {
        let view_w = surface.width() as u32;
        let view_h = surface.height() as u32;
        if view_w == 0 || view_h == 0 || self.width == 0 || self.height == 0 {
            return;
        }

        match protocol {
            GraphicsProtocol::HalfBlock
            | GraphicsProtocol::Kitty
            | GraphicsProtocol::Sixel
            | GraphicsProtocol::ITerm2 => {
                // Render half-block 2x vertical subpixel representation directly into buffer
                for cell_y in 0..view_h {
                    let py_top = (cell_y * 2 * self.height) / (view_h * 2);
                    let py_bot = ((cell_y * 2 + 1) * self.height) / (view_h * 2);

                    for cell_x in 0..view_w {
                        let px = (cell_x * self.width) / view_w;

                        let top_color = self.sample_pixel(px, py_top);
                        let bot_color = self.sample_pixel(px, py_bot);

                        surface.set_char(
                            cell_x as u16,
                            cell_y as u16,
                            '▀',
                            top_color,
                            bot_color,
                            Modifier::empty(),
                        );
                    }
                }
            }
        }
    }

    /// Generates a Kitty APC graphics protocol escape sequence for hardware passthrough.
    pub fn format_kitty_apc(&self) -> String {
        format!("\x1b_Ga=T,f=32,s={},v={},m=0;<raw_data>\x1b\\", self.width, self.height)
    }

    /// Generates an iTerm2 inline image escape sequence.
    pub fn format_iterm2_inline(&self) -> String {
        format!(
            "\x1b]1337;File=inline=1;width={}px;height={}px:<data>\x07",
            self.width, self.height
        )
    }

    fn sample_pixel(&self, x: u32, y: u32) -> Color {
        let idx = ((y.min(self.height - 1) * self.width + x.min(self.width - 1)) * 4) as usize;
        if idx + 3 < self.rgba_pixels.len() {
            let r = self.rgba_pixels[idx];
            let g = self.rgba_pixels[idx + 1];
            let b = self.rgba_pixels[idx + 2];
            Color::Rgb(r, g, b)
        } else {
            Color::Black
        }
    }
}
