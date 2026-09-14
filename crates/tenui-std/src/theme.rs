use tenui_core::Color;

use crate::cielab::{ColorProfile, downsample_to_ansi16, downsample_to_ansi256};

/// Semantic color theme palette.
#[derive(Debug, Clone, PartialEq)]
pub struct ThemePalette {
    pub name: String,
    pub is_dark: bool,
    pub bg: Color,
    pub surface: Color,
    pub fg: Color,
    pub fg_muted: Color,
    pub primary: Color,
    pub secondary: Color,
    pub success: Color,
    pub warning: Color,
    pub error: Color,
    pub accent: Color,
    pub border: Color,
}

impl ThemePalette {
    pub fn catppuccin_mocha() -> Self {
        Self {
            name: "Catppuccin Mocha".into(),
            is_dark: true,
            bg: Color::Rgb(30, 30, 46),           // Base
            surface: Color::Rgb(49, 50, 68),      // Surface0
            fg: Color::Rgb(205, 214, 244),        // Text
            fg_muted: Color::Rgb(147, 153, 178),  // Subtext0
            primary: Color::Rgb(137, 180, 250),   // Blue
            secondary: Color::Rgb(203, 166, 247), // Mauve
            success: Color::Rgb(166, 227, 161),   // Green
            warning: Color::Rgb(249, 226, 175),   // Yellow
            error: Color::Rgb(243, 139, 168),     // Red
            accent: Color::Rgb(148, 226, 213),    // Teal
            border: Color::Rgb(69, 71, 90),       // Surface1
        }
    }

    pub fn catppuccin_latte() -> Self {
        Self {
            name: "Catppuccin Latte".into(),
            is_dark: false,
            bg: Color::Rgb(239, 241, 245),       // Base
            surface: Color::Rgb(204, 208, 218),  // Surface0
            fg: Color::Rgb(76, 79, 105),         // Text
            fg_muted: Color::Rgb(124, 127, 147), // Subtext0
            primary: Color::Rgb(30, 102, 245),   // Blue
            secondary: Color::Rgb(136, 57, 239), // Mauve
            success: Color::Rgb(64, 160, 43),    // Green
            warning: Color::Rgb(223, 142, 29),   // Yellow
            error: Color::Rgb(210, 15, 57),      // Red
            accent: Color::Rgb(23, 146, 153),    // Teal
            border: Color::Rgb(188, 192, 204),   // Surface1
        }
    }

    pub fn gruvbox() -> Self {
        Self {
            name: "Gruvbox Dark".into(),
            is_dark: true,
            bg: Color::Rgb(40, 40, 40),           // Dark0
            surface: Color::Rgb(60, 56, 54),      // Dark2
            fg: Color::Rgb(235, 219, 178),        // Light1
            fg_muted: Color::Rgb(168, 153, 132),  // Light4
            primary: Color::Rgb(131, 165, 152),   // Blue
            secondary: Color::Rgb(211, 134, 155), // Purple
            success: Color::Rgb(184, 187, 38),    // Green
            warning: Color::Rgb(250, 189, 47),    // Yellow
            error: Color::Rgb(251, 73, 52),       // Red
            accent: Color::Rgb(142, 192, 124),    // Aqua
            border: Color::Rgb(80, 73, 69),       // Dark3
        }
    }

    pub fn tokyo_night() -> Self {
        Self {
            name: "Tokyo Night".into(),
            is_dark: true,
            bg: Color::Rgb(26, 27, 38),
            surface: Color::Rgb(36, 40, 59),
            fg: Color::Rgb(192, 202, 245),
            fg_muted: Color::Rgb(86, 95, 137),
            primary: Color::Rgb(122, 162, 247),
            secondary: Color::Rgb(187, 154, 247),
            success: Color::Rgb(158, 206, 106),
            warning: Color::Rgb(224, 175, 104),
            error: Color::Rgb(247, 118, 142),
            accent: Color::Rgb(125, 207, 255),
            border: Color::Rgb(41, 46, 66),
        }
    }

    pub fn nord() -> Self {
        Self {
            name: "Nord".into(),
            is_dark: true,
            bg: Color::Rgb(46, 52, 64),           // Polar Night 0
            surface: Color::Rgb(59, 66, 82),      // Polar Night 1
            fg: Color::Rgb(236, 239, 244),        // Snow Storm 6
            fg_muted: Color::Rgb(216, 222, 233),  // Snow Storm 4
            primary: Color::Rgb(136, 192, 208),   // Frost 8
            secondary: Color::Rgb(180, 142, 173), // Aurora 15
            success: Color::Rgb(163, 190, 140),   // Aurora 14
            warning: Color::Rgb(235, 203, 139),   // Aurora 13
            error: Color::Rgb(191, 97, 106),      // Aurora 11
            accent: Color::Rgb(129, 161, 193),    // Frost 9
            border: Color::Rgb(76, 86, 106),      // Polar Night 3
        }
    }

    pub fn solarized_dark() -> Self {
        Self {
            name: "Solarized Dark".into(),
            is_dark: true,
            bg: Color::Rgb(0, 43, 54),            // base03
            surface: Color::Rgb(7, 54, 66),       // base02
            fg: Color::Rgb(131, 148, 150),        // base0
            fg_muted: Color::Rgb(101, 123, 131),  // base00
            primary: Color::Rgb(38, 139, 210),    // blue
            secondary: Color::Rgb(108, 113, 196), // violet
            success: Color::Rgb(133, 153, 0),     // green
            warning: Color::Rgb(181, 137, 0),     // yellow
            error: Color::Rgb(220, 50, 47),       // red
            accent: Color::Rgb(42, 161, 152),     // cyan
            border: Color::Rgb(88, 110, 117),     // base01
        }
    }

    pub fn monokai() -> Self {
        Self {
            name: "High-Contrast Monokai".into(),
            is_dark: true,
            bg: Color::Rgb(39, 40, 34),
            surface: Color::Rgb(62, 61, 50),
            fg: Color::Rgb(248, 248, 242),
            fg_muted: Color::Rgb(117, 113, 94),
            primary: Color::Rgb(102, 217, 239),   // Cyan
            secondary: Color::Rgb(174, 129, 255), // Purple
            success: Color::Rgb(166, 226, 46),    // Green
            warning: Color::Rgb(230, 219, 116),   // Yellow
            error: Color::Rgb(249, 38, 114),      // Red/Pink
            accent: Color::Rgb(253, 151, 31),     // Orange
            border: Color::Rgb(73, 72, 62),
        }
    }

    // Dark themes

    pub fn dracula() -> Self {
        Self {
            name: "Dracula".into(),
            is_dark: true,
            bg: Color::Rgb(40, 42, 54),           // Background
            surface: Color::Rgb(68, 71, 90),      // Current Line
            fg: Color::Rgb(248, 248, 242),        // Foreground
            fg_muted: Color::Rgb(98, 114, 164),   // Comment
            primary: Color::Rgb(139, 233, 253),   // Cyan
            secondary: Color::Rgb(189, 147, 249), // Purple
            success: Color::Rgb(80, 250, 123),    // Green
            warning: Color::Rgb(241, 250, 140),   // Yellow
            error: Color::Rgb(255, 85, 85),       // Red
            accent: Color::Rgb(255, 121, 198),    // Pink
            border: Color::Rgb(68, 71, 90),       // Current Line
        }
    }

    pub fn one_dark() -> Self {
        Self {
            name: "One Dark".into(),
            is_dark: true,
            bg: Color::Rgb(40, 44, 52),           // Hue1
            surface: Color::Rgb(49, 53, 63),      // Hue2
            fg: Color::Rgb(171, 178, 191),        // Hue6
            fg_muted: Color::Rgb(92, 99, 112),    // Hue3
            primary: Color::Rgb(97, 175, 239),    // Blue
            secondary: Color::Rgb(198, 120, 221), // Purple
            success: Color::Rgb(152, 195, 121),   // Green
            warning: Color::Rgb(229, 192, 123),   // Yellow
            error: Color::Rgb(224, 108, 117),     // Red
            accent: Color::Rgb(86, 182, 194),     // Cyan
            border: Color::Rgb(65, 70, 83),       // Hue2 dark
        }
    }

    pub fn ayu_dark() -> Self {
        Self {
            name: "Ayu Dark".into(),
            is_dark: true,
            bg: Color::Rgb(10, 14, 20),           // Background
            surface: Color::Rgb(13, 18, 26),      // Panel bg
            fg: Color::Rgb(203, 204, 198),        // Foreground
            fg_muted: Color::Rgb(90, 100, 115),   // Comment
            primary: Color::Rgb(89, 193, 251),    // Tag/blue
            secondary: Color::Rgb(162, 105, 221), // Keyword/purple
            success: Color::Rgb(172, 211, 115),   // String/green
            warning: Color::Rgb(252, 190, 68),    // Warning/orange
            error: Color::Rgb(255, 85, 85),       // Error/red
            accent: Color::Rgb(89, 193, 251),     // Function/teal
            border: Color::Rgb(30, 37, 48),       // Panel border
        }
    }

    pub fn ayu_mirage() -> Self {
        Self {
            name: "Ayu Mirage".into(),
            is_dark: true,
            bg: Color::Rgb(31, 35, 48),           // Background
            surface: Color::Rgb(39, 43, 58),      // Active line
            fg: Color::Rgb(203, 204, 198),        // Foreground
            fg_muted: Color::Rgb(91, 97, 115),    // Comment
            primary: Color::Rgb(115, 208, 255),   // Blue
            secondary: Color::Rgb(210, 160, 255), // Purple
            success: Color::Rgb(186, 230, 126),   // Green
            warning: Color::Rgb(255, 204, 102),   // Orange
            error: Color::Rgb(255, 106, 106),     // Red
            accent: Color::Rgb(95, 212, 213),     // Cyan
            border: Color::Rgb(55, 60, 80),       // Border
        }
    }

    pub fn kanagawa() -> Self {
        Self {
            name: "Kanagawa".into(),
            is_dark: true,
            bg: Color::Rgb(31, 31, 40),           // Sumi Ink 0
            surface: Color::Rgb(42, 42, 58),      // Sumi Ink 1
            fg: Color::Rgb(220, 215, 186),        // Fuji White
            fg_muted: Color::Rgb(114, 113, 105),  // Sumi Ink 4
            primary: Color::Rgb(126, 156, 216),   // Crystal Blue
            secondary: Color::Rgb(210, 126, 153), // Sakura Pink
            success: Color::Rgb(152, 187, 108),   // Spring Green
            warning: Color::Rgb(255, 160, 102),   // Carp Yellow
            error: Color::Rgb(195, 64, 67),       // Peach Red
            accent: Color::Rgb(106, 168, 145),    // Wave Aqua
            border: Color::Rgb(54, 54, 70),       // Sumi Ink 2
        }
    }

    pub fn rose_pine() -> Self {
        Self {
            name: "Rosé Pine".into(),
            is_dark: true,
            bg: Color::Rgb(25, 23, 36),           // Base
            surface: Color::Rgb(31, 29, 46),      // Surface
            fg: Color::Rgb(224, 222, 244),        // Text
            fg_muted: Color::Rgb(110, 106, 134),  // Muted
            primary: Color::Rgb(156, 207, 216),   // Foam
            secondary: Color::Rgb(196, 167, 231), // Iris
            success: Color::Rgb(49, 116, 143),    // Pine
            warning: Color::Rgb(246, 193, 119),   // Gold
            error: Color::Rgb(235, 111, 146),     // Love
            accent: Color::Rgb(235, 188, 186),    // Rose
            border: Color::Rgb(38, 35, 58),       // Overlay
        }
    }

    pub fn rose_pine_moon() -> Self {
        Self {
            name: "Rosé Pine Moon".into(),
            is_dark: true,
            bg: Color::Rgb(35, 33, 54),           // Base
            surface: Color::Rgb(42, 39, 63),      // Surface
            fg: Color::Rgb(224, 222, 244),        // Text
            fg_muted: Color::Rgb(110, 106, 134),  // Muted
            primary: Color::Rgb(156, 207, 216),   // Foam
            secondary: Color::Rgb(196, 167, 231), // Iris
            success: Color::Rgb(62, 143, 176),    // Pine
            warning: Color::Rgb(246, 193, 119),   // Gold
            error: Color::Rgb(235, 111, 146),     // Love
            accent: Color::Rgb(234, 154, 151),    // Rose
            border: Color::Rgb(57, 53, 82),       // Overlay
        }
    }

    pub fn everforest_dark() -> Self {
        Self {
            name: "Everforest Dark".into(),
            is_dark: true,
            bg: Color::Rgb(45, 53, 47),           // bg0
            surface: Color::Rgb(53, 62, 55),      // bg1
            fg: Color::Rgb(211, 198, 170),        // fg
            fg_muted: Color::Rgb(131, 139, 123),  // grey1
            primary: Color::Rgb(127, 187, 179),   // aqua
            secondary: Color::Rgb(209, 134, 149), // pink
            success: Color::Rgb(167, 192, 128),   // green
            warning: Color::Rgb(219, 188, 127),   // yellow
            error: Color::Rgb(230, 126, 128),     // red
            accent: Color::Rgb(131, 192, 146),    // green2
            border: Color::Rgb(65, 75, 67),       // bg3
        }
    }

    pub fn gruvbox_light() -> Self {
        Self {
            name: "Gruvbox Light".into(),
            is_dark: false,
            bg: Color::Rgb(251, 241, 199),       // Light0
            surface: Color::Rgb(242, 229, 188),  // Light1
            fg: Color::Rgb(60, 56, 54),          // Dark0
            fg_muted: Color::Rgb(102, 92, 84),   // Dark2
            primary: Color::Rgb(7, 102, 120),    // Dark Aqua
            secondary: Color::Rgb(142, 68, 173), // Dark Purple
            success: Color::Rgb(121, 116, 14),   // Dark Green
            warning: Color::Rgb(181, 118, 20),   // Dark Orange
            error: Color::Rgb(157, 0, 6),        // Dark Red
            accent: Color::Rgb(66, 123, 88),     // Dark Aqua2
            border: Color::Rgb(213, 196, 161),   // Light2
        }
    }

    // Light themes

    pub fn solarized_light() -> Self {
        Self {
            name: "Solarized Light".into(),
            is_dark: false,
            bg: Color::Rgb(253, 246, 227),        // base3
            surface: Color::Rgb(238, 232, 213),   // base2
            fg: Color::Rgb(101, 123, 131),        // base00
            fg_muted: Color::Rgb(147, 161, 161),  // base1
            primary: Color::Rgb(38, 139, 210),    // blue
            secondary: Color::Rgb(108, 113, 196), // violet
            success: Color::Rgb(133, 153, 0),     // green
            warning: Color::Rgb(181, 137, 0),     // yellow
            error: Color::Rgb(220, 50, 47),       // red
            accent: Color::Rgb(42, 161, 152),     // cyan
            border: Color::Rgb(210, 200, 178),    // base2 dark
        }
    }

    pub fn github_light() -> Self {
        Self {
            name: "GitHub Light".into(),
            is_dark: false,
            bg: Color::Rgb(255, 255, 255),       // Canvas default
            surface: Color::Rgb(246, 248, 250),  // Canvas subtle
            fg: Color::Rgb(31, 35, 40),          // fg default
            fg_muted: Color::Rgb(101, 109, 118), // fg muted
            primary: Color::Rgb(9, 105, 218),    // Accent fg
            secondary: Color::Rgb(130, 80, 223), // Done
            success: Color::Rgb(26, 127, 55),    // Success fg
            warning: Color::Rgb(154, 103, 0),    // Attention fg
            error: Color::Rgb(207, 34, 46),      // Danger fg
            accent: Color::Rgb(0, 135, 145),     // Sponsors
            border: Color::Rgb(208, 215, 222),   // Border default
        }
    }

    pub fn github_dark() -> Self {
        Self {
            name: "GitHub Dark".into(),
            is_dark: true,
            bg: Color::Rgb(13, 17, 23),           // Canvas default
            surface: Color::Rgb(22, 27, 34),      // Canvas subtle
            fg: Color::Rgb(230, 237, 243),        // fg default
            fg_muted: Color::Rgb(125, 133, 144),  // fg muted
            primary: Color::Rgb(88, 166, 255),    // Accent fg
            secondary: Color::Rgb(188, 140, 255), // Done
            success: Color::Rgb(63, 185, 80),     // Success fg
            warning: Color::Rgb(210, 153, 34),    // Attention fg
            error: Color::Rgb(248, 81, 73),       // Danger fg
            accent: Color::Rgb(56, 189, 248),     // Sponsors
            border: Color::Rgb(48, 54, 61),       // Border default
        }
    }

    pub fn one_light() -> Self {
        Self {
            name: "One Light".into(),
            is_dark: false,
            bg: Color::Rgb(250, 250, 250),       // Mono3
            surface: Color::Rgb(241, 241, 241),  // Mono2
            fg: Color::Rgb(56, 58, 66),          // Mono1
            fg_muted: Color::Rgb(160, 161, 167), // Mono3 dark
            primary: Color::Rgb(64, 120, 242),   // Blue
            secondary: Color::Rgb(166, 38, 164), // Purple
            success: Color::Rgb(80, 161, 79),    // Green
            warning: Color::Rgb(193, 132, 1),    // Orange
            error: Color::Rgb(224, 56, 44),      // Red
            accent: Color::Rgb(1, 132, 188),     // Cyan
            border: Color::Rgb(220, 220, 220),   // Border
        }
    }

    pub fn ayu_light() -> Self {
        Self {
            name: "Ayu Light".into(),
            is_dark: false,
            bg: Color::Rgb(250, 250, 240),        // Background
            surface: Color::Rgb(242, 242, 232),   // Active line
            fg: Color::Rgb(90, 98, 112),          // Foreground
            fg_muted: Color::Rgb(160, 165, 175),  // Comment
            primary: Color::Rgb(55, 142, 204),    // Tag/blue
            secondary: Color::Rgb(162, 105, 176), // Keyword
            success: Color::Rgb(134, 179, 0),     // String/green
            warning: Color::Rgb(247, 135, 0),     // Warning
            error: Color::Rgb(211, 57, 41),       // Error
            accent: Color::Rgb(0, 160, 197),      // Function
            border: Color::Rgb(222, 222, 210),    // Border
        }
    }

    pub fn everforest_light() -> Self {
        Self {
            name: "Everforest Light".into(),
            is_dark: false,
            bg: Color::Rgb(253, 246, 227),        // bg0
            surface: Color::Rgb(245, 237, 215),   // bg1
            fg: Color::Rgb(92, 96, 89),           // fg
            fg_muted: Color::Rgb(157, 157, 145),  // grey1
            primary: Color::Rgb(53, 133, 120),    // aqua
            secondary: Color::Rgb(196, 101, 115), // pink
            success: Color::Rgb(109, 144, 74),    // green
            warning: Color::Rgb(223, 152, 70),    // yellow
            error: Color::Rgb(195, 75, 72),       // red
            accent: Color::Rgb(88, 148, 103),     // green2
            border: Color::Rgb(228, 218, 194),    // bg3
        }
    }

    // Special / Novelty themes

    /// High-energy neon palette inspired by cyberpunk aesthetics. Pure black base
    /// with electric magenta, cyan, and green accents.
    pub fn cyberpunk() -> Self {
        Self {
            name: "Cyberpunk Neon".into(),
            is_dark: true,
            bg: Color::Rgb(0, 0, 10),           // Near-black
            surface: Color::Rgb(10, 0, 30),     // Deep navy-black
            fg: Color::Rgb(0, 255, 200),        // Electric cyan-green
            fg_muted: Color::Rgb(80, 180, 160), // Dim cyan
            primary: Color::Rgb(0, 240, 255),   // Electric cyan
            secondary: Color::Rgb(255, 0, 200), // Hot magenta
            success: Color::Rgb(0, 255, 80),    // Matrix green
            warning: Color::Rgb(255, 200, 0),   // Neon yellow
            error: Color::Rgb(255, 0, 80),      // Neon red
            accent: Color::Rgb(180, 0, 255),    // Electric violet
            border: Color::Rgb(0, 80, 100),     // Dark teal border
        }
    }

    /// Phosphor green-on-black retro terminal. Mimics a vintage CRT monitor.
    pub fn retro_terminal() -> Self {
        Self {
            name: "Retro Terminal".into(),
            is_dark: true,
            bg: Color::Rgb(0, 8, 0),           // Near-black with green tint
            surface: Color::Rgb(0, 18, 0),     // Slightly lighter scanline row
            fg: Color::Rgb(0, 230, 0),         // P1 phosphor green
            fg_muted: Color::Rgb(0, 130, 0),   // Dim phosphor
            primary: Color::Rgb(0, 255, 0),    // Bright phosphor
            secondary: Color::Rgb(0, 200, 80), // Teal-green
            success: Color::Rgb(80, 255, 80),  // Bright green
            warning: Color::Rgb(200, 230, 0),  // Amber-green
            error: Color::Rgb(255, 50, 50),    // Alert red (off-phosphor)
            accent: Color::Rgb(0, 255, 120),   // Bright teal
            border: Color::Rgb(0, 50, 0),      // Dark green border
        }
    }

    /// Amber phosphor retro terminal. Mimics old IBM 3279 amber CRT displays.
    pub fn retro_amber() -> Self {
        Self {
            name: "Retro Amber".into(),
            is_dark: true,
            bg: Color::Rgb(12, 7, 0),           // Near-black with warm tint
            surface: Color::Rgb(22, 14, 0),     // Slightly lighter
            fg: Color::Rgb(255, 176, 0),        // Amber phosphor
            fg_muted: Color::Rgb(160, 100, 0),  // Dim amber
            primary: Color::Rgb(255, 200, 30),  // Bright amber
            secondary: Color::Rgb(255, 140, 0), // Orange-amber
            success: Color::Rgb(200, 220, 0),   // Yellow-green
            warning: Color::Rgb(255, 200, 0),   // Bright amber
            error: Color::Rgb(255, 60, 0),      // Red-orange alert
            accent: Color::Rgb(255, 220, 80),   // Warm highlight
            border: Color::Rgb(60, 35, 0),      // Dark amber border
        }
    }

    /// Maximum legibility for visual accessibility. Near-pure black/white with
    /// strong semantic accent colors. WCAG AAA targets throughout.
    pub fn high_contrast() -> Self {
        Self {
            name: "High Contrast".into(),
            is_dark: true,
            bg: Color::Rgb(0, 0, 0),              // Pure black
            surface: Color::Rgb(20, 20, 20),      // Near-black
            fg: Color::Rgb(255, 255, 255),        // Pure white
            fg_muted: Color::Rgb(200, 200, 200),  // Light grey
            primary: Color::Rgb(0, 180, 255),     // Sky blue
            secondary: Color::Rgb(200, 100, 255), // Violet
            success: Color::Rgb(0, 230, 80),      // Vivid green
            warning: Color::Rgb(255, 210, 0),     // Vivid yellow
            error: Color::Rgb(255, 80, 80),       // Vivid red
            accent: Color::Rgb(0, 240, 220),      // Vivid cyan
            border: Color::Rgb(100, 100, 100),    // Mid grey
        }
    }

    /// High contrast light theme for daytime / outdoor readability.
    pub fn high_contrast_light() -> Self {
        Self {
            name: "High Contrast Light".into(),
            is_dark: false,
            bg: Color::Rgb(255, 255, 255),      // Pure white
            surface: Color::Rgb(240, 240, 240), // Near-white
            fg: Color::Rgb(0, 0, 0),            // Pure black
            fg_muted: Color::Rgb(60, 60, 60),   // Dark grey
            primary: Color::Rgb(0, 80, 180),    // Deep blue
            secondary: Color::Rgb(100, 0, 160), // Deep violet
            success: Color::Rgb(0, 120, 20),    // Deep green
            warning: Color::Rgb(140, 80, 0),    // Deep orange
            error: Color::Rgb(180, 0, 0),       // Deep red
            accent: Color::Rgb(0, 120, 120),    // Deep teal
            border: Color::Rgb(160, 160, 160),  // Mid grey
        }
    }

    /// Soft pastel palette — lower contrast, pleasant for long reading sessions
    /// in low-light environments. Not WCAG AA for small text; use at your own risk.
    pub fn pastel() -> Self {
        Self {
            name: "Pastel".into(),
            is_dark: true,
            bg: Color::Rgb(32, 28, 36),           // Warm near-black
            surface: Color::Rgb(44, 40, 50),      // Slight lift
            fg: Color::Rgb(220, 210, 230),        // Pale lavender white
            fg_muted: Color::Rgb(150, 140, 165),  // Muted lavender
            primary: Color::Rgb(160, 190, 240),   // Periwinkle
            secondary: Color::Rgb(210, 170, 230), // Soft lilac
            success: Color::Rgb(180, 225, 175),   // Sage green
            warning: Color::Rgb(240, 220, 170),   // Soft butter
            error: Color::Rgb(240, 175, 190),     // Rose
            accent: Color::Rgb(170, 225, 215),    // Pale mint
            border: Color::Rgb(65, 58, 78),       // Muted border
        }
    }

    /// Downsamples all TrueColor tokens to the specified terminal capability profile.
    pub fn downsample(&self, profile: ColorProfile) -> Self {
        let map_color = |c: Color| -> Color {
            match (profile, c) {
                (ColorProfile::TrueColor, _) => c,
                (ColorProfile::Ansi256, Color::Rgb(r, g, b)) => downsample_to_ansi256(r, g, b),
                (ColorProfile::Ansi16, Color::Rgb(r, g, b)) => downsample_to_ansi16(r, g, b),
                _ => c,
            }
        };

        Self {
            name: format!("{} ({:?})", self.name, profile),
            is_dark: self.is_dark,
            bg: map_color(self.bg),
            surface: map_color(self.surface),
            fg: map_color(self.fg),
            fg_muted: map_color(self.fg_muted),
            primary: map_color(self.primary),
            secondary: map_color(self.secondary),
            success: map_color(self.success),
            warning: map_color(self.warning),
            error: map_color(self.error),
            accent: map_color(self.accent),
            border: map_color(self.border),
        }
    }
}

/// Computes the ITU-R BT.709 relative luminance L of an sRGB color.
pub fn relative_luminance(color: Color) -> f32 {
    let (r, g, b) = match color {
        Color::Rgb(r, g, b) => (r, g, b),
        Color::White => (255, 255, 255),
        Color::Black => (0, 0, 0),
        Color::Red | Color::LightRed => (255, 0, 0),
        Color::Green | Color::LightGreen => (0, 255, 0),
        Color::Yellow | Color::LightYellow => (255, 255, 0),
        Color::Blue | Color::LightBlue => (0, 0, 255),
        Color::Magenta | Color::LightMagenta => (255, 0, 255),
        Color::Cyan | Color::LightCyan => (0, 255, 255),
        Color::Gray => (128, 128, 128),
        Color::DarkGray => (64, 64, 64),
        _ => (128, 128, 128),
    };

    let linearize = |v: u8| -> f32 {
        let s = v as f32 / 255.0;
        if s <= 0.04045 {
            s / 12.92
        } else {
            ((s + 0.055) / 1.055).powf(2.4)
        }
    };

    0.2126 * linearize(r) + 0.7152 * linearize(g) + 0.0722 * linearize(b)
}

/// Computes WCAG 2.1 contrast ratio C in [1.0, 21.0].
/// C = (L1 + 0.05) / (L2 + 0.05)
pub fn contrast_ratio(fg: Color, bg: Color) -> f32 {
    let l_fg = relative_luminance(fg);
    let l_bg = relative_luminance(bg);

    let l1 = l_fg.max(l_bg);
    let l2 = l_fg.min(l_bg);

    (l1 + 0.05) / (l2 + 0.05)
}

/// WCAG 2.1 contrast auditor ensuring contrast ratio >= min_ratio (default 4.5 for WCAG AA).
/// If below threshold, dynamically adjusts foreground luminance to meet compliance.
pub fn ensure_contrast(fg: Color, bg: Color, min_ratio: f32) -> Color {
    let ratio = contrast_ratio(fg, bg);
    if ratio >= min_ratio {
        return fg;
    }

    let (mut r, mut g, mut b) = match fg {
        Color::Rgb(r, g, b) => (r as f32, g as f32, b as f32),
        _ => (255.0, 255.0, 255.0),
    };

    let bg_lum = relative_luminance(bg);
    // If background is dark, boost toward white; if light, attenuate toward black
    let target = if bg_lum < 0.5 { 255.0 } else { 0.0 };

    for _ in 0..20 {
        r = r + 0.15 * (target - r);
        g = g + 0.15 * (target - g);
        b = b + 0.15 * (target - b);

        let adjusted = Color::Rgb(r.round() as u8, g.round() as u8, b.round() as u8);
        if contrast_ratio(adjusted, bg) >= min_ratio {
            return adjusted;
        }
    }

    if bg_lum < 0.5 {
        Color::Rgb(255, 255, 255)
    } else {
        Color::Rgb(0, 0, 0)
    }
}

/// Picks a legible foreground for `bg`: white on a dark background, black on a light one (by
/// BT.709 luminance). Use this instead of a hardcoded `Color::White` wherever a widget paints
/// text over a theme- or caller-supplied background, so a light theme (e.g. Catppuccin Latte)
/// never renders white-on-white. For a color that must stay recognizable (an accent, a status
/// hue), prefer [`ensure_contrast`] to nudge it only when it fails contrast.
pub fn readable_on(bg: Color) -> Color {
    if relative_luminance(bg) > 0.5 {
        Color::Black
    } else {
        Color::White
    }
}

#[cfg(test)]
mod contrast_tests {
    use super::*;

    #[test]
    fn readable_on_picks_contrasting_ink() {
        // Light backgrounds (e.g. Catppuccin Latte surface) get black ink; dark get white.
        assert_eq!(readable_on(Color::Rgb(239, 241, 245)), Color::Black);
        assert_eq!(readable_on(Color::Rgb(204, 208, 218)), Color::Black);
        assert_eq!(readable_on(Color::Rgb(30, 30, 46)), Color::White);
        assert_eq!(readable_on(Color::White), Color::Black);
        assert_eq!(readable_on(Color::Black), Color::White);
        // And the chosen ink always clears a legibility threshold on that background.
        for bg in [Color::Rgb(239, 241, 245), Color::Rgb(30, 30, 46), Color::Gray] {
            assert!(contrast_ratio(readable_on(bg), bg) >= 3.0);
        }
    }
}
