use tenui_core::{canvas::CanvasSubviewMut, color::Color, geometry::Rect};

use crate::shadow::FocusHalo;

/// Tactile button styling variants.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ButtonVariant {
    /// Clean flat surface button
    Flat,
    /// Physically depressible 3D keycap with collapsible drop shadow
    Elevated3D,
    /// Pill / Capsule button with anti-aliased bookends (◖, ◗)
    Capsule,
    /// Muted transparent button with accent border
    Ghost,
}

/// A tactile interactive button with physical translation and zero-layout overhead.
#[derive(Clone, Debug)]
pub struct TactileButton {
    pub label: String,
    pub variant: ButtonVariant,
    pub accent: Color,
    pub is_focused: bool,
    pub is_active: bool,
    pub focus_halo: Option<FocusHalo>,
}

impl TactileButton {
    pub fn new(label: impl Into<String>, accent: Color) -> Self {
        Self {
            label: label.into(),
            variant: ButtonVariant::Elevated3D,
            accent,
            is_focused: false,
            is_active: false,
            focus_halo: None,
        }
    }

    pub fn new_keycap(label: impl Into<String>) -> Self {
        Self::new(label, Color::CYAN)
    }

    pub fn new_capsule(label: impl Into<String>) -> Self {
        Self::new(label, Color::CYAN).variant(ButtonVariant::Capsule)
    }

    pub fn render(&self, surface: &mut CanvasSubviewMut) {
        self.paint(surface);
    }

    pub fn variant(mut self, variant: ButtonVariant) -> Self {
        self.variant = variant;
        self
    }

    pub fn focus_halo(mut self, halo: FocusHalo) -> Self {
        self.focus_halo = Some(halo);
        self
    }

    pub fn paint(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;
        if w < 4 || h < 1 {
            return;
        }

        let (fg, bg) = match (self.is_focused, self.is_active) {
            (true, true) => (Color::BLACK, self.accent.brighten(0.2)),
            (true, false) => (Color::WHITE, self.accent),
            (false, true) => (Color::WHITE, self.accent.dim(0.2)),
            (false, false) => match self.variant {
                ButtonVariant::Ghost => (self.accent, Color::TRANSPARENT),
                _ => (Color::WHITE, self.accent.dim(0.1)),
            },
        };

        // Render Focus Halo if focused
        if self.is_focused
            && let Some(halo) = &self.focus_halo
        {
            halo.render_halo(surface, Rect::new(0, 0, w, h), 0.0);
        }

        match self.variant {
            ButtonVariant::Capsule => {
                surface.set_cell_styled(0, 0, '◖', bg, Color::TRANSPARENT);
                for x in 1..(w - 1) {
                    surface.set_cell_styled(x, 0, ' ', fg, bg);
                }
                surface.set_cell_styled(w - 1, 0, '◗', bg, Color::TRANSPARENT);

                let text_offset = (w.saturating_sub(self.label.chars().count() as u16)) / 2;
                surface.write_str_clipped(text_offset, 0, &self.label, fg, bg);
            }
            ButtonVariant::Elevated3D => {
                let btn_w = w.saturating_sub(1);
                let btn_h = h.saturating_sub(1);
                let offset = if self.is_active { 1 } else { 0 };

                // Draw Button Face (Depressed by +1 offset when active)
                for y in 0..btn_h {
                    for x in 0..btn_w {
                        surface.set_cell_styled(x + offset, y + offset, ' ', fg, bg);
                    }
                }
                let text_len = self.label.chars().count() as u16;
                let label_x = (btn_w.saturating_sub(text_len)) / 2 + offset;
                let label_y = (btn_h / 2) + offset;
                surface.write_str_clipped(label_x, label_y, &self.label, fg, bg);

                // Draw Collapsible Drop Shadow
                if !self.is_active {
                    let shadow_color = Color::BLACK.with_alpha(0.5);
                    for x in 1..=btn_w {
                        surface.set_cell_styled(x, btn_h, '▄', shadow_color, Color::TRANSPARENT);
                    }
                    for y in 1..=btn_h {
                        surface.set_cell_styled(btn_w, y, '▌', shadow_color, Color::TRANSPARENT);
                    }
                }
            }
            ButtonVariant::Flat | ButtonVariant::Ghost => {
                for y in 0..h {
                    for x in 0..w {
                        surface.set_cell_styled(x, y, ' ', fg, bg);
                    }
                }
                let text_len = self.label.chars().count() as u16;
                let label_x = (w.saturating_sub(text_len)) / 2;
                let label_y = h / 2;
                surface.write_str_clipped(label_x, label_y, &self.label, fg, bg);
            }
        }
    }
}

/// Split action button with integrated dropdown trigger.
#[derive(Clone, Debug)]
pub struct SplitActionButton {
    pub primary_label: String,
    pub accent: Color,
    pub zone_a_focused: bool,
    pub zone_b_focused: bool,
    pub is_active: bool,
}

impl SplitActionButton {
    pub fn new(primary_label: impl Into<String>, accent: Color) -> Self {
        Self {
            primary_label: primary_label.into(),
            accent,
            zone_a_focused: false,
            zone_b_focused: false,
            is_active: false,
        }
    }

    pub fn paint(&self, surface: &mut CanvasSubviewMut) {
        let w = surface.bounds.width;
        let h = surface.bounds.height;
        if w < 8 || h < 1 {
            return;
        }

        let split_x = w.saturating_sub(4);

        // Zone A (Primary) colors
        let (fg_a, bg_a) = match (self.zone_a_focused, self.is_active) {
            (true, true) => (Color::BLACK, self.accent.brighten(0.2)),
            (true, false) => (Color::WHITE, self.accent),
            (false, true) => (Color::WHITE, self.accent.dim(0.2)),
            (false, false) => (Color::WHITE, self.accent.dim(0.1)),
        };

        // Zone B (Dropdown) colors
        let (fg_b, bg_b) = match (self.zone_b_focused, self.is_active) {
            (true, true) => (Color::BLACK, self.accent.brighten(0.3)),
            (true, false) => (Color::BLACK, self.accent.brighten(0.15)),
            (false, _) => (Color::WHITE, self.accent.dim(0.15)),
        };

        // Paint Zone A
        for y in 0..h {
            for x in 0..split_x {
                surface.set_cell_styled(x, y, ' ', fg_a, bg_a);
            }
        }
        let text_len = self.primary_label.chars().count() as u16;
        let label_x = (split_x.saturating_sub(text_len)) / 2;
        let label_y = h / 2;
        surface.write_str_clipped(label_x, label_y, &self.primary_label, fg_a, bg_a);

        // Divider
        for y in 0..h {
            surface.set_cell_styled(split_x, y, '│', Color::WHITE.dim(0.3), bg_a);
        }

        // Paint Zone B (Dropdown trigger)
        for y in 0..h {
            for x in (split_x + 1)..w {
                surface.set_cell_styled(x, y, ' ', fg_b, bg_b);
            }
        }
        let arrow_x = split_x + 1 + (w - split_x - 1).saturating_sub(1) / 2;
        surface.set_cell_styled(arrow_x, h / 2, '▼', fg_b, bg_b);
    }
}
