use std::io::{self, Stdout, Write, stdout};

use crate::{
    buffer::{Buffer, CanvasSubviewMut, Rect},
    color::Color,
    guard::TerminalGuard,
    input::InputDemuxer,
    nonblocking::NonBlockingWriter,
    sgr::SgrCoalescer,
};

/// The primary Terminal interface managing double buffers, differential rendering,
/// and terminal hardware synchronization.
pub struct Terminal<W: Write = Stdout> {
    front: Buffer,
    back: Buffer,
    coalescer: SgrCoalescer,
    writer: W,
    guard: Option<TerminalGuard>,
    pub demuxer: InputDemuxer,
}

impl Terminal<Stdout> {
    /// Initializes a terminal with raw mode, alternate screen, and panic guards.
    pub fn new() -> io::Result<Self> {
        let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
        let guard = TerminalGuard::new()?;
        let mut front = Buffer::new(w, h);
        front.poison_all();
        let mut writer = stdout();
        let _ = writer.write_all(b"\x1b[2J\x1b[3J\x1b[H");
        let _ = writer.flush();
        Ok(Self {
            front,
            back: Buffer::new(w, h),
            coalescer: SgrCoalescer::new(),
            writer,
            guard: Some(guard),
            demuxer: InputDemuxer::new(),
        })
    }
}

impl Terminal<NonBlockingWriter> {
    /// Initializes a terminal with a non-blocking background writer thread.
    /// Frames are dropped automatically when stdout is backpressured (e.g. slow SSH),
    /// while internal back-buffer state is preserved so subsequent frames diff cleanly.
    pub fn non_blocking(capacity: usize) -> io::Result<Self> {
        let (w, h) = crossterm::terminal::size().unwrap_or((80, 24));
        let guard = TerminalGuard::new()?;
        let mut front = Buffer::new(w, h);
        front.poison_all();
        let mut writer = NonBlockingWriter::stdout(capacity);
        let _ = writer.write_all(b"\x1b[2J\x1b[3J\x1b[H");
        let _ = writer.flush();
        Ok(Self {
            front,
            back: Buffer::new(w, h),
            coalescer: SgrCoalescer::new(),
            writer,
            guard: Some(guard),
            demuxer: InputDemuxer::new(),
        })
    }

    /// Number of dropped frames due to stdout backpressure.
    pub fn dropped_frames(&self) -> u64 {
        self.writer.dropped_frames()
    }

    /// Number of successfully queued frames.
    pub fn queued_frames(&self) -> u64 {
        self.writer.queued_frames()
    }
}

impl<W: Write> Terminal<W> {
    /// Constructs a headless terminal with custom dimensions and writer.
    /// Essential for automated testing and CI.
    pub fn with_writer(writer: W, width: u16, height: u16) -> Self {
        Self {
            front: Buffer::new(width, height),
            back: Buffer::new(width, height),
            coalescer: SgrCoalescer::new(),
            writer,
            guard: None,
            demuxer: InputDemuxer::new(),
        }
    }

    pub fn size(&self) -> (u16, u16) {
        (self.back.width, self.back.height)
    }

    pub fn rect(&self) -> Rect {
        self.back.rect()
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        if self.back.width == width && self.back.height == height {
            return;
        }

        self.front.resize(width, height);
        self.back.resize(width, height);

        // 1. Purge host terminal emulator screen memory & reset cursor origin
        // \x1b[2J: Clear entire visible screen
        // \x1b[3J: Clear scrollback buffer (prevents dirty pulls on down-scaling)
        // \x1b[H:  Move cursor to row 1, col 1
        let _ = self.writer.write_all(b"\x1b[2J\x1b[3J\x1b[H");
        let _ = self.writer.flush();

        // 2. Invalidate front buffer completely so differential coalescer
        // re-emits every single cell on the next frame
        self.front.poison_all();

        self.coalescer.reset_state();
    }

    /// Access the current front (visible) buffer.
    pub fn front(&self) -> &Buffer {
        &self.front
    }

    /// Access the current back buffer.
    pub fn back(&self) -> &Buffer {
        &self.back
    }

    /// Mutable access to back buffer.
    pub fn back_mut(&mut self) -> &mut Buffer {
        &mut self.back
    }

    /// Access the underlying writer.
    pub fn writer(&self) -> &W {
        &self.writer
    }

    /// Mutable access to the underlying writer.
    pub fn writer_mut(&mut self) -> &mut W {
        &mut self.writer
    }

    /// Checks if an asynchronous SIGWINCH resize signal was received.
    pub fn has_resized(&self) -> bool {
        self.guard.as_ref().is_some_and(|g| g.has_resized())
    }

    /// Checks if an asynchronous SIGWINCH resize signal was received and resets the flag.
    pub fn take_resized(&self) -> bool {
        self.guard.as_ref().is_some_and(|g| g.take_resized())
    }

    /// Executes an immediate draw pass with direct access to the back buffer.
    pub fn draw_buffer<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        self.draw_buffer_with_bg(Color::Reset, f)
    }

    /// Executes an immediate draw pass with direct access to the back buffer,
    /// clearing the back buffer to the specified background color.
    pub fn draw_buffer_with_bg<F>(&mut self, bg: Color, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        if self.guard.is_some()
            && let Ok((w, h)) = crossterm::terminal::size()
            && (w, h) != (self.back.width, self.back.height)
        {
            self.resize(w, h);
        }

        self.back.clear_with_bg(bg);
        f(&mut self.back);

        let mut diff_bytes = Vec::with_capacity(2048);
        self.coalescer.write_diff(&mut diff_bytes, &self.front, &self.back)?;

        self.writer.write_all(&diff_bytes)?;
        self.writer.flush()?;

        // Pointer exchange on successful frame delivery
        std::mem::swap(&mut self.front, &mut self.back);
        Ok(())
    }

    /// Executes an immediate draw pass, providing a clipped subview of the entire buffer.
    pub fn draw<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        self.draw_buffer(|back| {
            let full_rect = back.rect();
            let mut subview = back.subview_mut(full_rect);
            f(&mut subview);
        })
    }

    /// Forces a complete repaint of all cells on the next frame.
    pub fn force_repaint(&mut self) {
        self.front.poison_all();
        self.coalescer.reset_state();
    }

    /// Emits a raw escape/control sequence through the terminal's own writer.
    ///
    /// This is the single output path for out-of-band sequences (clipboard, IME cursor
    /// sync, tactile audio, hyperlinks) so they travel the same writer as the diff stream
    /// — including the non-blocking/backpressure path when one is configured — instead of
    /// racing it via a separate `stdout()` handle.
    pub fn emit(&mut self, sequence: &str) -> io::Result<()> {
        self.writer.write_all(sequence.as_bytes())?;
        self.writer.flush()
    }

    /// Copies `text` to the system clipboard via OSC 52, routed through [`Self::emit`].
    pub fn copy(&mut self, text: &str) -> io::Result<()> {
        self.emit(&crate::clipboard::encode_osc52(text))
    }

    /// Retained-mode present: the back buffer is seeded from the currently displayed
    /// frame, `f` overwrites only the regions that changed, and the differential
    /// coalescer emits just those cells. Unlike [`Self::draw_buffer`] it does
    /// **not** clear — unpainted regions keep their on-screen content — which is what the
    /// retained render loop (`tenui-runtime`) needs for partial repaints.
    ///
    /// For a full repaint (first frame, resize, theme change) clear inside `f` first.
    pub fn present<F>(&mut self, f: F) -> io::Result<()>
    where
        F: FnOnce(&mut Buffer),
    {
        if self.guard.is_some()
            && let Ok((w, h)) = crossterm::terminal::size()
            && (w, h) != (self.back.width, self.back.height)
        {
            self.resize(w, h);
        }

        // Seed the back buffer with what is currently on screen so untouched regions
        // diff as unchanged and only `f`'s edits are emitted.
        self.back.clone_from(&self.front);
        f(&mut self.back);

        let mut diff_bytes = Vec::with_capacity(2048);
        self.coalescer.write_diff(&mut diff_bytes, &self.front, &self.back)?;
        self.writer.write_all(&diff_bytes)?;
        self.writer.flush()?;

        std::mem::swap(&mut self.front, &mut self.back);
        Ok(())
    }

    /// Like [`Self::present`], but the changed-cell set is computed by the injected
    /// `diff` closure (front, back) → dirty indices, and only those cells are emitted.
    ///
    /// This is the seam that lets an accelerated differ (e.g. `tenui-simd`'s
    /// `diff_buffers`) drive the compositor without `tenui-core` depending on it — the
    /// caller (the runtime) supplies the closure. With the exact dirty set the output is
    /// byte-identical to [`Self::present`].
    pub fn present_indexed<D, F>(&mut self, diff: D, f: F) -> io::Result<()>
    where
        D: FnOnce(&Buffer, &Buffer) -> Vec<usize>,
        F: FnOnce(&mut Buffer),
    {
        if self.guard.is_some()
            && let Ok((w, h)) = crossterm::terminal::size()
            && (w, h) != (self.back.width, self.back.height)
        {
            self.resize(w, h);
        }

        self.back.clone_from(&self.front);
        f(&mut self.back);

        let dirty = diff(&self.front, &self.back);
        let mut diff_bytes = Vec::with_capacity(2048);
        self.coalescer
            .write_diff_indexed(&mut diff_bytes, &self.front, &self.back, &dirty)?;
        self.writer.write_all(&diff_bytes)?;
        self.writer.flush()?;

        std::mem::swap(&mut self.front, &mut self.back);
        Ok(())
    }
}
