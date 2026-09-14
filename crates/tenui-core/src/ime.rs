//! IME Composition Anchoring and Hardware Cursor Sync.

/// Terminal hardware cursor shapes (DECSCUSR).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum CursorStyle {
    #[default]
    Default,
    BlinkingBlock,     // 1
    SteadyBlock,       // 2
    BlinkingUnderline, // 3
    SteadyUnderline,   // 4
    BlinkingBar,       // 5
    SteadyBar,         // 6
}

impl CursorStyle {
    pub fn decscusr_code(&self) -> u8 {
        match self {
            Self::Default => 0,
            Self::BlinkingBlock => 1,
            Self::SteadyBlock => 2,
            Self::BlinkingUnderline => 3,
            Self::SteadyUnderline => 4,
            Self::BlinkingBar => 5,
            Self::SteadyBar => 6,
        }
    }
}

/// Formats cursor position and style escape commands for hardware IME synchronization.
pub fn format_cursor_sync(x: u16, y: u16, style: CursorStyle) -> String {
    // CSI {style} q (DECSCUSR) and CSI {y};{x} H (CUP - 1-indexed)
    format!("\x1b[{} q\x1b[{};{}H\x1b[?25h", style.decscusr_code(), y + 1, x + 1)
}
