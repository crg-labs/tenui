use std::{io, time::Duration};

/// An input event produced by the Tenui protocol demuxer.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputEvent {
    /// A single keyboard action.
    Key(crossterm::event::KeyEvent),
    /// A mouse event.
    Mouse(crossterm::event::MouseEvent),
    /// SGR 1016 pixel-precision mouse event.
    PixelMouse {
        button: u8,
        pixel_x: u32,
        pixel_y: u32,
        is_release: bool,
    },
    /// A transactional paste payload intercepted via Bracketed Paste (`\x1b[200~` .. `\x1b[201~`).
    Paste(String),
    /// A terminal capability query response (e.g. DA1/DA2 response or cursor position report).
    CapabilityResponse(String),
    /// Terminal window resize event.
    Resize(u16, u16),
}

impl InputEvent {
    /// Returns true if this event represents an initial key press.
    pub fn is_key_press(&self) -> bool {
        matches!(self, InputEvent::Key(k) if k.kind == crossterm::event::KeyEventKind::Press)
    }

    /// Returns true if this event represents a continuous key repeat.
    /// Emitted by modern terminal emulators supporting the Kitty keyboard protocol.
    pub fn is_key_repeat(&self) -> bool {
        matches!(self, InputEvent::Key(k) if k.kind == crossterm::event::KeyEventKind::Repeat)
    }

    /// Returns true if this event represents a key release.
    pub fn is_key_release(&self) -> bool {
        matches!(self, InputEvent::Key(k) if k.kind == crossterm::event::KeyEventKind::Release)
    }

    /// Returns the key code if this event is a key action.
    pub fn key_code(&self) -> Option<crossterm::event::KeyCode> {
        match self {
            InputEvent::Key(k) => Some(k.code),
            _ => None,
        }
    }

    /// Returns key modifiers if this event is a key action.
    pub fn key_modifiers(&self) -> Option<crossterm::event::KeyModifiers> {
        match self {
            InputEvent::Key(k) => Some(k.modifiers),
            _ => None,
        }
    }
}

/// Decouples sub-cell pixel mouse coordinates to fractional character cells.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct PixelDecoupler {
    pub cell_width_px: f32,
    pub cell_height_px: f32,
}

impl Default for PixelDecoupler {
    fn default() -> Self {
        Self {
            cell_width_px: 10.0,
            cell_height_px: 20.0,
        }
    }
}

impl PixelDecoupler {
    pub fn new(cell_width_px: f32, cell_height_px: f32) -> Self {
        Self {
            cell_width_px: cell_width_px.max(1.0),
            cell_height_px: cell_height_px.max(1.0),
        }
    }

    /// Converts raw pixel coordinates to fractional cell coordinates (col_frac, row_frac).
    pub fn to_cell_coords(&self, pixel_x: u32, pixel_y: u32) -> (f32, f32) {
        (
            pixel_x as f32 / self.cell_width_px,
            pixel_y as f32 / self.cell_height_px,
        )
    }

    /// Computes fractional cell travel deltas.
    pub fn delta_cells(&self, dx_px: f32, dy_px: f32) -> (f32, f32) {
        (dx_px / self.cell_width_px, dy_px / self.cell_height_px)
    }
}

enum DemuxState {
    Normal,
    BracketedPaste(String),
    EscSequence(String),
}

/// Advanced Input and Protocol Demuxer.
///
/// Demuxes raw terminal input into:
/// - Transactional `InputEvent::Paste` payloads instead of separate keypress ticks.
/// - Terminal query capability responses (DA1, DA2, DECRQM).
/// - Standard crossterm key, mouse, and resize events.
pub struct InputDemuxer {
    state: DemuxState,
}

impl Default for InputDemuxer {
    fn default() -> Self {
        Self::new()
    }
}

impl InputDemuxer {
    pub fn new() -> Self {
        Self {
            state: DemuxState::Normal,
        }
    }

    /// Polls for an available input event within the specified timeout.
    pub fn poll(&self, timeout: Duration) -> io::Result<bool> {
        crossterm::event::poll(timeout)
    }

    /// Reads the next demuxed input event from the terminal.
    pub fn read(&mut self) -> io::Result<Option<InputEvent>> {
        use crossterm::event::Event;

        if !crossterm::event::poll(Duration::from_millis(10))? {
            return Ok(None);
        }

        match crossterm::event::read()? {
            Event::Key(key) => {
                // If crossterm itself decoded a paste event:
                Ok(Some(InputEvent::Key(key)))
            }
            Event::Mouse(mouse) => Ok(Some(InputEvent::Mouse(mouse))),
            Event::Resize(w, h) => Ok(Some(InputEvent::Resize(w, h))),
            Event::Paste(s) => Ok(Some(InputEvent::Paste(s))),
            Event::FocusGained | Event::FocusLost => Ok(None),
        }
    }

    /// Feeds raw input bytes into the protocol demuxer state machine.
    /// Used for direct byte stream parsing and headless test verification.
    pub fn feed_bytes(&mut self, bytes: &[u8]) -> Vec<InputEvent> {
        let mut events = Vec::new();
        let mut i = 0;

        while i < bytes.len() {
            match &mut self.state {
                DemuxState::Normal => {
                    // Check for bracketed paste start: \x1b[200~
                    if bytes[i..].starts_with(b"\x1b[200~") {
                        self.state = DemuxState::BracketedPaste(String::new());
                        i += 6;
                        continue;
                    }
                    // Check for escape sequence starting capability query
                    if bytes[i] == 0x1b {
                        self.state = DemuxState::EscSequence(String::from("\x1b"));
                        i += 1;
                        continue;
                    }

                    // Otherwise regular character
                    let ch = bytes[i] as char;
                    events.push(InputEvent::Key(crossterm::event::KeyEvent::from(
                        crossterm::event::KeyCode::Char(ch),
                    )));
                    i += 1;
                }
                DemuxState::BracketedPaste(content) => {
                    // Check for bracketed paste end: \x1b[201~
                    if bytes[i..].starts_with(b"\x1b[201~") {
                        let completed_paste = std::mem::take(content);
                        events.push(InputEvent::Paste(completed_paste));
                        self.state = DemuxState::Normal;
                        i += 6;
                    } else {
                        content.push(bytes[i] as char);
                        i += 1;
                    }
                }
                DemuxState::EscSequence(seq) => {
                    seq.push(bytes[i] as char);
                    let last = bytes[i] as char;
                    if last.is_ascii_alphabetic() || last == '~' {
                        let full_seq = std::mem::take(seq);
                        if full_seq.starts_with("\x1b[<") && (last == 'M' || last == 'm') {
                            let inner = &full_seq[3..full_seq.len() - 1];
                            let parts: Vec<&str> = inner.split(';').collect();
                            if parts.len() == 3
                                && let (Ok(btn), Ok(px), Ok(py)) =
                                    (parts[0].parse::<u8>(), parts[1].parse::<u32>(), parts[2].parse::<u32>())
                            {
                                events.push(InputEvent::PixelMouse {
                                    button: btn,
                                    pixel_x: px,
                                    pixel_y: py,
                                    is_release: last == 'm',
                                });
                                self.state = DemuxState::Normal;
                                i += 1;
                                continue;
                            }
                        }

                        events.push(InputEvent::CapabilityResponse(full_seq));
                        self.state = DemuxState::Normal;
                    }
                    i += 1;
                }
            }
        }

        events
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

    fn key_press(code: KeyCode) -> InputEvent {
        InputEvent::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        })
    }

    fn key_repeat(code: KeyCode) -> InputEvent {
        InputEvent::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Repeat,
            state: crossterm::event::KeyEventState::NONE,
        })
    }

    fn key_release(code: KeyCode) -> InputEvent {
        InputEvent::Key(KeyEvent {
            code,
            modifiers: KeyModifiers::NONE,
            kind: KeyEventKind::Release,
            state: crossterm::event::KeyEventState::NONE,
        })
    }

    #[test]
    fn is_key_press() {
        assert!(key_press(KeyCode::Char('a')).is_key_press());
        assert!(!key_repeat(KeyCode::Char('a')).is_key_press());
        assert!(!key_release(KeyCode::Char('a')).is_key_press());
        assert!(!InputEvent::Resize(80, 24).is_key_press());
    }

    #[test]
    fn is_key_repeat() {
        assert!(key_repeat(KeyCode::Char('b')).is_key_repeat());
        assert!(!key_press(KeyCode::Char('b')).is_key_repeat());
    }

    #[test]
    fn is_key_release() {
        assert!(key_release(KeyCode::Char('c')).is_key_release());
        assert!(!key_press(KeyCode::Char('c')).is_key_release());
    }

    #[test]
    fn key_code_extraction() {
        assert_eq!(key_press(KeyCode::Enter).key_code(), Some(KeyCode::Enter));
        assert_eq!(InputEvent::Resize(80, 24).key_code(), None);
        assert_eq!(InputEvent::Paste("hi".into()).key_code(), None);
    }

    #[test]
    fn key_modifiers_extraction() {
        let ev = InputEvent::Key(KeyEvent {
            code: KeyCode::Char('a'),
            modifiers: KeyModifiers::CONTROL | KeyModifiers::SHIFT,
            kind: KeyEventKind::Press,
            state: crossterm::event::KeyEventState::NONE,
        });
        let mods = ev.key_modifiers().unwrap();
        assert!(mods.contains(KeyModifiers::CONTROL));
        assert!(mods.contains(KeyModifiers::SHIFT));
        assert_eq!(InputEvent::Resize(1, 1).key_modifiers(), None);
    }

    // PixelDecoupler tests

    #[test]
    fn pixel_decoupler_default() {
        let d = PixelDecoupler::default();
        assert_eq!(d.cell_width_px, 10.0);
        assert_eq!(d.cell_height_px, 20.0);
    }

    #[test]
    fn pixel_decoupler_clamps_to_one() {
        let d = PixelDecoupler::new(0.0, -5.0);
        assert_eq!(d.cell_width_px, 1.0);
        assert_eq!(d.cell_height_px, 1.0);
    }

    #[test]
    fn pixel_decoupler_to_cell_coords() {
        let d = PixelDecoupler::new(10.0, 20.0);
        let (cx, cy) = d.to_cell_coords(50, 60);
        assert!((cx - 5.0).abs() < f32::EPSILON);
        assert!((cy - 3.0).abs() < f32::EPSILON);
    }

    #[test]
    fn pixel_decoupler_delta_cells() {
        let d = PixelDecoupler::new(10.0, 20.0);
        let (dx, dy) = d.delta_cells(20.0, 40.0);
        assert!((dx - 2.0).abs() < f32::EPSILON);
        assert!((dy - 2.0).abs() < f32::EPSILON);
    }

    // InputDemuxer feed_bytes tests

    #[test]
    fn demuxer_regular_chars() {
        let mut demux = InputDemuxer::new();
        let events = demux.feed_bytes(b"abc");
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].key_code(), Some(KeyCode::Char('a')));
        assert_eq!(events[1].key_code(), Some(KeyCode::Char('b')));
        assert_eq!(events[2].key_code(), Some(KeyCode::Char('c')));
    }

    #[test]
    fn demuxer_bracketed_paste() {
        let mut demux = InputDemuxer::new();
        let mut input = Vec::new();
        input.extend_from_slice(b"\x1b[200~");
        input.extend_from_slice(b"pasted text");
        input.extend_from_slice(b"\x1b[201~");
        let events = demux.feed_bytes(&input);
        assert_eq!(events.len(), 1);
        match &events[0] {
            InputEvent::Paste(s) => assert_eq!(s, "pasted text"),
            other => panic!("expected Paste, got {:?}", other),
        }
    }

    #[test]
    fn demuxer_sgr_pixel_mouse_press() {
        let mut demux = InputDemuxer::new();
        let events = demux.feed_bytes(b"\x1b[<0;100;200M");
        assert_eq!(events.len(), 1);
        match &events[0] {
            InputEvent::PixelMouse {
                button,
                pixel_x,
                pixel_y,
                is_release,
            } => {
                assert_eq!(*button, 0);
                assert_eq!(*pixel_x, 100);
                assert_eq!(*pixel_y, 200);
                assert!(!*is_release);
            }
            other => panic!("expected PixelMouse, got {:?}", other),
        }
    }

    #[test]
    fn demuxer_sgr_pixel_mouse_release() {
        let mut demux = InputDemuxer::new();
        let events = demux.feed_bytes(b"\x1b[<0;50;75m");
        assert_eq!(events.len(), 1);
        match &events[0] {
            InputEvent::PixelMouse { is_release, .. } => assert!(*is_release),
            other => panic!("expected PixelMouse release, got {:?}", other),
        }
    }

    #[test]
    fn demuxer_escape_sequence_capability_response() {
        let mut demux = InputDemuxer::new();
        let events = demux.feed_bytes(b"\x1b[?62c");
        assert_eq!(events.len(), 1);
        match &events[0] {
            InputEvent::CapabilityResponse(s) => assert_eq!(s, "\x1b[?62c"),
            other => panic!("expected CapabilityResponse, got {:?}", other),
        }
    }

    #[test]
    fn demuxer_interleaved_paste_and_keys() {
        let mut demux = InputDemuxer::new();
        let mut input = Vec::new();
        input.extend_from_slice(b"a");
        input.extend_from_slice(b"\x1b[200~X\x1b[201~");
        input.extend_from_slice(b"b");
        let events = demux.feed_bytes(&input);
        assert_eq!(events.len(), 3);
        assert_eq!(events[0].key_code(), Some(KeyCode::Char('a')));
        match &events[1] {
            InputEvent::Paste(s) => assert_eq!(s, "X"),
            other => panic!("expected Paste, got {:?}", other),
        }
        assert_eq!(events[2].key_code(), Some(KeyCode::Char('b')));
    }

    #[test]
    fn demuxer_empty_input() {
        let mut demux = InputDemuxer::new();
        let events = demux.feed_bytes(b"");
        assert!(events.is_empty());
    }
}
