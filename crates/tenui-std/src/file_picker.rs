use std::path::PathBuf;

use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// File metadata entry with mime-aware glyph mapping.
#[derive(Clone, Debug, PartialEq)]
pub struct FileEntry {
    pub name: String,
    pub is_dir: bool,
    pub size_bytes: u64,
}

impl FileEntry {
    pub fn new(name: impl Into<String>, is_dir: bool, size_bytes: u64) -> Self {
        Self {
            name: name.into(),
            is_dir,
            size_bytes,
        }
    }

    /// Resolves Nerd Font icon and color with ASCII fallback cascade.
    pub fn resolve_icon(&self, use_nerd_fonts: bool) -> (&'static str, Color) {
        crate::icons::Icon::for_path(&self.name, self.is_dir).resolve(use_nerd_fonts)
    }
}

/// Mime-aware directory navigator with interactive breadcrumb headers.
pub struct FilePicker {
    pub current_dir: PathBuf,
    pub entries: Vec<FileEntry>,
    pub selected_idx: usize,
    pub use_nerd_fonts: bool,
}

impl FilePicker {
    pub fn new(initial_path: PathBuf) -> Self {
        Self {
            current_dir: initial_path,
            entries: Vec::new(),
            selected_idx: 0,
            use_nerd_fonts: true,
        }
    }

    pub fn set_entries(&mut self, entries: Vec<FileEntry>) {
        self.entries = entries;
        self.selected_idx = self.selected_idx.min(self.entries.len().saturating_sub(1));
    }

    pub fn select_next(&mut self) {
        if !self.entries.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.entries.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.entries.is_empty() {
            self.selected_idx = match self.selected_idx {
                0 => self.entries.len().saturating_sub(1),
                idx => idx - 1,
            };
        }
    }

    pub fn selected_entry(&self) -> Option<&FileEntry> {
        self.entries.get(self.selected_idx)
    }

    /// Renders the breadcrumb path header and file tree slice into the surface.
    pub fn render(&self, surface: &mut CanvasSubviewMut<'_>, fg: Color, bg: Color) {
        let w = surface.width();
        let h = surface.height();
        if w < 10 || h < 2 {
            return;
        }

        // 1. Breadcrumb Path Header (Row 0)
        let path_str = self.current_dir.to_string_lossy();
        let breadcrumbs = format!(" ~ / {}", path_str.trim_start_matches('/'));
        surface.write_str_clipped(0, 0, &breadcrumbs, Color::CYAN, Color::Rgb(15, 23, 42));

        for x in (breadcrumbs.len() as u16)..w {
            surface.set_char(x, 0, '─', Color::DarkGray, bg, Modifier::empty());
        }

        // 2. File Entries
        let max_rows = h.saturating_sub(1) as usize;
        for (i, entry) in self.entries.iter().take(max_rows).enumerate() {
            let y = (i + 1) as u16;
            let is_selected = i == self.selected_idx;

            let row_bg = if is_selected {
                Color::Rgb(30, 58, 138) // Highlight selection
            } else {
                bg
            };

            let (icon, icon_color) = entry.resolve_icon(self.use_nerd_fonts);

            // Clear row
            for x in 0..w {
                surface.set_char(x, y, ' ', fg, row_bg, Modifier::empty());
            }

            // Selection marker
            if is_selected {
                surface.write_str_clipped(1, y, "▶", Color::CYAN, row_bg);
            }

            // Icon
            surface.write_str_clipped(3, y, icon, icon_color, row_bg);

            // Filename
            let icon_offset = if self.use_nerd_fonts { 6 } else { 10 };
            surface.write_str_clipped(
                icon_offset,
                y,
                &entry.name,
                if is_selected { Color::WHITE } else { fg },
                row_bg,
            );

            // File size
            if !entry.is_dir {
                let size_str = format!("{} B", entry.size_bytes);
                let size_x = w.saturating_sub(size_str.len() as u16 + 2);
                surface.write_str_clipped(size_x, y, &size_str, Color::DarkGray, row_bg);
            }
        }
    }
}
