//! Non-Alternate Screen Viewport (Inline Mode).
//!
//! Renders flexbox trees within an inline region in standard shell scrollback history,
//! preserving terminal command history without entering the Alternate Screen buffer.

use std::io::{self, Write};

/// Controller for inline viewport rendering in standard shell scrollback.
pub struct InlineViewport {
    height: u16,
    is_active: bool,
}

impl InlineViewport {
    pub fn new(height: u16) -> Self {
        Self {
            height: height.max(1),
            is_active: false,
        }
    }

    pub fn height(&self) -> u16 {
        self.height
    }

    /// Prepares the inline viewport by reserving `height` blank lines at the bottom of the shell.
    pub fn begin<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        let mut prep = String::new();
        // Hide cursor during initial allocation
        prep.push_str("\x1b[?25l");
        // Print `height` newlines to ensure room in scrollback
        for _ in 0..self.height {
            prep.push('\n');
        }
        // Move cursor back up `height` rows (CSI {height} A)
        prep.push_str(&format!("\x1b[{}A", self.height));
        writer.write_all(prep.as_bytes())?;
        writer.flush()?;
        self.is_active = true;
        Ok(())
    }

    /// Renders a frame slice within the inline viewport.
    pub fn render_frame<W: Write>(&self, writer: &mut W, diff_bytes: &[u8]) -> io::Result<()> {
        writer.write_all(diff_bytes)?;
        writer.flush()
    }

    /// Finishes inline mode, positioning the cursor below the viewport so shell resumes cleanly.
    pub fn finish<W: Write>(&mut self, writer: &mut W) -> io::Result<()> {
        if !self.is_active {
            return Ok(());
        }
        let mut exit_seq = String::new();
        // Move down past the viewport
        exit_seq.push_str(&format!("\x1b[{}B\r\n", self.height));
        // Restore cursor
        exit_seq.push_str("\x1b[?25h");
        writer.write_all(exit_seq.as_bytes())?;
        writer.flush()?;
        self.is_active = false;
        Ok(())
    }
}
