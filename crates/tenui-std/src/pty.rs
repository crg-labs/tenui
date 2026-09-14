use std::{
    io::{Read, Write},
    process::{Child, Command, Stdio},
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

use tenui_core::{CanvasSubviewMut, Cell, Color, Modifier};

/// VT100 / ANSI escape sequence parser state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ParserState {
    Normal,
    Escape,
    Csi,
}

/// Embedded Virtual PTY container hosting terminal sessions and VT100 emulation.
pub struct TerminalPane {
    pub width: u16,
    pub height: u16,
    pub cells: Vec<Cell>,
    pub cursor_x: u16,
    pub cursor_y: u16,
    pub cursor_visible: bool,
    pub current_fg: Color,
    pub current_bg: Color,
    pub current_modifier: Modifier,
    state: ParserState,
    csi_params: Vec<u16>,
    csi_current_num: Option<u16>,
    stdin_tx: Option<Sender<Vec<u8>>>,
    stdout_rx: Option<Receiver<Vec<u8>>>,
    child: Option<Child>,
}

impl TerminalPane {
    pub fn new(width: u16, height: u16) -> Self {
        let size = (width as usize) * (height as usize);
        let cells = vec![Cell::default(); size];
        Self {
            width,
            height,
            cells,
            cursor_x: 0,
            cursor_y: 0,
            cursor_visible: true,
            current_fg: Color::Reset,
            current_bg: Color::Reset,
            current_modifier: Modifier::empty(),
            state: ParserState::Normal,
            csi_params: Vec::new(),
            csi_current_num: None,
            stdin_tx: None,
            stdout_rx: None,
            child: None,
        }
    }

    /// Spawns a child process and attaches stdin/stdout channels.
    pub fn spawn_command(&mut self, cmd: &str, args: &[&str]) -> std::io::Result<()> {
        let mut child = Command::new(cmd)
            .args(args)
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()?;

        let mut child_stdout = child.stdout.take().expect("Failed to capture child stdout");
        let (out_tx, out_rx) = channel::<Vec<u8>>();
        thread::spawn(move || {
            let mut buf = [0u8; 1024];
            while let Ok(n) = child_stdout.read(&mut buf) {
                if n == 0 {
                    break;
                }
                if out_tx.send(buf[..n].to_vec()).is_err() {
                    break;
                }
            }
        });

        let mut child_stdin = child.stdin.take().expect("Failed to capture child stdin");
        let (in_tx, in_rx) = channel::<Vec<u8>>();
        thread::spawn(move || {
            while let Ok(data) = in_rx.recv() {
                if child_stdin.write_all(&data).is_err() {
                    break;
                }
                let _ = child_stdin.flush();
            }
        });

        self.child = Some(child);
        self.stdout_rx = Some(out_rx);
        self.stdin_tx = Some(in_tx);
        Ok(())
    }

    /// Reads pending output bytes from child process if any.
    pub fn poll_child(&mut self) {
        let mut pending = Vec::new();
        if let Some(rx) = self.stdout_rx.as_ref() {
            while let Ok(bytes) = rx.try_recv() {
                pending.push(bytes);
            }
        }
        for bytes in pending {
            self.process_bytes(&bytes);
        }
    }

    /// Sends input bytes (e.g. keypresses) to the running child process.
    pub fn send_input(&mut self, input: &[u8]) {
        if let Some(tx) = self.stdin_tx.as_ref() {
            let _ = tx.send(input.to_vec());
        }
    }

    /// Consumes and parses a stream of raw VT100 / ANSI escape sequence bytes.
    pub fn process_bytes(&mut self, bytes: &[u8]) {
        for &b in bytes {
            match self.state {
                ParserState::Normal => match b {
                    0x1b => {
                        self.state = ParserState::Escape;
                    }
                    b'\r' => {
                        self.cursor_x = 0;
                    }
                    b'\n' => {
                        self.cursor_y += 1;
                        if self.cursor_y >= self.height {
                            self.scroll_up();
                            self.cursor_y = self.height.saturating_sub(1);
                        }
                    }
                    b'\t' => {
                        self.cursor_x = (self.cursor_x + 8) & !7;
                        if self.cursor_x >= self.width {
                            self.cursor_x = self.width.saturating_sub(1);
                        }
                    }
                    0x08 => {
                        self.cursor_x = self.cursor_x.saturating_sub(1);
                    }
                    32..=126 | 160..=255 => {
                        let ch = b as char;
                        self.write_char(ch);
                    }
                    _ => {}
                },
                ParserState::Escape => {
                    match b {
                        b'[' => {
                            self.state = ParserState::Csi;
                            self.csi_params.clear();
                            self.csi_current_num = None;
                        }
                        b'c' => {
                            // Reset initial terminal state
                            self.clear_screen();
                            self.state = ParserState::Normal;
                        }
                        _ => {
                            self.state = ParserState::Normal;
                        }
                    }
                }
                ParserState::Csi => {
                    match b {
                        b'0'..=b'9' => {
                            let digit = (b - b'0') as u16;
                            self.csi_current_num = Some(self.csi_current_num.unwrap_or(0) * 10 + digit);
                        }
                        b';' => {
                            let num = self.csi_current_num.unwrap_or(0);
                            self.csi_params.push(num);
                            self.csi_current_num = None;
                        }
                        b'A' => {
                            // CUU: Cursor Up
                            let count = self.csi_current_num.unwrap_or(1).max(1);
                            self.cursor_y = self.cursor_y.saturating_sub(count);
                            self.state = ParserState::Normal;
                        }
                        b'B' => {
                            // CUD: Cursor Down
                            let count = self.csi_current_num.unwrap_or(1).max(1);
                            self.cursor_y = (self.cursor_y + count).min(self.height.saturating_sub(1));
                            self.state = ParserState::Normal;
                        }
                        b'C' => {
                            // CUF: Cursor Forward
                            let count = self.csi_current_num.unwrap_or(1).max(1);
                            self.cursor_x = (self.cursor_x + count).min(self.width.saturating_sub(1));
                            self.state = ParserState::Normal;
                        }
                        b'D' => {
                            // CUB: Cursor Back
                            let count = self.csi_current_num.unwrap_or(1).max(1);
                            self.cursor_x = self.cursor_x.saturating_sub(count);
                            self.state = ParserState::Normal;
                        }
                        b'H' | b'f' => {
                            // CUP: Cursor Position (1-based row;col)
                            if let Some(n) = self.csi_current_num {
                                self.csi_params.push(n);
                            }
                            let row = self.csi_params.first().copied().unwrap_or(1).max(1);
                            let col = self.csi_params.get(1).copied().unwrap_or(1).max(1);
                            self.cursor_y = (row - 1).min(self.height.saturating_sub(1));
                            self.cursor_x = (col - 1).min(self.width.saturating_sub(1));
                            self.state = ParserState::Normal;
                        }
                        b'J' => {
                            // ED: Erase in Display
                            let mode = self.csi_current_num.unwrap_or(0);
                            self.erase_display(mode);
                            self.state = ParserState::Normal;
                        }
                        b'K' => {
                            // EL: Erase in Line
                            let mode = self.csi_current_num.unwrap_or(0);
                            self.erase_line(mode);
                            self.state = ParserState::Normal;
                        }
                        b'm' => {
                            // SGR: Set Graphics Rendition
                            if let Some(n) = self.csi_current_num {
                                self.csi_params.push(n);
                            }
                            if self.csi_params.is_empty() {
                                self.csi_params.push(0);
                            }
                            self.apply_sgr();
                            self.state = ParserState::Normal;
                        }
                        _ => {
                            // Unknown or extended CSI terminator
                            self.state = ParserState::Normal;
                        }
                    }
                }
            }
        }
    }

    fn write_char(&mut self, ch: char) {
        if self.cursor_x >= self.width {
            self.cursor_x = 0;
            self.cursor_y += 1;
            if self.cursor_y >= self.height {
                self.scroll_up();
                self.cursor_y = self.height.saturating_sub(1);
            }
        }

        let idx = (self.cursor_y as usize) * (self.width as usize) + (self.cursor_x as usize);
        if idx < self.cells.len() {
            let mut cell = Cell::from_char(ch);
            cell.fg = self.current_fg;
            cell.bg = self.current_bg;
            cell.modifier = self.current_modifier;
            self.cells[idx] = cell;
        }
        self.cursor_x += 1;
    }

    pub fn scroll_up(&mut self) {
        let w = self.width as usize;
        let h = self.height as usize;
        if h <= 1 {
            self.clear_screen();
            return;
        }

        // Move rows 1..h up to 0..h-1
        self.cells.copy_within(w..(w * h), 0);

        // Clear bottom row
        let bot_start = (h - 1) * w;
        for c in &mut self.cells[bot_start..] {
            *c = Cell::default();
        }
    }

    pub fn clear_screen(&mut self) {
        for c in &mut self.cells {
            *c = Cell::default();
        }
        self.cursor_x = 0;
        self.cursor_y = 0;
    }

    fn erase_display(&mut self, mode: u16) {
        let cur_idx = (self.cursor_y as usize) * (self.width as usize) + (self.cursor_x as usize);
        let total = self.cells.len();
        match mode {
            0 => {
                // From cursor to end of screen
                for c in &mut self.cells[cur_idx..] {
                    *c = Cell::default();
                }
            }
            1 => {
                // From start of screen to cursor
                for c in &mut self.cells[..cur_idx.min(total)] {
                    *c = Cell::default();
                }
            }
            2 => {
                // Entire screen
                self.clear_screen();
            }
            _ => {}
        }
    }

    fn erase_line(&mut self, mode: u16) {
        let w = self.width as usize;
        let row_start = (self.cursor_y as usize) * w;
        let col = self.cursor_x as usize;
        let total = self.cells.len();

        match mode {
            0 => {
                // Cursor to end of line
                let start = row_start + col;
                let end = (row_start + w).min(total);
                for c in &mut self.cells[start..end] {
                    *c = Cell::default();
                }
            }
            1 => {
                // Start of line to cursor
                let start = row_start;
                let end = (row_start + col + 1).min(row_start + w).min(total);
                for c in &mut self.cells[start..end] {
                    *c = Cell::default();
                }
            }
            2 => {
                // Entire line
                let start = row_start;
                let end = (row_start + w).min(total);
                for c in &mut self.cells[start..end] {
                    *c = Cell::default();
                }
            }
            _ => {}
        }
    }

    fn apply_sgr(&mut self) {
        let mut idx = 0;
        while idx < self.csi_params.len() {
            let code = self.csi_params[idx];
            match code {
                0 => {
                    self.current_fg = Color::Reset;
                    self.current_bg = Color::Reset;
                    self.current_modifier = Modifier::empty();
                }
                1 => self.current_modifier.insert(Modifier::BOLD),
                2 => self.current_modifier.insert(Modifier::DIM),
                3 => self.current_modifier.insert(Modifier::ITALIC),
                4 => self.current_modifier.insert(Modifier::UNDERLINE),
                7 => self.current_modifier.insert(Modifier::REVERSE),
                22 => self.current_modifier.remove(Modifier::BOLD | Modifier::DIM),
                23 => self.current_modifier.remove(Modifier::ITALIC),
                24 => self.current_modifier.remove(Modifier::UNDERLINE),
                27 => self.current_modifier.remove(Modifier::REVERSE),
                30 => self.current_fg = Color::Black,
                31 => self.current_fg = Color::Red,
                32 => self.current_fg = Color::Green,
                33 => self.current_fg = Color::Yellow,
                34 => self.current_fg = Color::Blue,
                35 => self.current_fg = Color::Magenta,
                36 => self.current_fg = Color::Cyan,
                37 => self.current_fg = Color::White,
                38 => {
                    // Extended FG: 38;2;r;g;b or 38;5;idx
                    if idx + 4 < self.csi_params.len() && self.csi_params[idx + 1] == 2 {
                        let r = self.csi_params[idx + 2] as u8;
                        let g = self.csi_params[idx + 3] as u8;
                        let b = self.csi_params[idx + 4] as u8;
                        self.current_fg = Color::Rgb(r, g, b);
                        idx += 4;
                    } else if idx + 2 < self.csi_params.len() && self.csi_params[idx + 1] == 5 {
                        let i = self.csi_params[idx + 2] as u8;
                        self.current_fg = Color::Indexed(i);
                        idx += 2;
                    }
                }
                39 => self.current_fg = Color::Reset,
                40 => self.current_bg = Color::Black,
                41 => self.current_bg = Color::Red,
                42 => self.current_bg = Color::Green,
                43 => self.current_bg = Color::Yellow,
                44 => self.current_bg = Color::Blue,
                45 => self.current_bg = Color::Magenta,
                46 => self.current_bg = Color::Cyan,
                47 => self.current_bg = Color::White,
                48 => {
                    // Extended BG: 48;2;r;g;b or 48;5;idx
                    if idx + 4 < self.csi_params.len() && self.csi_params[idx + 1] == 2 {
                        let r = self.csi_params[idx + 2] as u8;
                        let g = self.csi_params[idx + 3] as u8;
                        let b = self.csi_params[idx + 4] as u8;
                        self.current_bg = Color::Rgb(r, g, b);
                        idx += 4;
                    } else if idx + 2 < self.csi_params.len() && self.csi_params[idx + 1] == 5 {
                        let i = self.csi_params[idx + 2] as u8;
                        self.current_bg = Color::Indexed(i);
                        idx += 2;
                    }
                }
                49 => self.current_bg = Color::Reset,
                90 => self.current_fg = Color::DarkGray,
                91 => self.current_fg = Color::LightRed,
                92 => self.current_fg = Color::LightGreen,
                93 => self.current_fg = Color::LightYellow,
                94 => self.current_fg = Color::LightBlue,
                95 => self.current_fg = Color::LightMagenta,
                96 => self.current_fg = Color::LightCyan,
                97 => self.current_fg = Color::White,
                _ => {}
            }
            idx += 1;
        }
    }

    /// Renders virtual terminal cells directly into a host `CanvasSubviewMut`.
    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width().min(self.width);
        let h = canvas.height().min(self.height);

        for y in 0..h {
            for x in 0..w {
                let idx = (y as usize) * (self.width as usize) + (x as usize);
                if idx < self.cells.len() {
                    let mut cell = self.cells[idx];
                    if self.cursor_visible && x == self.cursor_x && y == self.cursor_y {
                        cell.modifier |= Modifier::REVERSE;
                    }
                    canvas.set_cell(x, y, cell);
                }
            }
        }
    }
}

impl Drop for TerminalPane {
    fn drop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
