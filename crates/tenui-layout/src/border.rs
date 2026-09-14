use tenui_core::{CanvasSubviewMut, Color, Modifier, TextOverflow};

/// Border drawing characters.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BorderStyle {
    pub top_left: char,
    pub top_right: char,
    pub bottom_left: char,
    pub bottom_right: char,
    pub horizontal: char,
    pub vertical: char,
}

impl BorderStyle {
    // Standard Unicode box-drawing

    /// Thin single-line box (alias: `SINGLE`).
    pub const PLAIN: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '─',
        vertical: '│',
    };

    pub const SINGLE: Self = Self::PLAIN;

    /// Thin single-line with rounded corners — softest-looking option.
    pub const ROUNDED: Self = Self {
        top_left: '╭',
        top_right: '╮',
        bottom_left: '╰',
        bottom_right: '╯',
        horizontal: '─',
        vertical: '│',
    };

    /// Double-line — classic DOS/TUI dialog boxes.
    pub const DOUBLE: Self = Self {
        top_left: '╔',
        top_right: '╗',
        bottom_left: '╚',
        bottom_right: '╝',
        horizontal: '═',
        vertical: '║',
    };

    /// Heavy/bold single-line — strong emphasis.
    pub const THICK: Self = Self {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '━',
        vertical: '┃',
    };

    /// Plain ASCII — works on any terminal, no Unicode required.
    pub const ASCII: Self = Self {
        top_left: '+',
        top_right: '+',
        bottom_left: '+',
        bottom_right: '+',
        horizontal: '-',
        vertical: '|',
    };

    // Dashed & dotted

    /// Thin dashed lines — lighter visual weight than PLAIN.
    pub const DASHED: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '╌',
        vertical: '╎',
    };

    /// Heavy dashed lines.
    pub const DASHED_THICK: Self = Self {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '╍',
        vertical: '╏',
    };

    /// Double-dash thin.
    pub const DASH2: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '╌',
        vertical: '╎',
    };

    /// Triple-dash thin — more spaced out than DASHED.
    pub const DASH3: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '┄',
        vertical: '┆',
    };

    /// Triple-dash heavy.
    pub const DASH3_THICK: Self = Self {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '┅',
        vertical: '┇',
    };

    /// Quadruple-dash thin — fine dotted effect.
    pub const DASH4: Self = Self {
        top_left: '┌',
        top_right: '┐',
        bottom_left: '└',
        bottom_right: '┘',
        horizontal: '┈',
        vertical: '┊',
    };

    /// Quadruple-dash heavy.
    pub const DASH4_THICK: Self = Self {
        top_left: '┏',
        top_right: '┓',
        bottom_left: '┗',
        bottom_right: '┛',
        horizontal: '┉',
        vertical: '┋',
    };

    // Mixed weight (thin + thick combinations)

    /// Thick horizontal edges, thin vertical — "picture frame" look.
    pub const MIXED_HEAVY_H: Self = Self {
        top_left: '┍',
        top_right: '┑',
        bottom_left: '┕',
        bottom_right: '┙',
        horizontal: '━',
        vertical: '│',
    };

    /// Thin horizontal edges, thick vertical — "pillar" look.
    pub const MIXED_HEAVY_V: Self = Self {
        top_left: '┎',
        top_right: '┒',
        bottom_left: '┖',
        bottom_right: '┚',
        horizontal: '─',
        vertical: '┃',
    };

    // Block fill borders

    /// Full block █ — solid filled border, maximum visual weight.
    pub const BLOCK: Self = Self {
        top_left: '█',
        top_right: '█',
        bottom_left: '█',
        bottom_right: '█',
        horizontal: '█',
        vertical: '█',
    };

    /// Half-block ▀▄▌▐  creates a "raised edge" shadow illusion.
    pub const HALF_BLOCK: Self = Self {
        top_left: '▛',
        top_right: '▜',
        bottom_left: '▙',
        bottom_right: '▟',
        horizontal: '▀',
        vertical: '▌',
    };

    /// Outer half-block — top/right light, bottom/left dark (shadow effect).
    pub const SHADOW_BLOCK: Self = Self {
        top_left: '▗',
        top_right: '▖',
        bottom_left: '▝',
        bottom_right: '▘',
        horizontal: '▄',
        vertical: '▐',
    };

    // Decorative / symbolic

    /// Stars — `*` corners and edges. Classic "fancy" ASCII art border.
    pub const STARS: Self = Self {
        top_left: '*',
        top_right: '*',
        bottom_left: '*',
        bottom_right: '*',
        horizontal: '*',
        vertical: '*',
    };

    /// Hash — `#` everywhere. Dense decorative ASCII border.
    pub const HASH: Self = Self {
        top_left: '#',
        top_right: '#',
        bottom_left: '#',
        bottom_right: '#',
        horizontal: '#',
        vertical: '#',
    };

    /// Dot — `·` or `•` throughout. Light, airy feel.
    pub const DOTS: Self = Self {
        top_left: '·',
        top_right: '·',
        bottom_left: '·',
        bottom_right: '·',
        horizontal: '·',
        vertical: '·',
    };

    /// Bullet — heavier dot `•` variant.
    pub const BULLETS: Self = Self {
        top_left: '•',
        top_right: '•',
        bottom_left: '•',
        bottom_right: '•',
        horizontal: '•',
        vertical: '•',
    };

    /// Tilde `~` edges — relaxed, wave-like feel.
    pub const TILDE: Self = Self {
        top_left: '~',
        top_right: '~',
        bottom_left: '~',
        bottom_right: '~',
        horizontal: '~',
        vertical: '~',
    };

    /// Equals `=` horizontal, pipe `|` vertical — markdown-table look.
    pub const MARKDOWN: Self = Self {
        top_left: '+',
        top_right: '+',
        bottom_left: '+',
        bottom_right: '+',
        horizontal: '=',
        vertical: '|',
    };

    /// Braille dot pattern — almost invisible hairline border.
    pub const BRAILLE: Self = Self {
        top_left: '⡏',
        top_right: '⢹',
        bottom_left: '⣇',
        bottom_right: '⣸',
        horizontal: '⠉',
        vertical: '⡇',
    };

    // Double + thick hybrids

    /// Double outer line with thin inner connectors — two-layer depth.
    pub const DOUBLE_INNER: Self = Self {
        top_left: '╒',
        top_right: '╕',
        bottom_left: '╘',
        bottom_right: '╛',
        horizontal: '═',
        vertical: '│',
    };

    /// Thin horizontal, double vertical — "column" variant.
    pub const DOUBLE_SIDES: Self = Self {
        top_left: '╓',
        top_right: '╖',
        bottom_left: '╙',
        bottom_right: '╜',
        horizontal: '─',
        vertical: '║',
    };

    // Minimal / invisible

    /// Space border — completely invisible, just reserves margin cells.
    pub const BLANK: Self = Self {
        top_left: ' ',
        top_right: ' ',
        bottom_left: ' ',
        bottom_right: ' ',
        horizontal: ' ',
        vertical: ' ',
    };

    /// Arrow-head corners — directional feel, like a terminal prompt box.
    pub const ARROW: Self = Self {
        top_left: '►',
        top_right: '◄',
        bottom_left: '►',
        bottom_right: '◄',
        horizontal: '─',
        vertical: '│',
    };

    /// Diamond corners — elegant accent corners with plain edges.
    pub const DIAMOND: Self = Self {
        top_left: '◆',
        top_right: '◆',
        bottom_left: '◆',
        bottom_right: '◆',
        horizontal: '─',
        vertical: '│',
    };

    // Retro / game-inspired

    /// NES/SNES tile-box: square corners with bold lines. Classic game UI.
    pub const RETRO: Self = Self {
        top_left: '▄',
        top_right: '▄',
        bottom_left: '▀',
        bottom_right: '▀',
        horizontal: '▄',
        vertical: '█',
    };

    /// Outer glow suggestion: full-width block top/bottom, thin sides.
    pub const PANEL: Self = Self {
        top_left: '▐',
        top_right: '▌',
        bottom_left: '▐',
        bottom_right: '▌',
        horizontal: '▀',
        vertical: '│',
    };
}

/// Renders a styled border around the perimeter of a subview.
pub fn draw_border(subview: &mut CanvasSubviewMut<'_>, border: BorderStyle, fg: Color, bg: Color) {
    let w = subview.width();
    let h = subview.height();
    if w < 2 || h < 2 {
        return;
    }

    // Corners
    subview.set_char(0, 0, border.top_left, fg, bg, Modifier::empty());
    subview.set_char(w - 1, 0, border.top_right, fg, bg, Modifier::empty());
    subview.set_char(0, h - 1, border.bottom_left, fg, bg, Modifier::empty());
    subview.set_char(w - 1, h - 1, border.bottom_right, fg, bg, Modifier::empty());

    // Top and bottom edges
    for x in 1..w - 1 {
        subview.set_char(x, 0, border.horizontal, fg, bg, Modifier::empty());
        subview.set_char(x, h - 1, border.horizontal, fg, bg, Modifier::empty());
    }

    // Left and right edges
    for y in 1..h - 1 {
        subview.set_char(0, y, border.vertical, fg, bg, Modifier::empty());
        subview.set_char(w - 1, y, border.vertical, fg, bg, Modifier::empty());
    }
}

/// Renders a styled border around the perimeter of a subview with a header title,
/// protecting corner characters and handling overflow via `TextOverflow` (Ellipsis / Clip).
pub fn draw_border_with_title(
    subview: &mut CanvasSubviewMut<'_>,
    border: BorderStyle,
    title: &str,
    overflow: TextOverflow,
    fg: Color,
    bg: Color,
) {
    draw_border(subview, border, fg, bg);

    let trimmed = title.trim();
    if trimmed.is_empty() {
        return;
    }

    let w = subview.width();
    if w <= 6 {
        return; // Not enough room for borders, padding spaces, and title
    }

    // Available space for the raw title string:
    // Left border: 0, border horizontal: 1, padding space: 2, title: 3.., padding space: end, right border: w-1.
    let max_title_w = w.saturating_sub(5);

    // Padding space at x=2
    subview.set_char(2, 0, ' ', fg, bg, Modifier::empty());

    // Write title with overflow protection
    let written = subview.write_str_overflow(3, 0, trimmed, max_title_w, overflow, fg, bg);

    // Trailing padding space
    let pad_x = 3 + written;
    if pad_x < w - 1 {
        subview.set_char(pad_x, 0, ' ', fg, bg, Modifier::empty());
    }
}

/// Renders a styled border with a left title and an optional right badge,
/// preventing overlap, protecting corners, and applying `TextOverflow` cleanly.
pub fn draw_border_with_header(
    subview: &mut CanvasSubviewMut<'_>,
    border: BorderStyle,
    left_title: &str,
    right_badge: Option<&str>,
    overflow: TextOverflow,
    fg: Color,
    bg: Color,
) {
    draw_border(subview, border, fg, bg);

    let w = subview.width();
    if w <= 6 {
        return;
    }

    let trimmed_left = left_title.trim();
    let trimmed_right = right_badge.map(|s| s.trim()).unwrap_or("");

    // Calculate width needed for right badge
    let right_len = if trimmed_right.is_empty() {
        0
    } else {
        CanvasSubviewMut::measure_text_width(trimmed_right) + 2
    };

    // Right badge is placed at: w - 1 - right_len
    if right_len > 0 && w > right_len + 4 {
        let right_x = w - 1 - right_len;
        subview.set_char(right_x, 0, ' ', fg, bg, Modifier::empty());
        let _ = subview.write_str_overflow(right_x + 1, 0, trimmed_right, right_len - 2, TextOverflow::Clip, fg, bg);
        subview.set_char(right_x + right_len - 1, 0, ' ', fg, bg, Modifier::empty());
    }

    // Available space for left title:
    let max_left_w = if right_len > 0 {
        w.saturating_sub(right_len + 6)
    } else {
        w.saturating_sub(5)
    };

    if !trimmed_left.is_empty() && max_left_w > 0 {
        subview.set_char(2, 0, ' ', fg, bg, Modifier::empty());
        let written = subview.write_str_overflow(3, 0, trimmed_left, max_left_w, overflow, fg, bg);
        let pad_x = 3 + written;
        if pad_x < w - 1 && (right_len == 0 || pad_x < w - 1 - right_len) {
            subview.set_char(pad_x, 0, ' ', fg, bg, Modifier::empty());
        }
    }
}

/// Renders a collision-free two-sided bar (e.g. top navigation bar or status bar).
/// Guarantees that left-aligned content and right-aligned content never collide or overwrite each other.
/// If available width is constrained, `left_str` is truncated with `overflow` to protect the right-side summary.
#[allow(clippy::too_many_arguments)]
pub fn draw_split_header(
    subview: &mut CanvasSubviewMut<'_>,
    left_str: &str,
    left_fg: Color,
    left_bg: Color,
    right_str: &str,
    right_fg: Color,
    right_bg: Color,
    bar_bg: Color,
    overflow: TextOverflow,
) {
    let w = subview.width();
    if w == 0 {
        return;
    }

    // Clear row with bar_bg
    for x in 0..w {
        subview.set_char(x, 0, ' ', Color::Reset, bar_bg, Modifier::empty());
    }

    let right_len = CanvasSubviewMut::measure_text_width(right_str);
    if right_len >= w {
        subview.write_str_overflow(0, 0, right_str, w, TextOverflow::Ellipsis, right_fg, right_bg);
        return;
    }

    let right_x = w - right_len;
    subview.write_str_clipped(right_x, 0, right_str, right_fg, right_bg);

    let left_avail = right_x.saturating_sub(1);
    if left_avail > 0 && !left_str.is_empty() {
        subview.write_str_overflow(0, 0, left_str, left_avail, overflow, left_fg, left_bg);
    }
}

/// Renders a horizontal divider at `y` with optional section title and clean T-junction connectors.
pub fn draw_horizontal_divider(
    subview: &mut CanvasSubviewMut<'_>,
    y: u16,
    title: Option<&str>,
    border: BorderStyle,
    fg: Color,
    bg: Color,
) {
    let w = subview.width();
    let h = subview.height();
    if y >= h || w < 2 {
        return;
    }

    // Derive T-junction connectors from the border style's character set.
    let (left_conn, line_char, right_conn) = match border {
        BorderStyle::DOUBLE => ('╠', '═', '╣'),
        BorderStyle::DOUBLE_INNER => ('╞', '═', '╡'),
        BorderStyle::DOUBLE_SIDES => ('╟', '─', '╢'),
        BorderStyle::THICK | BorderStyle::DASHED_THICK | BorderStyle::DASH3_THICK | BorderStyle::DASH4_THICK => {
            ('┣', '━', '┫')
        }
        BorderStyle::MIXED_HEAVY_H => ('┝', '━', '┥'),
        BorderStyle::MIXED_HEAVY_V => ('┠', '─', '┨'),
        BorderStyle::ASCII | BorderStyle::MARKDOWN => ('+', border.horizontal, '+'),
        BorderStyle::BLOCK => ('█', '█', '█'),
        BorderStyle::HASH => ('#', '#', '#'),
        BorderStyle::STARS => ('*', '*', '*'),
        BorderStyle::DOTS | BorderStyle::BULLETS | BorderStyle::TILDE => {
            (border.horizontal, border.horizontal, border.horizontal)
        }
        BorderStyle::BRAILLE => ('⡏', '⠉', '⢹'),
        BorderStyle::BLANK => (' ', ' ', ' '),
        // All thin variants (PLAIN/ROUNDED/SINGLE/DASHED/DASH2/DASH3/DASH4/ARROW/DIAMOND/PANEL/HALF_BLOCK/SHADOW_BLOCK/RETRO) → standard thin T
        _ if border.horizontal == '─'
            || border.horizontal == '╌'
            || border.horizontal == '┄'
            || border.horizontal == '┈' =>
        {
            ('├', border.horizontal, '┤')
        }
        _ => ('├', border.horizontal, '┤'),
    };

    subview.set_char(0, y, left_conn, fg, bg, Modifier::empty());
    for x in 1..w.saturating_sub(1) {
        subview.set_char(x, y, line_char, fg, bg, Modifier::empty());
    }
    subview.set_char(w - 1, y, right_conn, fg, bg, Modifier::empty());

    if let Some(t) = title {
        let trimmed = t.trim();
        if !trimmed.is_empty() && w > 8 {
            let max_t = w.saturating_sub(6);
            subview.set_char(2, y, ' ', fg, bg, Modifier::empty());
            let written = subview.write_str_overflow(3, y, trimmed, max_t, TextOverflow::Ellipsis, fg, bg);
            let pad_x = 3 + written;
            if pad_x < w - 1 {
                subview.set_char(pad_x, y, ' ', fg, bg, Modifier::empty());
            }
        }
    }
}
