use std::{io::Cursor, path::Path};

use tenui_core::{Buffer, Cell, Color, Terminal};
use tenui_layout::{TerminalLayoutExt, Ui};

/// Virtual terminal capturing dimensions, emitted byte streams, and cursor states.
#[derive(Debug, Clone)]
pub struct VirtualTerminal {
    pub width: u16,
    pub height: u16,
    pub writer: Cursor<Vec<u8>>,
}

impl VirtualTerminal {
    pub fn new(width: u16, height: u16) -> Self {
        Self {
            width,
            height,
            writer: Cursor::new(Vec::new()),
        }
    }

    pub fn bytes(&self) -> &[u8] {
        self.writer.get_ref().as_slice()
    }

    pub fn clear_bytes(&mut self) {
        self.writer.get_mut().clear();
        self.writer.set_position(0);
    }
}

/// Headless test harness for simulating and asserting terminal screens in CI.
pub struct TenuiTestHarness {
    terminal: Terminal<Cursor<Vec<u8>>>,
    reflow_count: usize,
    paint_count: usize,
}

impl TenuiTestHarness {
    pub fn new(width: u16, height: u16) -> Self {
        let cursor = Cursor::new(Vec::new());
        Self {
            terminal: Terminal::with_writer(cursor, width, height),
            reflow_count: 0,
            paint_count: 0,
        }
    }

    pub fn headless(width: u16, height: u16) -> Self {
        Self::new(width, height)
    }

    pub fn terminal(&self) -> &Terminal<Cursor<Vec<u8>>> {
        &self.terminal
    }

    pub fn terminal_mut(&mut self) -> &mut Terminal<Cursor<Vec<u8>>> {
        &mut self.terminal
    }

    pub fn resize(&mut self, width: u16, height: u16) {
        self.terminal.resize(width, height);
    }

    pub fn draw_layout<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Ui<'_>),
    {
        self.terminal.draw_layout(f).expect("harness draw_layout succeeds");
        self.paint_count += 1;
    }

    pub fn draw_layout_with_bg<F>(&mut self, bg: Color, f: F)
    where
        F: FnOnce(&mut Ui<'_>),
    {
        self.terminal
            .draw_layout_with_bg(bg, f)
            .expect("harness draw_layout_with_bg succeeds");
        self.paint_count += 1;
    }

    pub fn draw_buffer_with_bg<F>(&mut self, bg: Color, f: F)
    where
        F: FnOnce(&mut Buffer),
    {
        self.terminal
            .draw_buffer_with_bg(bg, f)
            .expect("harness draw_buffer_with_bg succeeds");
        self.paint_count += 1;
    }

    pub fn draw_buffer<F>(&mut self, f: F)
    where
        F: FnOnce(&mut Buffer),
    {
        self.terminal.draw_buffer(f).expect("harness draw_buffer succeeds");
        self.paint_count += 1;
    }

    pub fn cell_at(&self, x: u16, y: u16) -> Option<Cell> {
        self.terminal.front().get(x, y).copied()
    }

    pub fn get_cell(&self, x: u16, y: u16) -> Option<Cell> {
        self.cell_at(x, y)
    }

    pub fn symbol_at(&self, x: u16, y: u16) -> String {
        self.cell_at(x, y)
            .map(|c| c.symbol.as_str().to_string())
            .unwrap_or_else(|| " ".to_string())
    }

    pub fn get_text_at(&self, start_x: u16, y: u16, len: u16) -> String {
        let mut actual = String::new();
        for x in start_x..(start_x + len) {
            actual.push_str(&self.symbol_at(x, y));
        }
        actual
    }

    pub fn get_line(&self, y: u16) -> String {
        let buf = self.terminal.front();
        if y >= buf.height {
            return String::new();
        }
        let mut out = String::new();
        for x in 0..buf.width {
            if let Some(cell) = buf.get(x, y)
                && !cell.is_continuation
            {
                out.push_str(cell.symbol.as_str());
            }
        }
        out
    }

    pub fn assert_symbol(&self, x: u16, y: u16, expected: &str) {
        let actual = self.symbol_at(x, y);
        assert_eq!(
            actual, expected,
            "Expected symbol '{}' at ({}, {}), found '{}'",
            expected, x, y, actual
        );
    }

    pub fn assert_text(&self, start_x: u16, y: u16, expected: &str) {
        let actual = self.get_text_at(start_x, y, expected.chars().count() as u16);
        assert_eq!(
            actual, expected,
            "Expected text '{}' at ({}, {}), found '{}'",
            expected, start_x, y, actual
        );
    }

    /// Generates a multiline string snapshot of the current front screen.
    pub fn dump_screen(&self) -> String {
        let buf = self.terminal.front();
        let mut out = String::new();
        for y in 0..buf.height {
            for x in 0..buf.width {
                if let Some(cell) = buf.get(x, y)
                    && !cell.is_continuation
                {
                    out.push_str(cell.symbol.as_str());
                }
            }
            out.push('\n');
        }
        out
    }

    pub fn emitted_bytes(&self) -> &[u8] {
        self.terminal.writer().get_ref().as_slice()
    }

    pub fn total_bytes_emitted(&self) -> usize {
        self.terminal.writer().get_ref().len()
    }

    pub fn clear_emitted_bytes(&mut self) {
        self.terminal.writer_mut().get_mut().clear();
        self.terminal.writer_mut().set_position(0);
    }

    pub fn reflow_count(&self) -> usize {
        self.reflow_count
    }

    pub fn paint_count(&self) -> usize {
        self.paint_count
    }

    pub fn inc_reflow(&mut self) {
        self.reflow_count += 1;
    }

    pub fn inc_paint(&mut self) {
        self.paint_count += 1;
    }

    pub fn reset_counters(&mut self) {
        self.reflow_count = 0;
        self.paint_count = 0;
    }

    /// Asserts that the dumped screen matches the golden snapshot file.
    /// If UPDATE_SNAPSHOTS=1 or the snapshot file doesn't exist, writes the snapshot.
    pub fn assert_golden_snapshot(&self, test_name: &str) {
        let current = self.dump_screen();
        let snapshot_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests").join("snapshots");
        let _ = std::fs::create_dir_all(&snapshot_dir);
        let snapshot_path = snapshot_dir.join(format!("{}.snap", test_name));

        if std::env::var("UPDATE_SNAPSHOTS").as_deref() == Ok("1") || !snapshot_path.exists() {
            std::fs::write(&snapshot_path, &current).expect("write golden snapshot");
        } else {
            let expected = std::fs::read_to_string(&snapshot_path).expect("read golden snapshot");
            assert_eq!(
                current, expected,
                "Golden snapshot mismatch for '{}'. Run with UPDATE_SNAPSHOTS=1 to accept changes.",
                test_name
            );
        }
    }
}
