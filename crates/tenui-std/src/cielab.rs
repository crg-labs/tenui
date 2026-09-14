use tenui_core::Color;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Lab {
    pub l: f32,
    pub a: f32,
    pub b: f32,
}

impl Lab {
    pub fn delta_e(&self, other: &Lab) -> f32 {
        let dl = self.l - other.l;
        let da = self.a - other.a;
        let db = self.b - other.b;
        (dl * dl + da * da + db * db).sqrt()
    }
}

/// Converts an sRGB triplet to CIELAB (D65 standard illuminant).
pub fn rgb_to_cielab(r: u8, g: u8, b: u8) -> Lab {
    let linearize = |v: u8| -> f32 {
        let s = v as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };

    let lr = linearize(r);
    let lg = linearize(g);
    let lb = linearize(b);

    let x = (lr * 0.4124564 + lg * 0.3575761 + lb * 0.1804375) / 0.95047;
    let y = (lr * 0.2126729 + lg * 0.7151522 + lb * 0.0721750) / 1.00000;
    let z = (lr * 0.0193339 + lg * 0.119_192 + lb * 0.9503041) / 1.08883;

    let f = |t: f32| -> f32 {
        if t > 0.008856 {
            t.powf(1.0 / 3.0)
        } else {
            7.787 * t + (16.0 / 116.0)
        }
    };

    let fx = f(x);
    let fy = f(y);
    let fz = f(z);

    Lab {
        l: 116.0 * fy - 16.0,
        a: 500.0 * (fx - fy),
        b: 200.0 * (fy - fz),
    }
}

const ANSI_16_PALETTE: [(Color, (u8, u8, u8)); 16] = [
    (Color::Black, (0, 0, 0)),
    (Color::Red, (205, 0, 0)),
    (Color::Green, (0, 205, 0)),
    (Color::Yellow, (205, 205, 0)),
    (Color::Blue, (0, 0, 238)),
    (Color::Magenta, (205, 0, 205)),
    (Color::Cyan, (0, 205, 205)),
    (Color::Gray, (229, 229, 229)),
    (Color::DarkGray, (127, 127, 127)),
    (Color::LightRed, (255, 0, 0)),
    (Color::LightGreen, (0, 255, 0)),
    (Color::LightYellow, (255, 255, 0)),
    (Color::LightBlue, (92, 92, 255)),
    (Color::LightMagenta, (255, 0, 255)),
    (Color::LightCyan, (0, 255, 255)),
    (Color::White, (255, 255, 255)),
];

/// Perceptually downsamples a TrueColor RGB to the nearest ANSI 16 color using CIELAB ΔE.
pub fn downsample_to_ansi16(r: u8, g: u8, b: u8) -> Color {
    let target_lab = rgb_to_cielab(r, g, b);

    let mut best_color = Color::White;
    let mut min_delta = f32::MAX;

    for (color, (pr, pg, pb)) in ANSI_16_PALETTE {
        let palette_lab = rgb_to_cielab(pr, pg, pb);
        let dist = target_lab.delta_e(&palette_lab);
        if dist < min_delta {
            min_delta = dist;
            best_color = color;
        }
    }

    best_color
}

/// Downsamples TrueColor RGB to the nearest color index in the ANSI 256 color cube.
pub fn downsample_to_ansi256(r: u8, g: u8, b: u8) -> Color {
    let target_lab = rgb_to_cielab(r, g, b);
    let mut best_idx = 16u8;
    let mut min_delta = f32::MAX;

    // 6x6x6 color cube: steps at 0, 95, 135, 175, 215, 255
    const STEPS: [u8; 6] = [0, 95, 135, 175, 215, 255];

    for (r_i, &cr) in STEPS.iter().enumerate() {
        for (g_i, &cg) in STEPS.iter().enumerate() {
            for (b_i, &cb) in STEPS.iter().enumerate() {
                let code = 16 + (36 * r_i + 6 * g_i + b_i) as u8;
                let lab = rgb_to_cielab(cr, cg, cb);
                let d = target_lab.delta_e(&lab);
                if d < min_delta {
                    min_delta = d;
                    best_idx = code;
                }
            }
        }
    }

    Color::Indexed(best_idx)
}

/// Terminal color support capability profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColorProfile {
    TrueColor,
    Ansi256,
    Ansi16,
}

/// Detects terminal color support from standard environment hints.
pub fn detect_color_capability() -> ColorProfile {
    if let Ok(val) = std::env::var("COLORTERM")
        && (val == "truecolor" || val == "24bit")
    {
        return ColorProfile::TrueColor;
    }

    if let Ok(val) = std::env::var("TERM")
        && val.contains("256color")
    {
        return ColorProfile::Ansi256;
    }

    ColorProfile::Ansi16
}
