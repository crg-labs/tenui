//! Clickable Hyperlinks (OSC 8).

/// Formats text as an OSC 8 clickable terminal hyperlink:
/// `\x1b]8;;{url}\x1b\{text}\x1b]8;;\x1b\`
pub fn format_osc8_hyperlink(url: &str, text: &str) -> String {
    format!("\x1b]8;;{}\x1b\\{}\x1b]8;;\x1b\\", url, text)
}
