//! Terminal Driver abstraction for native, headless, and WebAssembly targets.

use std::{io, time::Duration};

use crate::input::InputEvent;

/// An abstract terminal driver interface decoupled from the underlying host OS.
pub trait TerminalDriver {
    /// Polls for an available input event within an optional timeout.
    fn poll_event(&mut self, timeout: Option<Duration>) -> io::Result<Option<InputEvent>>;

    /// Writes differential terminal bytes to stdout or the virtual canvas.
    fn write_diff(&mut self, bytes: &[u8]) -> io::Result<()>;

    /// Flushes any buffered bytes.
    fn flush(&mut self) -> io::Result<()>;

    /// Returns the current terminal dimensions as (columns, rows).
    fn terminal_size(&self) -> (u16, u16);
}

/// Headless in-memory virtual terminal driver used for testing and headless snapshot capture.
pub struct VirtualTerminalDriver {
    pub width: u16,
    pub height: u16,
    pub output_sink: Vec<u8>,
    pub input_queue: Vec<InputEvent>,
}

impl VirtualTerminalDriver {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            output_sink: Vec::new(),
            input_queue: Vec::new(),
        }
    }

    pub fn push_event(&mut self, event: InputEvent) {
        self.input_queue.push(event);
    }
}

impl TerminalDriver for VirtualTerminalDriver {
    fn poll_event(&mut self, _timeout: Option<Duration>) -> io::Result<Option<InputEvent>> {
        if self.input_queue.is_empty() {
            Ok(None)
        } else {
            Ok(Some(self.input_queue.remove(0)))
        }
    }

    fn write_diff(&mut self, bytes: &[u8]) -> io::Result<()> {
        self.output_sink.extend_from_slice(bytes);
        Ok(())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    fn terminal_size(&self) -> (u16, u16) {
        (self.width, self.height)
    }
}
