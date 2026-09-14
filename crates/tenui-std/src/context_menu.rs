use tenui_core::{CanvasSubviewMut, Modifier};

use crate::theme::ThemePalette;

#[derive(Debug, Clone)]
pub enum MenuItem {
    Action { label: String, enabled: bool },
    Separator,
    Submenu { label: String, children: Vec<MenuItem> },
}

impl MenuItem {
    pub fn action(label: impl Into<String>) -> Self {
        Self::Action {
            label: label.into(),
            enabled: true,
        }
    }

    pub fn disabled(label: impl Into<String>) -> Self {
        Self::Action {
            label: label.into(),
            enabled: false,
        }
    }

    pub fn separator() -> Self {
        Self::Separator
    }

    pub fn submenu(label: impl Into<String>, children: Vec<MenuItem>) -> Self {
        Self::Submenu {
            label: label.into(),
            children,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ContextMenu {
    pub items: Vec<MenuItem>,
    pub selected: usize,
    pub visible: bool,
}

impl ContextMenu {
    pub fn new(items: Vec<MenuItem>) -> Self {
        Self {
            items,
            selected: 0,
            visible: false,
        }
    }

    pub fn show(&mut self) {
        self.visible = true;
        self.selected = 0;
        self.skip_to_next_actionable(true);
    }

    pub fn hide(&mut self) {
        self.visible = false;
    }

    pub fn move_up(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let mut idx = self.selected;
        loop {
            idx = idx.checked_sub(1).unwrap_or(self.items.len() - 1);
            if idx == self.selected {
                break;
            }
            if self.is_actionable(idx) {
                self.selected = idx;
                break;
            }
        }
    }

    pub fn move_down(&mut self) {
        if self.items.is_empty() {
            return;
        }
        let mut idx = self.selected;
        loop {
            idx = (idx + 1) % self.items.len();
            if idx == self.selected {
                break;
            }
            if self.is_actionable(idx) {
                self.selected = idx;
                break;
            }
        }
    }

    pub fn selected_label(&self) -> Option<&str> {
        match self.items.get(self.selected)? {
            MenuItem::Action { label, enabled: true } => Some(label),
            MenuItem::Submenu { label, .. } => Some(label),
            _ => None,
        }
    }

    pub fn width(&self) -> u16 {
        let max_label: usize = self
            .items
            .iter()
            .map(|i| match i {
                MenuItem::Action { label, .. } => label.len(),
                MenuItem::Submenu { label, .. } => label.len() + 2,
                MenuItem::Separator => 0,
            })
            .max()
            .unwrap_or(0);
        (max_label + 4).max(8) as u16
    }

    pub fn height(&self) -> u16 {
        self.items.len() as u16
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        if !self.visible {
            return;
        }
        let w = canvas.width() as usize;

        for (y, item) in self.items.iter().enumerate() {
            let is_selected = y == self.selected;

            match item {
                MenuItem::Separator => {
                    let line: String = "─".repeat(w);
                    for (x, ch) in line.chars().enumerate() {
                        if x < w {
                            canvas.set_char(
                                x as u16,
                                y as u16,
                                ch,
                                palette.border,
                                palette.surface,
                                Modifier::empty(),
                            );
                        }
                    }
                }
                MenuItem::Action { label, enabled } => {
                    let (fg, bg) = if is_selected && *enabled {
                        (palette.bg, palette.primary)
                    } else if *enabled {
                        (palette.fg, palette.surface)
                    } else {
                        (palette.fg_muted, palette.surface)
                    };
                    let padded = format!("  {:<width$}", label, width = w.saturating_sub(2));
                    for (x, ch) in padded.chars().enumerate() {
                        if x < w {
                            canvas.set_char(x as u16, y as u16, ch, fg, bg, Modifier::empty());
                        }
                    }
                }
                MenuItem::Submenu { label, .. } => {
                    let (fg, bg) = if is_selected {
                        (palette.bg, palette.primary)
                    } else {
                        (palette.fg, palette.surface)
                    };
                    let arrow_label = format!("{} ▸", label);
                    let padded = format!("  {:<width$}", arrow_label, width = w.saturating_sub(2));
                    for (x, ch) in padded.chars().enumerate() {
                        if x < w {
                            canvas.set_char(x as u16, y as u16, ch, fg, bg, Modifier::empty());
                        }
                    }
                }
            }
        }
    }

    fn is_actionable(&self, idx: usize) -> bool {
        matches!(
            self.items.get(idx),
            Some(MenuItem::Action { enabled: true, .. } | MenuItem::Submenu { .. })
        )
    }

    fn skip_to_next_actionable(&mut self, forward: bool) {
        if self.is_actionable(self.selected) {
            return;
        }
        if forward {
            self.move_down();
        } else {
            self.move_up();
        }
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn context_menu_navigation_skips_separators() {
        let mut menu = ContextMenu::new(vec![
            MenuItem::action("Cut"),
            MenuItem::separator(),
            MenuItem::action("Paste"),
        ]);
        menu.show();
        assert_eq!(menu.selected, 0);
        menu.move_down();
        assert_eq!(menu.selected, 2);
        menu.move_up();
        assert_eq!(menu.selected, 0);
    }

    #[test]
    fn context_menu_renders() {
        let mut menu = ContextMenu::new(vec![
            MenuItem::action("Open"),
            MenuItem::action("Save"),
            MenuItem::disabled("Delete"),
        ]);
        menu.show();

        let palette = ThemePalette::catppuccin_mocha();
        let w = menu.width();
        let h = menu.height();
        let mut buf = Buffer::new(w, h);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, w, h));
        menu.render(&mut canvas, &palette);

        assert_eq!(buf.get(2, 0).unwrap().fg, palette.bg);
    }

    #[test]
    fn context_menu_selected_label() {
        let mut menu = ContextMenu::new(vec![MenuItem::action("Copy"), MenuItem::action("Paste")]);
        menu.show();
        assert_eq!(menu.selected_label(), Some("Copy"));
        menu.move_down();
        assert_eq!(menu.selected_label(), Some("Paste"));
    }
}
