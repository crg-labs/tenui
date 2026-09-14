use tenui_core::{canvas::CanvasSubviewMut, color::Color};

/// Border corner geometry archetypes for container frames.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BorderCorner {
    /// Standard rectangular corner: ┌ ┐ └ ┘
    Sharp,
    /// Rounded corner: ╭ ╮ ╰ ╯
    Rounded,
    /// Diagonal triangular subpixel quadrant: ◤ ◥ ◣ ◢
    Chamfered,
    /// Double line frame: ╔ ╗ ╚ ╝
    Double,
    /// Ultra-thin sub-cell border lines using 1/8th and half blocks (▔, ▀,  , ▄)
    SubCellThin,
    /// Directional asymmetric lighting pairing thin top/left lines with heavy bottom/right fractional blocks (▄, ▌)
    AsymmetricOcclusion,
}

/// Configuration for advanced border geometry and surface framing.
#[derive(Clone, Debug, PartialEq)]
pub struct FrameConfig {
    pub corner: BorderCorner,
    pub border_color: Color,
    pub background: Option<Color>,
    pub badge_cutout: Option<String>,
}

impl Default for FrameConfig {
    fn default() -> Self {
        Self {
            corner: BorderCorner::Sharp,
            border_color: Color::WHITE,
            background: None,
            badge_cutout: None,
        }
    }
}

impl FrameConfig {
    pub fn with_corner(mut self, corner: BorderCorner) -> Self {
        self.corner = corner;
        self
    }

    pub fn with_color(mut self, color: Color) -> Self {
        self.border_color = color;
        self
    }

    pub fn with_background(mut self, bg: Color) -> Self {
        self.background = Some(bg);
        self
    }

    pub fn with_badge(mut self, badge: impl Into<String>) -> Self {
        self.badge_cutout = Some(badge.into());
        self
    }
}

pub struct BorderCompositor;

impl BorderCompositor {
    /// Renders high-resolution border framing with optional sub-cell geometry and embedded cutout badges.
    pub fn render_frame(surface: &mut CanvasSubviewMut, config: &FrameConfig) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;
        if w < 2 || h < 2 {
            return;
        }

        let bg = config.background.unwrap_or(Color::TRANSPARENT);

        match config.corner {
            BorderCorner::AsymmetricOcclusion => {
                // Top-left lighting simulation:
                // Top border: '─', Left border: '│'
                // Bottom border: '▄', Right border: '▌'
                // Corners:
                surface.set_cell_styled(0, 0, '┌', config.border_color, bg);
                surface.set_cell_styled(w - 1, 0, '┐', config.border_color, bg);
                surface.set_cell_styled(0, h - 1, '└', config.border_color, bg);
                surface.set_cell_styled(w - 1, h - 1, '▟', config.border_color, bg);

                for y in 1..(h - 1) {
                    surface.set_cell_styled(0, y, '│', config.border_color, bg);
                    surface.set_cell_styled(w - 1, y, '▌', config.border_color, bg);
                }

                for x in 1..(w - 1) {
                    surface.set_cell_styled(x, h - 1, '▄', config.border_color, bg);
                }

                Self::render_top_edge(surface, w, config, bg, '─');
            }
            BorderCorner::SubCellThin => {
                // Top: '▔' (U+2594), Bottom: ' ' (U+2581), Sides: '│'
                surface.set_cell_styled(0, 0, '┌', config.border_color, bg);
                surface.set_cell_styled(w - 1, 0, '┐', config.border_color, bg);
                surface.set_cell_styled(0, h - 1, '└', config.border_color, bg);
                surface.set_cell_styled(w - 1, h - 1, '┘', config.border_color, bg);

                for y in 1..(h - 1) {
                    surface.set_cell_styled(0, y, '│', config.border_color, bg);
                    surface.set_cell_styled(w - 1, y, '│', config.border_color, bg);
                }

                for x in 1..(w - 1) {
                    surface.set_cell_styled(x, h - 1, ' ', config.border_color, bg);
                }

                Self::render_top_edge(surface, w, config, bg, '▔');
            }
            BorderCorner::Sharp | BorderCorner::Rounded | BorderCorner::Chamfered | BorderCorner::Double => {
                let (c_tl, c_tr, c_bl, c_br, v_edge, h_edge) = match config.corner {
                    BorderCorner::Rounded => ('╭', '╮', '╰', '╯', '│', '─'),
                    BorderCorner::Chamfered => ('◤', '◥', '◣', '◢', '│', '─'),
                    BorderCorner::Double => ('╔', '╗', '╚', '╝', '║', '═'),
                    // Sharp (and, defensively, anything else routed here) renders square.
                    _ => ('┌', '┐', '└', '┘', '│', '─'),
                };

                // Render Corners
                surface.set_cell_styled(0, 0, c_tl, config.border_color, bg);
                surface.set_cell_styled(w - 1, 0, c_tr, config.border_color, bg);
                surface.set_cell_styled(0, h - 1, c_bl, config.border_color, bg);
                surface.set_cell_styled(w - 1, h - 1, c_br, config.border_color, bg);

                // Render Vertical Edges
                for y in 1..(h - 1) {
                    surface.set_cell_styled(0, y, v_edge, config.border_color, bg);
                    surface.set_cell_styled(w - 1, y, v_edge, config.border_color, bg);
                }

                // Render Bottom Edge
                for x in 1..(w - 1) {
                    surface.set_cell_styled(x, h - 1, h_edge, config.border_color, bg);
                }

                Self::render_top_edge(surface, w, config, bg, h_edge);
            }
        }
    }

    fn render_top_edge(surface: &mut CanvasSubviewMut, w: u16, config: &FrameConfig, bg: Color, h_edge: char) {
        if let Some(badge) = &config.badge_cutout {
            let badge_len = badge.chars().count() as u16;
            let start_x = 2u16;

            if start_x + badge_len + 2 < w {
                // Left border run
                for x in 1..(start_x - 1) {
                    surface.set_cell_styled(x, 0, h_edge, config.border_color, bg);
                }
                // Left Junction
                let left_junction = if config.corner == BorderCorner::Double {
                    '╣'
                } else {
                    '┤'
                };
                surface.set_cell_styled(start_x - 1, 0, left_junction, config.border_color, bg);

                // Badge Content
                surface.write_str_clipped(start_x, 0, badge, Color::WHITE, config.border_color);

                // Right Junction
                let right_junction = if config.corner == BorderCorner::Double {
                    '╠'
                } else {
                    '├'
                };
                surface.set_cell_styled(start_x + badge_len, 0, right_junction, config.border_color, bg);

                // Right border run
                for x in (start_x + badge_len + 1)..(w - 1) {
                    surface.set_cell_styled(x, 0, h_edge, config.border_color, bg);
                }
                return;
            }
        }

        // Standard Top Edge without Cutout
        for x in 1..(w - 1) {
            surface.set_cell_styled(x, 0, h_edge, config.border_color, bg);
        }
    }
}
