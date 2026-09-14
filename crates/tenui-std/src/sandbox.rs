//! Out-of-Process Panes & Fault Recovery.

use std::process::{Child, Command, Stdio};

use tenui_core::{Buffer, CanvasSubviewMut, Color, Modifier};

/// Operating state of an isolated child subprocess pane.
#[derive(Debug, PartialEq, Eq)]
pub enum SubprocessState {
    Running,
    Crashed(String),
    Reloading,
    Terminated,
}

/// Supervisory host container managing an isolated out-of-process component.
///
/// If an isolated child process panics, segfaults, or is terminated by the host OS kernel:
/// - The supervisory host catches the death notice or broken pipe.
/// - The primary Taffy layout remains stable.
/// - The compositor displays a localized crash glyph banner (`[!] Pane Process Terminated - Press 'r' to Reload`).
pub struct SubprocessView {
    pub command: String,
    pub args: Vec<String>,
    pub state: SubprocessState,
    pub local_buffer: Buffer,
    child: Option<Child>,
}

impl SubprocessView {
    pub fn new(command: &str, args: &[&str], width: u16, height: u16) -> Self {
        let mut view = Self {
            command: command.to_string(),
            args: args.iter().map(|s| s.to_string()).collect(),
            state: SubprocessState::Terminated,
            local_buffer: Buffer::new(width, height),
            child: None,
        };
        let _ = view.spawn();
        view
    }

    /// Spawns the isolated child process.
    pub fn spawn(&mut self) -> std::io::Result<()> {
        let child = Command::new(&self.command)
            .args(&self.args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn();

        match child {
            Ok(c) => {
                self.child = Some(c);
                self.state = SubprocessState::Running;
                Ok(())
            }
            Err(e) => {
                self.state = SubprocessState::Crashed(format!("Spawn failed: {}", e));
                Err(e)
            }
        }
    }

    /// Checks process liveness and catches crash transitions.
    pub fn poll_liveness(&mut self) {
        if let Some(child) = self.child.as_mut() {
            match child.try_wait() {
                Ok(Some(status)) => {
                    self.state = SubprocessState::Crashed(format!("Exited with status: {}", status));
                }
                Ok(None) => {
                    self.state = SubprocessState::Running;
                }
                Err(e) => {
                    self.state = SubprocessState::Crashed(format!("IPC broken: {}", e));
                }
            }
        }
    }

    /// Explicitly simulates a crash or terminates child for fault tolerance testing.
    pub fn trigger_crash(&mut self, reason: &str) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
        }
        self.state = SubprocessState::Crashed(reason.to_string());
    }

    /// Restarts the child process after a crash.
    pub fn reload(&mut self) -> std::io::Result<()> {
        self.state = SubprocessState::Reloading;
        self.spawn()
    }

    /// Renders the pane or the localized crash boundary into the surface slice.
    pub fn render(&mut self, surface: &mut CanvasSubviewMut<'_>) {
        self.poll_liveness();

        match &self.state {
            SubprocessState::Running => {
                let w = surface.width().min(self.local_buffer.width);
                let h = surface.height().min(self.local_buffer.height);
                for y in 0..h {
                    for x in 0..w {
                        if let Some(cell) = self.local_buffer.get(x, y) {
                            surface.set_cell(x, y, *cell);
                        }
                    }
                }
            }
            SubprocessState::Crashed(reason) => {
                let w = surface.width();
                let h = surface.height();
                for y in 0..h {
                    for x in 0..w {
                        surface.set_char(x, y, ' ', Color::Reset, Color::Reset, Modifier::empty());
                    }
                }

                let banner = "[!] Pane Process Terminated - Press 'r' to Reload";
                let banner_x = (w.saturating_sub(banner.len() as u16)) / 2;
                let banner_y = h / 2;
                surface.set_string(banner_x, banner_y, banner, Color::White, Color::Red, Modifier::BOLD);

                let reason_str = format!("Error: {}", reason);
                let reason_x = (w.saturating_sub(reason_str.len() as u16)) / 2;
                if banner_y + 1 < h {
                    surface.set_string(
                        reason_x,
                        banner_y + 1,
                        &reason_str,
                        Color::Red,
                        Color::Reset,
                        Modifier::DIM,
                    );
                }
            }
            SubprocessState::Reloading => {
                surface.set_string(0, 0, "Reloading pane...", Color::Yellow, Color::Reset, Modifier::ITALIC);
            }
            SubprocessState::Terminated => {
                surface.set_string(0, 0, "Pane stopped", Color::DarkGray, Color::Reset, Modifier::empty());
            }
        }
    }
}
