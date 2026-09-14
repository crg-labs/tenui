use tenui_core::{CanvasSubviewMut, Modifier};

use crate::theme::ThemePalette;

pub struct Tab {
    pub label: String,
    pub closable: bool,
}

impl Tab {
    pub fn new(label: impl Into<String>) -> Self {
        Self {
            label: label.into(),
            closable: false,
        }
    }

    pub fn closable(mut self) -> Self {
        self.closable = true;
        self
    }
}

pub struct TabBar {
    pub tabs: Vec<Tab>,
    pub active: usize,
    pub scroll_offset: usize,
}

impl TabBar {
    pub fn new(tabs: Vec<Tab>) -> Self {
        Self {
            tabs,
            active: 0,
            scroll_offset: 0,
        }
    }

    pub fn select(&mut self, index: usize) {
        if index < self.tabs.len() {
            self.active = index;
        }
    }

    pub fn select_next(&mut self) {
        if !self.tabs.is_empty() {
            self.active = (self.active + 1) % self.tabs.len();
        }
    }

    pub fn select_prev(&mut self) {
        if !self.tabs.is_empty() {
            self.active = self.active.checked_sub(1).unwrap_or(self.tabs.len() - 1);
        }
    }

    pub fn close(&mut self, index: usize) -> Option<Tab> {
        if index < self.tabs.len() && self.tabs[index].closable {
            let tab = self.tabs.remove(index);
            if self.active >= self.tabs.len() && !self.tabs.is_empty() {
                self.active = self.tabs.len() - 1;
            }
            Some(tab)
        } else {
            None
        }
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        let w = canvas.width() as usize;
        let mut col = 0usize;

        for (i, tab) in self.tabs.iter().enumerate().skip(self.scroll_offset) {
            if col >= w {
                break;
            }

            let is_active = i == self.active;
            let (fg, bg, mods) = if is_active {
                (palette.bg, palette.primary, Modifier::BOLD)
            } else {
                (palette.fg_muted, palette.surface, Modifier::empty())
            };

            let close_suffix = if tab.closable { " ×" } else { "" };
            let text = format!(" {}{} ", tab.label, close_suffix);

            for ch in text.chars() {
                if col < w {
                    canvas.set_char(col as u16, 0, ch, fg, bg, mods);
                    col += 1;
                }
            }

            if col < w {
                canvas.set_char(col as u16, 0, '│', palette.border, palette.surface, Modifier::empty());
                col += 1;
            }
        }

        for x in col..w {
            canvas.set_char(x as u16, 0, ' ', palette.fg, palette.surface, Modifier::empty());
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn tab_bar_renders_active_tab() {
        let tabs = vec![Tab::new("Files"), Tab::new("Search"), Tab::new("Git")];
        let bar = TabBar::new(tabs);
        let palette = ThemePalette::catppuccin_mocha();

        let mut buf = Buffer::new(40, 1);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 1));
        bar.render(&mut canvas, &palette);

        let first_cell = buf.get(1, 0).unwrap();
        assert_eq!(first_cell.fg, palette.bg);
        assert_eq!(first_cell.bg, palette.primary);
    }

    #[test]
    fn tab_bar_navigation() {
        let tabs = vec![Tab::new("A"), Tab::new("B"), Tab::new("C")];
        let mut bar = TabBar::new(tabs);

        assert_eq!(bar.active, 0);
        bar.select_next();
        assert_eq!(bar.active, 1);
        bar.select_next();
        assert_eq!(bar.active, 2);
        bar.select_next();
        assert_eq!(bar.active, 0);
        bar.select_prev();
        assert_eq!(bar.active, 2);
    }

    #[test]
    fn tab_bar_close() {
        let tabs = vec![Tab::new("Keep"), Tab::new("Close").closable(), Tab::new("Also Keep")];
        let mut bar = TabBar::new(tabs);
        bar.active = 1;

        let closed = bar.close(1);
        assert!(closed.is_some());
        assert_eq!(closed.unwrap().label, "Close");
        assert_eq!(bar.tabs.len(), 2);
        assert_eq!(bar.active, 1);
    }
}
