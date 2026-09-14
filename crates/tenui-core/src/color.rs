/// Terminal color representations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Color {
    #[default]
    Reset,
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    Gray,
    DarkGray,
    LightRed,
    LightGreen,
    LightYellow,
    LightBlue,
    LightMagenta,
    LightCyan,
    White,
    Indexed(u8),
    Rgb(u8, u8, u8),
}

/// Appends `n` as base-10 ASCII to `out` without the `core::fmt` machinery, the SGR
/// hot path formats many small integers per frame, and `write!`'s `Formatter`/`Display`
/// overhead showed up prominently in profiling. No allocation, no `unsafe`.
pub(crate) fn push_dec(out: &mut String, n: u32) {
    if n < 10 {
        out.push((b'0' + n as u8) as char);
        return;
    }
    let mut buf = [0u8; 10];
    let mut i = buf.len();
    let mut v = n;
    while v > 0 {
        i -= 1;
        buf[i] = b'0' + (v % 10) as u8;
        v /= 10;
    }
    for &b in &buf[i..] {
        out.push(b as char);
    }
}

impl Color {
    /// Formats foreground SGR parameter sequence into the given string buffer.
    pub fn write_fg_sgr(&self, out: &mut String) {
        match self {
            Color::Reset => out.push_str("39"),
            Color::Black => out.push_str("30"),
            Color::Red => out.push_str("31"),
            Color::Green => out.push_str("32"),
            Color::Yellow => out.push_str("33"),
            Color::Blue => out.push_str("34"),
            Color::Magenta => out.push_str("35"),
            Color::Cyan => out.push_str("36"),
            Color::Gray => out.push_str("37"),
            Color::DarkGray => out.push_str("90"),
            Color::LightRed => out.push_str("91"),
            Color::LightGreen => out.push_str("92"),
            Color::LightYellow => out.push_str("93"),
            Color::LightBlue => out.push_str("94"),
            Color::LightMagenta => out.push_str("95"),
            Color::LightCyan => out.push_str("96"),
            Color::White => out.push_str("97"),
            Color::Indexed(idx) => {
                out.push_str("38;5;");
                push_dec(out, *idx as u32);
            }
            Color::Rgb(r, g, b) => {
                out.push_str("38;2;");
                push_dec(out, *r as u32);
                out.push(';');
                push_dec(out, *g as u32);
                out.push(';');
                push_dec(out, *b as u32);
            }
        }
    }

    /// Formats background SGR parameter sequence into the given string buffer.
    pub fn write_bg_sgr(&self, out: &mut String) {
        match self {
            Color::Reset => out.push_str("49"),
            Color::Black => out.push_str("40"),
            Color::Red => out.push_str("41"),
            Color::Green => out.push_str("42"),
            Color::Yellow => out.push_str("43"),
            Color::Blue => out.push_str("44"),
            Color::Magenta => out.push_str("45"),
            Color::Cyan => out.push_str("46"),
            Color::Gray => out.push_str("47"),
            Color::DarkGray => out.push_str("100"),
            Color::LightRed => out.push_str("101"),
            Color::LightGreen => out.push_str("102"),
            Color::LightYellow => out.push_str("103"),
            Color::LightBlue => out.push_str("104"),
            Color::LightMagenta => out.push_str("105"),
            Color::LightCyan => out.push_str("106"),
            Color::White => out.push_str("107"),
            Color::Indexed(idx) => {
                out.push_str("48;5;");
                push_dec(out, *idx as u32);
            }
            Color::Rgb(r, g, b) => {
                out.push_str("48;2;");
                push_dec(out, *r as u32);
                out.push(';');
                push_dec(out, *g as u32);
                out.push(';');
                push_dec(out, *b as u32);
            }
        }
    }

    pub const TRANSPARENT: Color = Color::Reset;
    pub const BLACK: Color = Color::Black;
    pub const WHITE: Color = Color::White;
    pub const RED: Color = Color::Red;
    pub const GREEN: Color = Color::Green;
    pub const YELLOW: Color = Color::Yellow;
    pub const BLUE: Color = Color::Blue;
    pub const MAGENTA: Color = Color::Magenta;
    pub const CYAN: Color = Color::Cyan;
    pub const GRAY: Color = Color::Gray;
    pub const DARK_GRAY: Color = Color::DarkGray;

    /// Constructs an RGB color from a 24-bit hexadecimal integer (e.g. 0x38bdf8).
    pub const fn hex(rgb: u32) -> Self {
        Color::Rgb(
            ((rgb >> 16) & 0xFF) as u8,
            ((rgb >> 8) & 0xFF) as u8,
            (rgb & 0xFF) as u8,
        )
    }

    /// Uppercase alias for DSL ergonomics matching the specification.
    #[allow(non_snake_case)]
    pub const fn HEX(rgb: u32) -> Self {
        Self::hex(rgb)
    }

    /// A lossless 32-bit key uniquely identifying this color *including its variant*.
    ///
    /// Unlike [`Self::to_rgb`] (which collapses `Reset`/named/`Rgb` that resolve to
    /// the same pixel), equal keys ⟺ equal `Color`. Used by the SIMD differential
    /// compositor to pack a cell's colors into vector lanes without losing diff fidelity.
    pub const fn diff_key(&self) -> u32 {
        match self {
            Color::Reset => 0,
            Color::Black => 1,
            Color::Red => 2,
            Color::Green => 3,
            Color::Yellow => 4,
            Color::Blue => 5,
            Color::Magenta => 6,
            Color::Cyan => 7,
            Color::Gray => 8,
            Color::DarkGray => 9,
            Color::LightRed => 10,
            Color::LightGreen => 11,
            Color::LightYellow => 12,
            Color::LightBlue => 13,
            Color::LightMagenta => 14,
            Color::LightCyan => 15,
            Color::White => 16,
            Color::Indexed(idx) => 0x0100_0000 | (*idx as u32),
            Color::Rgb(r, g, b) => 0x0200_0000 | ((*r as u32) << 16) | ((*g as u32) << 8) | (*b as u32),
        }
    }

    /// Converts this color representation to an approximate (R, G, B) tuple.
    pub fn to_rgb(&self) -> (u8, u8, u8) {
        match self {
            Color::Reset => (0, 0, 0),
            Color::Black => (0, 0, 0),
            Color::Red => (205, 49, 49),
            Color::Green => (13, 188, 121),
            Color::Yellow => (229, 229, 16),
            Color::Blue => (36, 114, 200),
            Color::Magenta => (188, 63, 188),
            Color::Cyan => (17, 168, 205),
            Color::Gray => (204, 204, 204),
            Color::DarkGray => (102, 102, 102),
            Color::LightRed => (241, 76, 76),
            Color::LightGreen => (35, 209, 139),
            Color::LightYellow => (245, 245, 67),
            Color::LightBlue => (59, 142, 234),
            Color::LightMagenta => (214, 112, 214),
            Color::LightCyan => (41, 184, 219),
            Color::White => (255, 255, 255),
            Color::Indexed(idx) => (*idx, *idx, *idx), // Grayscale fallback for raw indexed
            Color::Rgb(r, g, b) => (*r, *g, *b),
        }
    }

    /// Linearly interpolates between this color and `other` in sRGB space.
    pub fn lerp(&self, other: Color, t: f32) -> Color {
        let t = t.clamp(0.0, 1.0);
        let (r1, g1, b1) = self.to_rgb();
        let (r2, g2, b2) = other.to_rgb();

        let r = (r1 as f32 + (r2 as f32 - r1 as f32) * t).round() as u8;
        let g = (g1 as f32 + (g2 as f32 - g1 as f32) * t).round() as u8;
        let b = (b1 as f32 + (b2 as f32 - b1 as f32) * t).round() as u8;

        Color::Rgb(r, g, b)
    }

    /// Brightens the color by a fractional factor (0.0 = unchanged, 1.0 = full white).
    pub fn brighten(&self, factor: f32) -> Color {
        self.lerp(Color::WHITE, factor.clamp(0.0, 1.0))
    }

    /// Dims the color by a fractional factor (0.0 = unchanged, 1.0 = full black).
    pub fn dim(&self, factor: f32) -> Color {
        self.lerp(Color::BLACK, factor.clamp(0.0, 1.0))
    }

    /// Blends the color with alpha transparency toward black.
    pub fn with_alpha(&self, alpha: f32) -> Color {
        let alpha = alpha.clamp(0.0, 1.0);
        let (r, g, b) = self.to_rgb();
        Color::Rgb(
            (r as f32 * alpha).round() as u8,
            (g as f32 * alpha).round() as u8,
            (b as f32 * alpha).round() as u8,
        )
    }

    /// Relative luminance in ITU-R BT.709 space [0.0..1.0].
    pub fn relative_luminance(&self) -> f32 {
        let (r, g, b) = self.to_rgb();
        (0.2126 * r as f32 + 0.7152 * g as f32 + 0.0722 * b as f32) / 255.0
    }
}
