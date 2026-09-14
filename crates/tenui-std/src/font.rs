use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// 4×6 bitmap font definitions for ASCII 0x20..=0x7E (A-Z, 0-9, punctuation).
/// Each character is represented by 6 rows of 4 bits (highest bit unused or col 3).
/// Bits [3, 2, 1, 0] map to columns [0, 1, 2, 3] from left to right.
pub mod bitmap4x6 {
    /// Glyph bitmap: 6 rows, 4 bits per row (values 0..15).
    pub type Glyph4x6 = [u8; 6];

    /// Returns the 4×6 bitmap for ASCII character `c`, or a fallback block for unknown.
    pub const fn get_glyph(c: char) -> Glyph4x6 {
        let ch = if c >= 'a' && c <= 'z' {
            (c as u8 - b'a' + b'A') as char
        } else {
            c
        };

        match ch {
            ' ' => [0b0000, 0b0000, 0b0000, 0b0000, 0b0000, 0b0000],
            'A' => [0b0110, 0b1001, 0b1111, 0b1001, 0b1001, 0b1001],
            'B' => [0b1110, 0b1001, 0b1110, 0b1001, 0b1001, 0b1110],
            'C' => [0b0111, 0b1000, 0b1000, 0b1000, 0b1000, 0b0111],
            'D' => [0b1110, 0b1001, 0b1001, 0b1001, 0b1001, 0b1110],
            'E' => [0b1111, 0b1000, 0b1110, 0b1000, 0b1000, 0b1111],
            'F' => [0b1111, 0b1000, 0b1110, 0b1000, 0b1000, 0b1000],
            'G' => [0b0111, 0b1000, 0b1000, 0b1011, 0b1001, 0b0111],
            'H' => [0b1001, 0b1001, 0b1111, 0b1001, 0b1001, 0b1001],
            'I' => [0b1110, 0b0100, 0b0100, 0b0100, 0b0100, 0b1110],
            'J' => [0b0011, 0b0001, 0b0001, 0b0001, 0b1001, 0b0110],
            'K' => [0b1001, 0b1010, 0b1100, 0b1100, 0b1010, 0b1001],
            'L' => [0b1000, 0b1000, 0b1000, 0b1000, 0b1000, 0b1111],
            'M' => [0b1001, 0b1111, 0b1001, 0b1001, 0b1001, 0b1001],
            'N' => [0b1001, 0b1101, 0b1001, 0b1011, 0b1001, 0b1001],
            'O' => [0b0110, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110],
            'P' => [0b1110, 0b1001, 0b1110, 0b1000, 0b1000, 0b1000],
            'Q' => [0b0110, 0b1001, 0b1001, 0b1001, 0b1011, 0b0111],
            'R' => [0b1110, 0b1001, 0b1110, 0b1100, 0b1010, 0b1001],
            'S' => [0b0111, 0b1000, 0b0110, 0b0001, 0b0001, 0b1110],
            'T' => [0b1111, 0b0110, 0b0110, 0b0110, 0b0110, 0b0110],
            'U' => [0b1001, 0b1001, 0b1001, 0b1001, 0b1001, 0b0110],
            'V' => [0b1001, 0b1001, 0b1001, 0b1001, 0b0110, 0b0110],
            'W' => [0b1001, 0b1001, 0b1001, 0b1111, 0b1111, 0b1001],
            'X' => [0b1001, 0b1001, 0b0110, 0b0110, 0b1001, 0b1001],
            'Y' => [0b1001, 0b1001, 0b0110, 0b0110, 0b0110, 0b0110],
            'Z' => [0b1111, 0b0001, 0b0010, 0b0110, 0b1000, 0b1111],
            '0' => [0b0110, 0b1001, 0b1011, 0b1101, 0b1001, 0b0110],
            '1' => [0b0100, 0b1100, 0b0100, 0b0100, 0b0100, 0b1110],
            '2' => [0b1110, 0b0001, 0b0110, 0b1000, 0b1000, 0b1111],
            '3' => [0b1110, 0b0001, 0b0110, 0b0001, 0b0001, 0b1110],
            '4' => [0b1001, 0b1001, 0b1111, 0b0001, 0b0001, 0b0001],
            '5' => [0b1111, 0b1000, 0b1110, 0b0001, 0b0001, 0b1110],
            '6' => [0b0111, 0b1000, 0b1110, 0b1001, 0b1001, 0b0110],
            '7' => [0b1111, 0b0001, 0b0010, 0b0100, 0b0100, 0b0100],
            '8' => [0b0110, 0b1001, 0b0110, 0b1001, 0b1001, 0b0110],
            '9' => [0b0110, 0b1001, 0b0111, 0b0001, 0b0001, 0b1110],
            ':' => [0b0000, 0b0110, 0b0000, 0b0110, 0b0000, 0b0000],
            '.' => [0b0000, 0b0000, 0b0000, 0b0000, 0b0110, 0b0110],
            ',' => [0b0000, 0b0000, 0b0000, 0b0000, 0b0100, 0b1000],
            '-' => [0b0000, 0b0000, 0b1111, 0b1111, 0b0000, 0b0000],
            '_' => [0b0000, 0b0000, 0b0000, 0b0000, 0b1111, 0b1111],
            '!' => [0b0110, 0b0110, 0b0110, 0b0110, 0b0000, 0b0110],
            '?' => [0b1110, 0b0001, 0b0110, 0b0100, 0b0000, 0b0100],
            '/' => [0b0001, 0b0010, 0b0010, 0b0100, 0b0100, 0b1000],
            '\\' => [0b1000, 0b0100, 0b0100, 0b0010, 0b0010, 0b0001],
            '+' => [0b0000, 0b0100, 0b1110, 0b0100, 0b0000, 0b0000],
            '=' => [0b0000, 0b1111, 0b0000, 0b1111, 0b0000, 0b0000],
            '#' => [0b1010, 0b1111, 0b1010, 0b1111, 0b1010, 0b0000],
            '*' => [0b0000, 0b1010, 0b0100, 0b1010, 0b0000, 0b0000],
            '[' => [0b1110, 0b1000, 0b1000, 0b1000, 0b1000, 0b1110],
            ']' => [0b0111, 0b0001, 0b0001, 0b0001, 0b0001, 0b0111],
            '(' => [0b0100, 0b1000, 0b1000, 0b1000, 0b1000, 0b0100],
            ')' => [0b0010, 0b0001, 0b0001, 0b0001, 0b0001, 0b0010],
            '<' => [0b0010, 0b0100, 0b1000, 0b0100, 0b0010, 0b0000],
            '>' => [0b1000, 0b0100, 0b0010, 0b0100, 0b1000, 0b0000],
            '|' => [0b0110, 0b0110, 0b0110, 0b0110, 0b0110, 0b0110],
            _ => [0b1111, 0b1111, 0b1111, 0b1111, 0b1111, 0b1111],
        }
    }
}

/// Rendering style for the [`Banner`] widget.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BigFontStyle {
    /// Half-block rendering using `▀`, `▄`, and `█`.
    /// Each terminal row holds 2 vertical bitmap pixels, producing a crisp, compact 3-row headline.
    #[default]
    HalfBlock,
    /// Full-cell block rendering where each pixel is drawn as a full cell `█`.
    /// Produces a bold, punchy 6-row banner.
    FullBlock,
}

/// Alignment of rendered banner text within its canvas subview.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BannerAlignment {
    #[default]
    Left,
    Center,
    Right,
}

/// Large stylized banner text widget inspired by the bitmap geometry of the classic Tenui font.
///
/// Converts short strings (titles, headlines, clocks, version badges) into crisp, multi-row
/// block-character glyphs that render without any external fonts or image protocols.
///
/// # Example
///
/// ```rust,no_run
/// use tenui_std::font::{Banner, BigFontStyle, BannerAlignment};
/// use tenui_core::Color;
///
/// let banner = Banner::new("TERMINUS")
///     .with_fg(Color::Cyan)
///     .with_style(BigFontStyle::HalfBlock)
///     .with_alignment(BannerAlignment::Center);
/// ```
#[derive(Debug, Clone)]
pub struct Banner {
    pub text: String,
    pub fg: Color,
    pub bg: Color,
    pub style: BigFontStyle,
    pub alignment: BannerAlignment,
}

impl Banner {
    pub fn new(text: impl Into<String>) -> Self {
        Self {
            text: text.into(),
            fg: Color::Reset,
            bg: Color::Reset,
            style: BigFontStyle::HalfBlock,
            alignment: BannerAlignment::Left,
        }
    }

    pub fn with_fg(mut self, fg: Color) -> Self {
        self.fg = fg;
        self
    }

    pub fn with_bg(mut self, bg: Color) -> Self {
        self.bg = bg;
        self
    }

    pub fn with_style(mut self, style: BigFontStyle) -> Self {
        self.style = style;
        self
    }

    pub fn with_alignment(mut self, alignment: BannerAlignment) -> Self {
        self.alignment = alignment;
        self
    }

    /// Computes the layout dimensions `(width_columns, height_rows)` needed for this banner.
    pub fn measure(&self) -> (u16, u16) {
        let chars_count = self.text.chars().count();
        if chars_count == 0 {
            return (0, 0);
        }
        // Each glyph is 4 columns wide + 1 space between characters
        let total_w = (chars_count * 5).saturating_sub(1) as u16;
        let total_h = match self.style {
            BigFontStyle::HalfBlock => 3, // 6 pixels / 2 pixels per row
            BigFontStyle::FullBlock => 6, // 6 pixels = 6 rows
        };
        (total_w, total_h)
    }

    /// Renders the banner onto `canvas` respecting bounds, clipping, and alignment.
    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let cw = canvas.width();
        let ch = canvas.height();
        if cw == 0 || ch == 0 || self.text.is_empty() {
            return;
        }

        let (banner_w, _banner_h) = self.measure();
        let start_x = match self.alignment {
            BannerAlignment::Left => 0,
            BannerAlignment::Center => cw.saturating_sub(banner_w) / 2,
            BannerAlignment::Right => cw.saturating_sub(banner_w),
        };

        match self.style {
            BigFontStyle::HalfBlock => {
                // 3 rows: row 0 (pixels 0,1), row 1 (pixels 2,3), row 2 (pixels 4,5)
                for cell_y in 0..3u16 {
                    if cell_y >= ch {
                        break;
                    }
                    let top_pixel_row = (cell_y * 2) as usize;
                    let bot_pixel_row = top_pixel_row + 1;

                    let mut cursor_x = start_x;
                    for ch_char in self.text.chars() {
                        let glyph = bitmap4x6::get_glyph(ch_char);
                        let top_bits = glyph[top_pixel_row];
                        let bot_bits = glyph[bot_pixel_row];

                        for col in 0..4u16 {
                            let x = cursor_x + col;
                            if x >= cw {
                                break;
                            }
                            let bit_mask = 1 << (3 - col);
                            let top_on = (top_bits & bit_mask) != 0;
                            let bot_on = (bot_bits & bit_mask) != 0;

                            let sym = match (top_on, bot_on) {
                                (true, true) => '█',
                                (true, false) => '▀',
                                (false, true) => '▄',
                                (false, false) => ' ',
                            };

                            if sym != ' ' {
                                canvas.set_char(x, cell_y, sym, self.fg, self.bg, Modifier::empty());
                            }
                        }
                        cursor_x += 5; // 4 cols + 1 space
                    }
                }
            }
            BigFontStyle::FullBlock => {
                for pixel_y in 0..6u16 {
                    if pixel_y >= ch {
                        break;
                    }
                    let mut cursor_x = start_x;
                    for ch_char in self.text.chars() {
                        let glyph = bitmap4x6::get_glyph(ch_char);
                        let row_bits = glyph[pixel_y as usize];

                        for col in 0..4u16 {
                            let x = cursor_x + col;
                            if x >= cw {
                                break;
                            }
                            let bit_mask = 1 << (3 - col);
                            if (row_bits & bit_mask) != 0 {
                                canvas.set_char(x, pixel_y, '█', self.fg, self.bg, Modifier::empty());
                            }
                        }
                        cursor_x += 5;
                    }
                }
            }
        }
    }
}

// Braille Micro-Canvas & Sub-Pixel Font Engine

/// High-resolution 2×4 sub-pixel Braille canvas.
///
/// Maps terminal character cells into 2-column × 4-row dot matrices (`U+2800..U+28FF`).
/// Allows plotting sub-pixel lines, dots, graphs, and miniature text annotations.
#[derive(Debug, Clone)]
pub struct BrailleCanvas {
    pub width_cells: u16,
    pub height_cells: u16,
    dots: Vec<u8>,
}

impl BrailleCanvas {
    /// Creates a new sub-pixel Braille canvas sized in character cell dimensions.
    /// Pixel dimensions will be `(width_cells * 2, height_cells * 4)`.
    pub fn new(width_cells: u16, height_cells: u16) -> Self {
        let size = (width_cells as usize) * (height_cells as usize);
        Self {
            width_cells,
            height_cells,
            dots: vec![0; size],
        }
    }

    /// Effective pixel width (2 dots per character cell).
    #[inline]
    pub fn pixel_width(&self) -> u16 {
        self.width_cells * 2
    }

    /// Effective pixel height (4 dots per character cell).
    #[inline]
    pub fn pixel_height(&self) -> u16 {
        self.height_cells * 4
    }

    /// Clears all dots in the canvas.
    pub fn clear(&mut self) {
        self.dots.fill(0);
    }

    /// Sets or clears an individual sub-pixel dot at `(px, py)`.
    pub fn set_pixel(&mut self, px: u16, py: u16, enabled: bool) {
        if px >= self.pixel_width() || py >= self.pixel_height() {
            return;
        }

        let cell_x = px / 2;
        let cell_y = py / 4;
        let dot_x = px % 2;
        let dot_y = py % 4;

        // Standard Unicode Braille dot bitmask mapping:
        // Dot 1 (0x01): (0,0)  Dot 4 (0x08): (1,0)
        // Dot 2 (0x02): (0,1)  Dot 5 (0x10): (1,1)
        // Dot 3 (0x04): (0,2)  Dot 6 (0x20): (1,2)
        // Dot 7 (0x40): (0,3)  Dot 8 (0x80): (1,3)
        let mask = match (dot_x, dot_y) {
            (0, 0) => 0x01,
            (0, 1) => 0x02,
            (0, 2) => 0x04,
            (0, 3) => 0x40,
            (1, 0) => 0x08,
            (1, 1) => 0x10,
            (1, 2) => 0x20,
            (1, 3) => 0x80,
            _ => 0,
        };

        let idx = (cell_y as usize) * (self.width_cells as usize) + (cell_x as usize);
        if enabled {
            self.dots[idx] |= mask;
        } else {
            self.dots[idx] &= !mask;
        }
    }

    /// Draws a sub-pixel line using Bresenham's algorithm.
    pub fn draw_line(&mut self, mut x0: i32, mut y0: i32, x1: i32, y1: i32) {
        let dx = (x1 - x0).abs();
        let sx = if x0 < x1 { 1 } else { -1 };
        let dy = -(y1 - y0).abs();
        let sy = if y0 < y1 { 1 } else { -1 };
        let mut err = dx + dy;

        loop {
            if x0 >= 0 && y0 >= 0 {
                self.set_pixel(x0 as u16, y0 as u16, true);
            }
            if x0 == x1 && y0 == y1 {
                break;
            }
            let e2 = 2 * err;
            if e2 >= dy {
                err += dy;
                x0 += sx;
            }
            if e2 <= dx {
                err += dx;
                y0 += sy;
            }
        }
    }

    /// Draws miniature text using the 4×6 bitmap font starting at pixel coordinate `(px, py)`.
    pub fn draw_text(&mut self, mut px: u16, py: u16, text: &str) {
        for c in text.chars() {
            let glyph = bitmap4x6::get_glyph(c);
            for row in 0..6u16 {
                let bits = glyph[row as usize];
                for col in 0..4u16 {
                    let bit_mask = 1 << (3 - col);
                    if (bits & bit_mask) != 0 {
                        self.set_pixel(px + col, py + row, true);
                    }
                }
            }
            px += 5; // 4 dots wide + 1 dot space
        }
    }

    /// Blits the Braille canvas to a `CanvasSubviewMut` using the given colors.
    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let max_x = self.width_cells.min(canvas.width());
        let max_y = self.height_cells.min(canvas.height());

        for cy in 0..max_y {
            for cx in 0..max_x {
                let idx = (cy as usize) * (self.width_cells as usize) + (cx as usize);
                let byte = self.dots[idx];
                let ch = if byte == 0 {
                    ' '
                } else {
                    char::from_u32(0x2800 + (byte as u32)).unwrap_or(' ')
                };
                if ch != ' ' {
                    canvas.set_char(cx, cy, ch, fg, bg, Modifier::empty());
                }
            }
        }
    }

    /// Formats the canvas into a multi-line string of Braille characters.
    pub fn to_braille_string(&self) -> String {
        let mut out = String::with_capacity((self.width_cells as usize + 1) * self.height_cells as usize);
        for cy in 0..self.height_cells {
            for cx in 0..self.width_cells {
                let idx = (cy as usize) * (self.width_cells as usize) + (cx as usize);
                let byte = self.dots[idx];
                let ch = char::from_u32(0x2800 + (byte as u32)).unwrap_or(' ');
                out.push(ch);
            }
            if cy + 1 < self.height_cells {
                out.push('\n');
            }
        }
        out
    }
}

// Unicode Typographic Font Transformations

/// Unicode typographic font variant styles.
///
/// Replaces standard ASCII alphanumeric characters with corresponding mathematical
/// or stylized Unicode alphabets (monospace, sans-serif, blackboard bold, small caps, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum UnicodeFont {
    #[default]
    Plain,
    /// Fixed-width monospace: `𝚝𝚎𝚛𝚖𝚒𝚗𝚞𝚜 𝟶𝟷𝟸`
    Monospace,
    /// Clean geometric sans-serif: `𝗍𝖾𝗋𝗆𝗂𝗇𝚞𝚜 𝟢𝟣𝟤`
    Sans,
    /// Strong sans-serif bold: `𝘁𝗲𝗿𝗺𝗶𝗻𝘂𝘀 𝟬𝟭𝟮`
    SansBold,
    /// Classic serif bold: `𝐭𝐞𝐫𝐦𝐢𝐧𝐮𝐬 𝟎𝟏𝟐`
    SerifBold,
    /// Double-struck blackboard bold: `𝕥𝕖𝕣𝕞𝕚𝚗𝚞𝚜 𝟘𝟙𝟚`
    DoubleStruck,
    /// Petite small capitals: `ᴛᴇʀᴍɪɴᴜs`
    SmallCaps,
    /// Traditional Fraktur / Gothic: `𝔱𝔢𝔯𝔪𝔦𝔫𝔲𝔰`
    Fraktur,
    /// Elegant cursive / script: `𝓉ℯ𝓇𝓂𝒾𝓃𝓊𝓈`
    Script,
    /// Circled characters: `ⓣⓔⓡⓜⓘⓝⓤⓢ ①②③`
    Circled,
    /// Wide fullwidth Asian typography: `ｔｅｒｍｉｎｕｓ`
    Fullwidth,
}

impl UnicodeFont {
    /// Maps an individual character to its styled representation under this font.
    /// Non-alphanumeric characters pass through unchanged.
    pub fn transform_char(&self, c: char) -> char {
        match self {
            Self::Plain => c,
            Self::Monospace => match c {
                'A'..='Z' => char::from_u32(0x1D670 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D68A + (c as u32 - 'a' as u32)).unwrap_or(c),
                '0'..='9' => char::from_u32(0x1D7F6 + (c as u32 - '0' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::Sans => match c {
                'A'..='Z' => char::from_u32(0x1D5A0 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D5BA + (c as u32 - 'a' as u32)).unwrap_or(c),
                '0'..='9' => char::from_u32(0x1D7E2 + (c as u32 - '0' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::SansBold => match c {
                'A'..='Z' => char::from_u32(0x1D5D4 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D5EE + (c as u32 - 'a' as u32)).unwrap_or(c),
                '0'..='9' => char::from_u32(0x1D7EC + (c as u32 - '0' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::SerifBold => match c {
                'A'..='Z' => char::from_u32(0x1D400 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D41A + (c as u32 - 'a' as u32)).unwrap_or(c),
                '0'..='9' => char::from_u32(0x1D7CE + (c as u32 - '0' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::DoubleStruck => match c {
                'C' => 'ℂ',
                'H' => 'ℍ',
                'N' => 'ℕ',
                'P' => 'ℙ',
                'Q' => 'ℚ',
                'R' => 'ℝ',
                'Z' => 'ℤ',
                'A'..='Z' => char::from_u32(0x1D538 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D552 + (c as u32 - 'a' as u32)).unwrap_or(c),
                '0'..='9' => char::from_u32(0x1D7D8 + (c as u32 - '0' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::SmallCaps => match c {
                'A' | 'a' => 'ᴀ',
                'B' | 'b' => 'ʙ',
                'C' | 'c' => 'ᴄ',
                'D' | 'd' => 'ᴅ',
                'E' | 'e' => 'ᴇ',
                'F' | 'f' => 'ғ',
                'G' | 'g' => 'ɢ',
                'H' | 'h' => 'ʜ',
                'I' | 'i' => 'ɪ',
                'J' | 'j' => 'ᴊ',
                'K' | 'k' => 'ᴋ',
                'L' | 'l' => 'ʟ',
                'M' | 'm' => 'ᴍ',
                'N' | 'n' => 'ɴ',
                'O' | 'o' => 'ᴏ',
                'P' | 'p' => 'ᴘ',
                'Q' | 'q' => 'ǫ',
                'R' | 'r' => 'ʀ',
                'S' | 's' => 's',
                'T' | 't' => 'ᴛ',
                'U' | 'u' => 'ᴜ',
                'V' | 'v' => 'ᴠ',
                'W' | 'w' => 'ᴡ',
                'X' | 'x' => 'x',
                'Y' | 'y' => 'ʏ',
                'Z' | 'z' => 'ᴢ',
                _ => c,
            },
            Self::Fraktur => match c {
                'C' => 'ℭ',
                'H' => 'ℌ',
                'I' => 'ℑ',
                'R' => 'ℜ',
                'Z' => 'ℨ',
                'A'..='Z' => char::from_u32(0x1D504 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D51E + (c as u32 - 'a' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::Script => match c {
                'B' => 'ℬ',
                'E' => 'ℰ',
                'F' => 'ℱ',
                'H' => 'ℋ',
                'I' => 'ℐ',
                'L' => 'ℒ',
                'M' => 'ℳ',
                'R' => 'ℛ',
                'e' => 'ℯ',
                'g' => 'ℊ',
                'o' => 'ℴ',
                'A'..='Z' => char::from_u32(0x1D49C + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x1D4B6 + (c as u32 - 'a' as u32)).unwrap_or(c),
                _ => c,
            },
            Self::Circled => match c {
                'A'..='Z' => char::from_u32(0x24B6 + (c as u32 - 'A' as u32)).unwrap_or(c),
                'a'..='z' => char::from_u32(0x24D0 + (c as u32 - 'a' as u32)).unwrap_or(c),
                '1'..='9' => char::from_u32(0x2460 + (c as u32 - '1' as u32)).unwrap_or(c),
                '0' => '⓪',
                _ => c,
            },
            Self::Fullwidth => match c {
                '!'..='~' => char::from_u32(0xFF01 + (c as u32 - '!' as u32)).unwrap_or(c),
                ' ' => '　',
                _ => c,
            },
        }
    }

    /// Transforms an entire string into this font's typographic style.
    pub fn transform(&self, text: &str) -> String {
        if *self == Self::Plain {
            return text.to_string();
        }
        text.chars().map(|c| self.transform_char(c)).collect()
    }
}

// Tests

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn test_banner_measurement_and_rendering() {
        let banner = Banner::new("GO")
            .with_fg(Color::Cyan)
            .with_style(BigFontStyle::HalfBlock);

        let (w, h) = banner.measure();
        assert_eq!(w, 9); // 2 chars * 4 + 1 space = 9
        assert_eq!(h, 3); // HalfBlock height = 3

        let mut buf = Buffer::new(20, 5);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 20, 5));
            banner.render(&mut canvas);
        }

        // Must have non-space glyphs rendered in the first 3 rows
        let row0: String = (0..9).map(|x| buf.get(x, 0).unwrap().symbol.as_str()).collect();
        assert!(row0.contains('█') || row0.contains('▀') || row0.contains('▄'));
    }

    #[test]
    fn test_braille_canvas_subpixel_dots() {
        let mut canvas = BrailleCanvas::new(10, 5);
        assert_eq!(canvas.pixel_width(), 20);
        assert_eq!(canvas.pixel_height(), 20);

        // Set top-left dot
        canvas.set_pixel(0, 0, true);
        let s = canvas.to_braille_string();
        let first_ch = s.chars().next().unwrap();
        // Dot 1 (0,0) is U+2801 ('⠁')
        assert_eq!(first_ch, '⠁');

        // Draw micro text
        canvas.draw_text(2, 2, "OK");
        let rendered = canvas.to_braille_string();
        assert!(rendered.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)));
    }

    #[test]
    fn test_unicode_font_transformations() {
        let text = "Tenui 2026";

        let mono = UnicodeFont::Monospace.transform(text);
        assert_eq!(mono, "𝚃𝚎𝚗𝚞𝚒 𝟸𝟶𝟸𝟼");

        let sans_bold = UnicodeFont::SansBold.transform(text);
        assert_eq!(sans_bold, "𝗧𝗲𝗻𝘂𝗶 𝟮𝟬𝟮𝟲");

        let dbl = UnicodeFont::DoubleStruck.transform("Rust");
        assert_eq!(dbl, "ℝ𝕦𝕤𝕥");

        let small = UnicodeFont::SmallCaps.transform("tui");
        assert_eq!(small, "ᴛᴜɪ");

        let circled = UnicodeFont::Circled.transform("ABC");
        assert_eq!(circled, "ⒶⒷⒸ");
    }
}
