use std::{
    io::{self, Write, stdout},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use tracing::{debug, warn};

static CLEANUP_INSTALLED: AtomicBool = AtomicBool::new(false);

/// RAII Terminal guard ensuring emergency terminal restoration upon normal exit,
/// POSIX termination signals, or application panic.
pub struct TerminalGuard {
    active: bool,
    sigint_flag: Arc<AtomicBool>,
    sigwinch_flag: Arc<AtomicBool>,
}

impl TerminalGuard {
    /// Emergency terminal reset written on restore, signal, and panic.
    ///
    /// Restores the terminal to sane defaults regardless of which modes the
    /// application enabled: SGR attributes off, synchronized-output (`2026`) and
    /// bracketed-paste (`2004`) off, all four mouse-tracking modes off
    /// (`1000`/`1002`/`1003` and SGR-extended `1006`), leave the alternate
    /// screen (`1049`), and show the cursor (`25`). The mouse disables are
    /// essential: without them, a panic while mouse capture is active leaves the
    /// host shell emitting raw mouse-report escapes on every pointer movement.
    pub const RESET_SEQUENCE: &'static str =
        "\x1b[0m\x1b[?2026l\x1b[?2004l\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1049l\x1b[?25h";
    pub const INIT_SEQUENCE: &'static str = "\x1b[?1049h\x1b[2J\x1b[H\x1b[?25l\x1b[?2004h";

    /// Initializes terminal raw mode, alternate screen buffer, and panic hooks.
    pub fn new() -> io::Result<Self> {
        enable_raw_mode()?;

        let mut out = stdout();
        out.write_all(Self::INIT_SEQUENCE.as_bytes())?;
        out.flush()?;

        let sigint_flag = Arc::new(AtomicBool::new(false));
        let sigwinch_flag = Arc::new(AtomicBool::new(false));

        #[cfg(unix)]
        {
            use signal_hook::consts::signal::*;
            let _ = signal_hook::flag::register(SIGINT, Arc::clone(&sigint_flag));
            let _ = signal_hook::flag::register(SIGTERM, Arc::clone(&sigint_flag));
            let _ = signal_hook::flag::register(SIGQUIT, Arc::clone(&sigint_flag));
            let _ = signal_hook::flag::register(SIGWINCH, Arc::clone(&sigwinch_flag));
        }

        if !CLEANUP_INSTALLED.swap(true, Ordering::SeqCst) {
            let default_panic_hook = std::panic::take_hook();
            std::panic::set_hook(Box::new(move |panic_info| {
                // Best-effort cleanup: surface failures but never obstruct the
                // default hook (which prints the panic and unwinds/aborts).
                if let Err(e) = disable_raw_mode() {
                    warn!("panic cleanup: failed to disable raw mode: {e}");
                }
                let mut out = stdout();
                if let Err(e) = out.write_all(Self::RESET_SEQUENCE.as_bytes()) {
                    warn!("panic cleanup: failed to write terminal reset: {e}");
                }
                let _ = out.flush();
                default_panic_hook(panic_info);
            }));
        }

        debug!("terminal guard installed (raw mode + alternate screen)");
        Ok(Self {
            active: true,
            sigint_flag,
            sigwinch_flag,
        })
    }

    /// Checks if a termination signal (SIGINT/SIGTERM/SIGQUIT) was caught.
    pub fn should_terminate(&self) -> bool {
        self.sigint_flag.load(Ordering::Relaxed)
    }

    /// Checks if a window resize signal (SIGWINCH) was caught and resets the flag.
    pub fn take_resized(&self) -> bool {
        self.sigwinch_flag.swap(false, Ordering::Relaxed)
    }

    /// Checks if a window resize signal (SIGWINCH) was caught without resetting the flag.
    pub fn has_resized(&self) -> bool {
        self.sigwinch_flag.load(Ordering::Relaxed)
    }

    /// Writes an OSC 52 base64 clipboard sequence directly to stdout.
    pub fn copy_to_clipboard_osc52(content: &str) -> io::Result<()> {
        let mut out = stdout();
        out.write_all(crate::clipboard::encode_osc52(content).as_bytes())?;
        out.flush()
    }

    /// Explicitly restores the terminal before dropping.
    pub fn restore(&mut self) -> io::Result<()> {
        if self.active {
            self.active = false;
            let mut out = stdout();
            // Emit the reset best-effort: log a write failure but still attempt
            // to leave raw mode, and surface that error to the caller.
            if let Err(e) = out.write_all(Self::RESET_SEQUENCE.as_bytes()) {
                warn!("failed to write terminal reset sequence: {e}");
            }
            if let Err(e) = out.flush() {
                warn!("failed to flush terminal reset sequence: {e}");
            }
            disable_raw_mode()?;
        }
        Ok(())
    }
}

impl Drop for TerminalGuard {
    fn drop(&mut self) {
        // Drop can't propagate, so a failed restore would otherwise vanish.
        if let Err(e) = self.restore() {
            warn!("terminal guard restore failed on drop: {e}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalGuard;

    #[test]
    fn reset_sequence_disables_mouse_tracking_and_restores_terminal() {
        let seq = TerminalGuard::RESET_SEQUENCE;
        // All four mouse-tracking modes must be turned off, or a panic with
        // mouse capture active corrupts the host shell.
        for mode in ["\x1b[?1000l", "\x1b[?1002l", "\x1b[?1003l", "\x1b[?1006l"] {
            assert!(seq.contains(mode), "RESET_SEQUENCE must contain {mode:?}");
        }
        // And it must still restore the rest of the terminal state.
        assert!(seq.contains("\x1b[?2004l"), "bracketed paste off");
        assert!(seq.contains("\x1b[?1049l"), "leave alternate screen");
        assert!(seq.contains("\x1b[?25h"), "show cursor");
    }
}
