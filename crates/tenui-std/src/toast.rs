use tenui_core::{CanvasSubviewMut, Modifier};

use crate::theme::ThemePalette;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastLevel {
    Info,
    Success,
    Warning,
    Error,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ToastPosition {
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

pub struct Toast {
    pub message: String,
    pub level: ToastLevel,
    pub remaining_ticks: u32,
}

impl Toast {
    pub fn new(message: impl Into<String>, level: ToastLevel, duration_ticks: u32) -> Self {
        Self {
            message: message.into(),
            level,
            remaining_ticks: duration_ticks,
        }
    }

    pub fn info(message: impl Into<String>, ticks: u32) -> Self {
        Self::new(message, ToastLevel::Info, ticks)
    }

    pub fn success(message: impl Into<String>, ticks: u32) -> Self {
        Self::new(message, ToastLevel::Success, ticks)
    }

    pub fn warning(message: impl Into<String>, ticks: u32) -> Self {
        Self::new(message, ToastLevel::Warning, ticks)
    }

    pub fn error(message: impl Into<String>, ticks: u32) -> Self {
        Self::new(message, ToastLevel::Error, ticks)
    }

    pub fn is_expired(&self) -> bool {
        self.remaining_ticks == 0
    }

    pub fn tick(&mut self) {
        self.remaining_ticks = self.remaining_ticks.saturating_sub(1);
    }
}

pub struct ToastStack {
    pub toasts: Vec<Toast>,
    pub position: ToastPosition,
    pub max_visible: usize,
}

impl ToastStack {
    pub fn new(position: ToastPosition) -> Self {
        Self {
            toasts: Vec::new(),
            position,
            max_visible: 5,
        }
    }

    pub fn push(&mut self, toast: Toast) {
        self.toasts.push(toast);
    }

    pub fn tick(&mut self) {
        for t in &mut self.toasts {
            t.tick();
        }
        self.toasts.retain(|t| !t.is_expired());
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>, palette: &ThemePalette) {
        let screen_w = canvas.width();
        let screen_h = canvas.height();

        let visible: Vec<&Toast> = self.toasts.iter().rev().take(self.max_visible).collect();

        for (i, toast) in visible.iter().enumerate() {
            let toast_w = (toast.message.len() + 6).min(screen_w as usize) as u16;
            let x = match self.position {
                ToastPosition::TopRight | ToastPosition::BottomRight => screen_w.saturating_sub(toast_w + 1),
                ToastPosition::TopLeft | ToastPosition::BottomLeft => 1,
            };
            let y = match self.position {
                ToastPosition::TopRight | ToastPosition::TopLeft => (i as u16) + 1,
                ToastPosition::BottomRight | ToastPosition::BottomLeft => screen_h.saturating_sub((i as u16) + 2),
            };

            if y >= screen_h {
                continue;
            }

            let (icon, accent) = match toast.level {
                ToastLevel::Info => ("ℹ", palette.primary),
                ToastLevel::Success => ("✓", palette.success),
                ToastLevel::Warning => ("⚠", palette.warning),
                ToastLevel::Error => ("✗", palette.error),
            };

            let text = format!(" {} {} ", icon, toast.message);
            let bg = palette.surface;

            for (cx, ch) in text.chars().enumerate() {
                let draw_x = x + cx as u16;
                if draw_x < screen_w {
                    let fg = if cx < 3 { accent } else { palette.fg };
                    canvas.set_char(draw_x, y, ch, fg, bg, Modifier::empty());
                }
            }
        }
    }
}

impl Default for ToastStack {
    fn default() -> Self {
        Self::new(ToastPosition::TopRight)
    }
}

#[cfg(test)]
mod tests {
    use tenui_core::{Buffer, Rect};

    use super::*;

    #[test]
    fn toast_lifecycle() {
        let mut toast = Toast::info("saved", 3);
        assert!(!toast.is_expired());
        toast.tick();
        toast.tick();
        toast.tick();
        assert!(toast.is_expired());
    }

    #[test]
    fn toast_stack_removes_expired() {
        let mut stack = ToastStack::default();
        stack.push(Toast::info("a", 1));
        stack.push(Toast::success("b", 3));

        stack.tick();
        assert_eq!(stack.toasts.len(), 1);
        assert_eq!(stack.toasts[0].message, "b");
    }

    #[test]
    fn toast_stack_renders() {
        let mut stack = ToastStack::new(ToastPosition::TopRight);
        stack.push(Toast::error("connection failed", 60));

        let palette = ThemePalette::catppuccin_mocha();
        let mut buf = Buffer::new(50, 10);
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 50, 10));
        stack.render(&mut canvas, &palette);

        let cell = buf.get(49u16.saturating_sub(20), 1).unwrap();
        assert_ne!(cell.symbol.as_str(), " ");
    }
}
