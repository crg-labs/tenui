use bitflags::bitflags;
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::color::Color;

bitflags! {
    /// ANSI text modifiers (SGR attributes).
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Modifier: u16 {
        const BOLD          = 0b0000_0000_0001;
        const DIM           = 0b0000_0000_0010;
        const ITALIC        = 0b0000_0000_0100;
        const UNDERLINE     = 0b0000_0000_1000;
        const BLINK         = 0b0000_0001_0000;
        const REVERSE       = 0b0000_0010_0000;
        const HIDDEN        = 0b0000_0100_0000;
        const STRIKETHROUGH = 0b0000_1000_0000;
    }
}

impl Modifier {
    /// An empty modifier set with no attributes.
    pub const PLAIN: Self = Self::empty();

    /// Combines modifier flags in a const context.
    #[inline]
    pub const fn with(self, other: Self) -> Self {
        self.union(other)
    }

    /// Removes modifier flags in a const context.
    #[inline]
    pub const fn without(self, other: Self) -> Self {
        self.difference(other)
    }

    /// Checks if specific modifier flags are set in a const context.
    #[inline]
    pub const fn has(self, other: Self) -> bool {
        self.contains(other)
    }
}

/// Extended underline styles (CSI 4:Xm).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum UnderlineStyle {
    #[default]
    None,
    Single, // CSI 4:1m
    Double, // CSI 4:2m
    Curly,  // CSI 4:3m
    Dotted, // CSI 4:4m
    Dashed, // CSI 4:5m
}

impl UnderlineStyle {
    pub fn sgr_code(&self) -> Option<&'static str> {
        match self {
            Self::None => Some("24"),
            Self::Single => Some("4:1"),
            Self::Double => Some("4:2"),
            Self::Curly => Some("4:3"),
            Self::Dotted => Some("4:4"),
            Self::Dashed => Some("4:5"),
        }
    }
}

/// Global interned string pool for complex Extended Grapheme Clusters (> 7 bytes).
///
/// A grapheme is leaked exactly once, for the process lifetime, so it can be handed
/// back as a cheap `&'static str` (letting `CompactSymbol::as_str` stay allocation-free).
/// The `ids` map gives O(1) dedup, so re-interning an already-seen grapheme neither
/// re-scans nor re-leaks. Bounded growth is inherent to interning-for-lifetime and is
/// capped by the number of *distinct* complex graphemes ever rendered.
#[derive(Default)]
struct EgcPool {
    by_id: Vec<&'static str>,
    ids: std::collections::HashMap<&'static str, u32>,
}

fn egc_pool() -> &'static std::sync::RwLock<EgcPool> {
    static POOL: std::sync::OnceLock<std::sync::RwLock<EgcPool>> = std::sync::OnceLock::new();
    POOL.get_or_init(|| std::sync::RwLock::new(EgcPool::default()))
}

/// Interns a complex grapheme cluster into the global pool, returning its 32-bit index.
pub fn intern_egc(s: &str) -> u32 {
    {
        let pool = egc_pool().read().unwrap();
        if let Some(&idx) = pool.ids.get(s) {
            return idx;
        }
    }
    let mut pool = egc_pool().write().unwrap();
    // Re-check: another thread may have interned `s` between the read and write locks.
    if let Some(&idx) = pool.ids.get(s) {
        return idx;
    }
    let leaked: &'static str = Box::leak(s.to_string().into_boxed_str());
    let idx = pool.by_id.len() as u32;
    pool.by_id.push(leaked);
    pool.ids.insert(leaked, idx);
    idx
}

/// Retrieves an interned grapheme cluster by pool index.
pub fn get_interned_egc(idx: u32) -> Option<&'static str> {
    let pool = egc_pool().read().unwrap();
    pool.by_id.get(idx as usize).copied()
}

/// Spillover descriptor referencing the interned EGC string pool.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, Default)]
pub struct CellSpillover {
    pub pool_index: u32,
    pub width: u8,
    pub _reserved: [u8; 3],
}

/// Low-level C-compatible cell data union for compact 8-byte inlined representation.
#[repr(C)]
#[derive(Clone, Copy)]
pub union CellData {
    pub inline: [u8; 8],
    pub spillover: CellSpillover,
}

/// A compact grapheme symbol representation holding up to 7 UTF-8 bytes inline,
/// or pointing to an interned EGC spillover pool for complex sequences (> 7 bytes).
#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub struct CompactSymbol {
    bytes: [u8; 7],
    len: u8,
}

impl Default for CompactSymbol {
    fn default() -> Self {
        let mut s = Self { bytes: [0; 7], len: 1 };
        s.bytes[0] = b' ';
        s
    }
}

impl CompactSymbol {
    pub const SPACE: Self = Self {
        bytes: [b' ', 0, 0, 0, 0, 0, 0],
        len: 1,
    };

    pub fn from_char(ch: char) -> Self {
        let mut bytes = [0u8; 7];
        let s = ch.encode_utf8(&mut bytes);
        let len = s.len() as u8;
        Self { bytes, len }
    }

    #[allow(clippy::should_implement_trait)]
    pub fn from_str(s: &str) -> Self {
        if s.len() <= 7 {
            let mut bytes = [0u8; 7];
            bytes[..s.len()].copy_from_slice(s.as_bytes());
            Self {
                bytes,
                len: s.len() as u8,
            }
        } else {
            let pool_idx = intern_egc(s);
            let mut bytes = [0u8; 7];
            bytes[0..4].copy_from_slice(&pool_idx.to_ne_bytes());
            Self {
                bytes,
                len: 0x80, // High bit indicates interned spillover
            }
        }
    }
}

impl std::str::FromStr for CompactSymbol {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        Ok(Self::from_str(s))
    }
}

impl CompactSymbol {
    pub fn is_spillover(&self) -> bool {
        (self.len & 0x80) != 0
    }

    pub fn is_empty(&self) -> bool {
        self.as_str().trim().is_empty()
    }

    pub fn as_str(&self) -> &str {
        if self.is_spillover() {
            let idx = u32::from_ne_bytes([self.bytes[0], self.bytes[1], self.bytes[2], self.bytes[3]]);
            get_interned_egc(idx).unwrap_or(" ")
        } else if self.len == 0 {
            " "
        } else {
            std::str::from_utf8(&self.bytes[..self.len as usize]).unwrap_or(" ")
        }
    }
}

impl std::fmt::Debug for CompactSymbol {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self.as_str())
    }
}

/// Host terminal capabilities for visual degradation cascading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TerminalCapabilities {
    pub has_utf8: bool,
    pub has_truecolor: bool,
    pub has_pixel_mouse: bool,
    pub has_bracketed_paste: bool,
}

impl Default for TerminalCapabilities {
    fn default() -> Self {
        Self {
            has_utf8: true,
            has_truecolor: true,
            has_pixel_mouse: true,
            has_bracketed_paste: true,
        }
    }
}

/// A single character cell on the terminal grid.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub symbol: CompactSymbol,
    pub fg: Color,
    pub bg: Color,
    pub modifier: Modifier,
    pub underline_color: Color,
    pub underline_style: UnderlineStyle,
    pub width: u8,
    pub is_continuation: bool,
}

impl Default for Cell {
    fn default() -> Self {
        Self {
            symbol: CompactSymbol::SPACE,
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            underline_color: Color::Reset,
            underline_style: UnderlineStyle::None,
            width: 1,
            is_continuation: false,
        }
    }
}

impl Cell {
    /// A precomputed blank cell with default attributes.
    pub const EMPTY: Self = Self {
        symbol: CompactSymbol::SPACE,
        fg: Color::Reset,
        bg: Color::Reset,
        modifier: Modifier::PLAIN,
        underline_color: Color::Reset,
        underline_style: UnderlineStyle::None,
        width: 1,
        is_continuation: false,
    };

    /// Returns a const blank cell.
    #[inline]
    pub const fn blank() -> Self {
        Self::EMPTY
    }

    /// A poison sentinel cell guaranteed to never match any valid rendered cell.
    /// Used by buffer invalidation to force differential rendering of every coordinate.
    pub const POISON: Self = Self {
        symbol: CompactSymbol {
            bytes: [0xFF, 0xFF, 0xFF, 0, 0, 0, 0],
            len: 3,
        },
        fg: Color::Reset,
        bg: Color::Reset,
        modifier: Modifier::PLAIN,
        underline_color: Color::Reset,
        underline_style: UnderlineStyle::None,
        width: 1,
        is_continuation: false,
    };

    /// Returns a const poison sentinel cell.
    #[inline]
    pub const fn poison() -> Self {
        Self::POISON
    }

    /// Creates an empty cell with a specific background color.
    #[inline]
    pub fn empty_with_bg(bg: Color) -> Self {
        let mut cell = Self::EMPTY;
        cell.bg = bg;
        cell
    }

    pub fn new(symbol: &str) -> Self {
        let width = UnicodeWidthStr::width(symbol) as u8;
        Self {
            symbol: CompactSymbol::from_str(symbol),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            underline_color: Color::Reset,
            underline_style: UnderlineStyle::None,
            width: width.max(1),
            is_continuation: false,
        }
    }

    pub fn from_char(ch: char) -> Self {
        let width = UnicodeWidthChar::width(ch).unwrap_or(1) as u8;
        Self {
            symbol: CompactSymbol::from_char(ch),
            fg: Color::Reset,
            bg: Color::Reset,
            modifier: Modifier::empty(),
            underline_color: Color::Reset,
            underline_style: UnderlineStyle::None,
            width: width.max(1),
            is_continuation: false,
        }
    }

    pub fn set_char(&mut self, ch: char) -> &mut Self {
        self.symbol = CompactSymbol::from_char(ch);
        self.width = UnicodeWidthChar::width(ch).unwrap_or(1).max(1) as u8;
        self.is_continuation = false;
        self
    }

    /// Merges an 8-bit Braille dot mask into this cell's Unicode Braille codepoint.
    pub fn merge_braille_dot(&mut self, bit: u8) -> &mut Self {
        let current_pattern = if let Some(ch) = self.symbol.as_str().chars().next() {
            let cp = ch as u32;
            if (0x2800..=0x28FF).contains(&cp) {
                (cp - 0x2800) as u8
            } else {
                0u8
            }
        } else {
            0u8
        };
        let new_pattern = current_pattern | bit;
        let ch = char::from_u32(0x2800 + new_pattern as u32).unwrap_or(' ');
        self.set_char(ch)
    }

    pub fn set_symbol(&mut self, s: &str) -> &mut Self {
        self.symbol = CompactSymbol::from_str(s);
        self.width = UnicodeWidthStr::width(s).max(1) as u8;
        self.is_continuation = false;
        self
    }

    pub fn set_fg(&mut self, color: Color) -> &mut Self {
        self.fg = color;
        self
    }

    pub fn set_bg(&mut self, color: Color) -> &mut Self {
        self.bg = color;
        self
    }

    pub fn set_modifier(&mut self, modifier: Modifier) -> &mut Self {
        self.modifier = modifier;
        self
    }

    pub fn set_underline_color(&mut self, color: Color) -> &mut Self {
        self.underline_color = color;
        self
    }

    pub fn set_underline_style(&mut self, style: UnderlineStyle) -> &mut Self {
        self.underline_style = style;
        self
    }

    pub fn as_str(&self) -> &str {
        self.symbol.as_str()
    }

    pub fn as_grapheme_str(&self) -> &str {
        self.symbol.as_str()
    }

    /// Resolves visual fallback cascade when running on legacy or non-UTF8 terminals.
    pub fn resolve_degradation(&self, caps: &TerminalCapabilities) -> (char, Color, Color) {
        if caps.has_utf8 {
            return (self.as_str().chars().next().unwrap_or(' '), self.fg, self.bg);
        }

        let fallback_char = match self.as_str() {
            "│" | "┃" => '|',
            "─" | "━" => '-',
            "┌" | "┏" => '+',
            "┐" | "┓" => '+',
            "└" | "┗" => '+',
            "┘" | "┛" => '+',
            "├" | "┣" => '+',
            "┤" | "┫" => '+',
            "┬" | "┳" => '+',
            "┴" | "┻" => '+',
            "┼" | "╋" => '+',
            "•" | "●" => '*',
            "✔" => 'V',
            "✖" => 'X',
            other => other.chars().next().unwrap_or('?'),
        };

        (fallback_char, self.fg, self.bg)
    }

    pub fn reset(&mut self) {
        *self = Self::default();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::color::Color;

    // ---- CompactSymbol tests ----

    #[test]
    fn compact_symbol_space() {
        assert_eq!(CompactSymbol::SPACE.as_str(), " ");
        assert!(!CompactSymbol::SPACE.is_spillover());
    }

    #[test]
    fn compact_symbol_from_char_ascii() {
        let s = CompactSymbol::from_char('A');
        assert_eq!(s.as_str(), "A");
        assert!(!s.is_spillover());
    }

    #[test]
    fn compact_symbol_from_char_multibyte() {
        let s = CompactSymbol::from_char('中');
        assert_eq!(s.as_str(), "中");
        assert!(!s.is_spillover());
    }

    #[test]
    fn compact_symbol_from_str_short() {
        let s = CompactSymbol::from_str("hello");
        assert_eq!(s.as_str(), "hello");
        assert!(!s.is_spillover());
    }

    #[test]
    fn compact_symbol_from_str_exact_seven() {
        let s = CompactSymbol::from_str("abcdefg");
        assert_eq!(s.as_str(), "abcdefg");
        assert!(!s.is_spillover());
    }

    #[test]
    fn compact_symbol_from_str_spillover() {
        let s = CompactSymbol::from_str("abcdefgh");
        assert_eq!(s.as_str(), "abcdefgh");
        assert!(s.is_spillover());
    }

    #[test]
    fn compact_symbol_from_str_long_egc() {
        let emoji = "👨‍👩‍👧‍👦";
        let s = CompactSymbol::from_str(emoji);
        assert_eq!(s.as_str(), emoji);
        assert!(s.is_spillover());
    }

    #[test]
    fn compact_symbol_is_empty() {
        assert!(CompactSymbol::SPACE.is_empty());
        assert!(!CompactSymbol::from_char('A').is_empty());
    }

    // ---- Cell tests ----

    #[test]
    fn cell_new() {
        let c = Cell::new("X");
        assert_eq!(c.as_str(), "X");
        assert_eq!(c.fg, Color::Reset);
        assert_eq!(c.bg, Color::Reset);
        assert_eq!(c.width, 1);
    }

    #[test]
    fn cell_new_wide() {
        let c = Cell::new("中");
        assert_eq!(c.as_str(), "中");
        assert_eq!(c.width, 2);
    }

    #[test]
    fn cell_from_char() {
        let c = Cell::from_char('Z');
        assert_eq!(c.as_str(), "Z");
        assert_eq!(c.width, 1);
        assert!(!c.is_continuation);
    }

    #[test]
    fn cell_set_char() {
        let mut c = Cell::default();
        c.set_char('Q');
        assert_eq!(c.as_str(), "Q");
        assert_eq!(c.width, 1);
    }

    #[test]
    fn cell_set_symbol() {
        let mut c = Cell::default();
        c.set_symbol("Hello");
        assert_eq!(c.as_str(), "Hello");
    }

    #[test]
    fn cell_reset() {
        let mut c = Cell::from_char('Z');
        c.set_fg(Color::Red);
        c.set_bg(Color::Blue);
        c.set_modifier(Modifier::BOLD);
        c.reset();
        assert_eq!(c, Cell::default());
    }

    #[test]
    fn cell_merge_braille_dot() {
        let mut c = Cell::default();
        c.set_char('\u{2800}');
        c.merge_braille_dot(0x01);
        assert_eq!(c.as_str(), "\u{2801}");
        c.merge_braille_dot(0x08);
        assert_eq!(c.as_str(), "\u{2809}");
    }

    #[test]
    fn cell_merge_braille_dot_non_braille() {
        let mut c = Cell::from_char('A');
        c.merge_braille_dot(0x01);
        assert_eq!(c.as_str(), "\u{2801}");
    }

    #[test]
    fn cell_resolve_degradation_utf8() {
        let caps = TerminalCapabilities::default();
        let c = Cell::new("│");
        let (ch, _, _) = c.resolve_degradation(&caps);
        assert_eq!(ch, '│');
    }

    #[test]
    fn cell_resolve_degradation_no_utf8() {
        let caps = TerminalCapabilities {
            has_utf8: false,
            ..Default::default()
        };
        let c = Cell::new("│");
        let (ch, _, _) = c.resolve_degradation(&caps);
        assert_eq!(ch, '|');
    }

    #[test]
    fn cell_resolve_degradation_box_chars() {
        let caps = TerminalCapabilities {
            has_utf8: false,
            ..Default::default()
        };
        let pairs = [
            ("─", '-'),
            ("┌", '+'),
            ("┐", '+'),
            ("└", '+'),
            ("┘", '+'),
            ("•", '*'),
            ("✔", 'V'),
            ("✖", 'X'),
        ];
        for (sym, expected) in pairs {
            let c = Cell::new(sym);
            let (ch, _, _) = c.resolve_degradation(&caps);
            assert_eq!(ch, expected, "degradation of {:?}", sym);
        }
    }

    // Modifier tests

    #[test]
    fn modifier_with() {
        let m = Modifier::BOLD.with(Modifier::ITALIC);
        assert!(m.has(Modifier::BOLD));
        assert!(m.has(Modifier::ITALIC));
        assert!(!m.has(Modifier::UNDERLINE));
    }

    #[test]
    fn modifier_without() {
        let m = Modifier::BOLD.with(Modifier::ITALIC).without(Modifier::BOLD);
        assert!(!m.has(Modifier::BOLD));
        assert!(m.has(Modifier::ITALIC));
    }

    #[test]
    fn modifier_plain_is_empty() {
        assert_eq!(Modifier::PLAIN, Modifier::empty());
        assert!(!Modifier::PLAIN.has(Modifier::BOLD));
    }

    // UnderlineStyle tests

    #[test]
    fn underline_style_sgr_codes() {
        assert_eq!(UnderlineStyle::None.sgr_code(), Some("24"));
        assert_eq!(UnderlineStyle::Single.sgr_code(), Some("4:1"));
        assert_eq!(UnderlineStyle::Double.sgr_code(), Some("4:2"));
        assert_eq!(UnderlineStyle::Curly.sgr_code(), Some("4:3"));
        assert_eq!(UnderlineStyle::Dotted.sgr_code(), Some("4:4"));
        assert_eq!(UnderlineStyle::Dashed.sgr_code(), Some("4:5"));
    }

    #[test]
    fn cell_empty_with_bg() {
        let c = Cell::empty_with_bg(Color::Red);
        assert_eq!(c.bg, Color::Red);
        assert_eq!(c.as_str(), " ");
    }

    #[test]
    fn cell_poison_differs_from_default() {
        assert_ne!(Cell::POISON, Cell::default());
        assert_ne!(Cell::POISON, Cell::EMPTY);
    }
}
