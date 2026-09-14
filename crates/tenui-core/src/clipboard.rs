//! Terminal clipboard bridge: OSC 52 copy-out plus a local register for in-app paste.
//!
//! Copy writes the system clipboard from inside the terminal via OSC 52
//! (`\x1b]52;c;<base64>\x07`). Most terminals disable OSC 52 clipboard *reads* for
//! security, so paste cannot reliably round-trip back through the terminal — the
//! [`Clipboard`] therefore also keeps a **local register** that every in-app copy
//! populates, giving Ctrl+C / Ctrl+V a consistent value regardless of terminal support.
//! System pastes still arrive out-of-band as bracketed paste
//! ([`crate::input::InputEvent::Paste`]).

use std::io::{self, Write};

/// A clipboard combining the system clipboard (OSC 52, write-only) with a local register.
#[derive(Debug, Clone, Default)]
pub struct Clipboard {
    register: String,
}

impl Clipboard {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the local register without touching the terminal.
    pub fn set_register(&mut self, text: impl Into<String>) {
        self.register = text.into();
    }

    /// The current local register contents (last in-app copy or programmatic set).
    pub fn register(&self) -> &str {
        &self.register
    }

    /// Copies `text`: stores it in the local register and returns the OSC 52 escape for
    /// the caller to write into its own output buffer (integrates with frame batching).
    #[must_use = "the returned OSC 52 escape must be written to the terminal to update the system clipboard"]
    pub fn copy(&mut self, text: &str) -> String {
        self.register = text.to_string();
        encode_osc52(text)
    }

    /// Copies `text`, writing the OSC 52 escape directly to `out` and flushing.
    pub fn copy_to<W: Write>(&mut self, out: &mut W, text: &str) -> io::Result<()> {
        self.register = text.to_string();
        out.write_all(encode_osc52(text).as_bytes())?;
        out.flush()
    }

    /// The text an in-app paste (e.g. Ctrl+V) should insert, the local register.
    ///
    /// System (bracketed) pastes are delivered separately as `InputEvent::Paste`.
    pub fn paste(&self) -> &str {
        &self.register
    }

    pub fn is_empty(&self) -> bool {
        self.register.is_empty()
    }

    pub fn clear(&mut self) {
        self.register.clear();
    }
}

/// A clipboard action, decoupled from the key chord that triggered it. Produced by an
/// input layer (e.g. `tenui-ext`'s `ClipboardLayer`) and applied to a focused
/// [`ClipboardTarget`] via [`apply_clipboard_command`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ClipboardCommand {
    Copy,
    Cut,
    Paste,
    SelectAll,
}

/// A widget that can participate in clipboard operations. Implemented by the text widgets
/// so one input path (Ctrl+C/X/V, bracketed paste) drives them all.
pub trait ClipboardTarget {
    /// Copies the current selection into `clip`; returns the OSC 52 escape to emit.
    fn clipboard_copy(&self, clip: &mut Clipboard) -> String;
    /// Cuts the current selection into `clip` (mutating the target); returns the escape.
    fn clipboard_cut(&mut self, clip: &mut Clipboard) -> String;
    /// Inserts `clip`'s register at the caret (replacing any selection).
    fn clipboard_paste(&mut self, clip: &Clipboard);
    /// Selects everything (default: no-op for targets without a selection model).
    fn clipboard_select_all(&mut self) {}
}

/// Applies a [`ClipboardCommand`] to a focused target, returning the OSC 52 escape to emit
/// (for Copy/Cut), or `None` (for Paste/SelectAll).
pub fn apply_clipboard_command(
    target: &mut dyn ClipboardTarget,
    command: ClipboardCommand,
    clip: &mut Clipboard,
) -> Option<String> {
    match command {
        ClipboardCommand::Copy => Some(target.clipboard_copy(clip)),
        ClipboardCommand::Cut => Some(target.clipboard_cut(clip)),
        ClipboardCommand::Paste => {
            target.clipboard_paste(clip);
            None
        }
        ClipboardCommand::SelectAll => {
            target.clipboard_select_all();
            None
        }
    }
}

/// Builds the OSC 52 clipboard-set escape for `text` (`\x1b]52;c;<base64>\x07`).
#[must_use]
pub fn encode_osc52(text: &str) -> String {
    format!("\x1b]52;c;{}\x07", base64_encode(text.as_bytes()))
}

/// Minimal standard base64 encoder without external dependencies.
pub fn base64_encode(data: &[u8]) -> String {
    const TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(data.len().div_ceil(3) * 4);
    for chunk in data.chunks(3) {
        let b0 = chunk[0];
        let b1 = chunk.get(1).copied().unwrap_or(0);
        let b2 = chunk.get(2).copied().unwrap_or(0);

        out.push(TABLE[(b0 >> 2) as usize] as char);
        out.push(TABLE[(((b0 & 0x03) << 4) | (b1 >> 4)) as usize] as char);
        if chunk.len() > 1 {
            out.push(TABLE[(((b1 & 0x0f) << 2) | (b2 >> 6)) as usize] as char);
        } else {
            out.push('=');
        }
        if chunk.len() > 2 {
            out.push(TABLE[(b2 & 0x3f) as usize] as char);
        } else {
            out.push('=');
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_base64_encode_matches_reference() {
        assert_eq!(base64_encode(b""), "");
        assert_eq!(base64_encode(b"f"), "Zg==");
        assert_eq!(base64_encode(b"fo"), "Zm8=");
        assert_eq!(base64_encode(b"foo"), "Zm9v");
        assert_eq!(base64_encode(b"foobar"), "Zm9vYmFy");
    }

    #[test]
    fn test_encode_osc52_shape() {
        let seq = encode_osc52("hi");
        assert_eq!(seq, "\x1b]52;c;aGk=\x07");
    }

    #[test]
    fn test_copy_populates_register_and_returns_escape() {
        let mut clip = Clipboard::new();
        let esc = clip.copy("hello");
        assert_eq!(clip.register(), "hello");
        assert_eq!(clip.paste(), "hello");
        assert!(esc.starts_with("\x1b]52;c;"));
        assert!(esc.ends_with('\x07'));
    }

    #[test]
    fn test_copy_to_writer() {
        let mut clip = Clipboard::new();
        let mut buf: Vec<u8> = Vec::new();
        clip.copy_to(&mut buf, "abc").unwrap();
        assert_eq!(String::from_utf8(buf).unwrap(), encode_osc52("abc"));
        assert_eq!(clip.register(), "abc");
    }
}
