use tenui_core::{CanvasSubviewMut, Color, Rect};
use tenui_layout::{BorderStyle, draw_border};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PopoverPlacement {
    Top,
    Bottom,
    Left,
    Right,
}

/// An anchor-positioned floating popover overlay.
pub struct Popover {
    pub anchor: Rect,
    pub width: u16,
    pub height: u16,
    pub placement: PopoverPlacement,
}

impl Popover {
    pub fn new(anchor: Rect, width: u16, height: u16, placement: PopoverPlacement) -> Self {
        Self {
            anchor,
            width,
            height,
            placement,
        }
    }

    /// Computes the clamped bounding box of the popover relative to the screen bounds.
    pub fn calculate_bounds(&self, screen: Rect) -> Rect {
        let (raw_x, raw_y) = match self.placement {
            PopoverPlacement::Bottom => (self.anchor.x, self.anchor.bottom()),
            PopoverPlacement::Top => (self.anchor.x, self.anchor.y.saturating_sub(self.height)),
            PopoverPlacement::Right => (self.anchor.right(), self.anchor.y),
            PopoverPlacement::Left => (self.anchor.x.saturating_sub(self.width), self.anchor.y),
        };

        // Clamp to screen bounds
        let max_x = screen.width.saturating_sub(self.width);
        let max_y = screen.height.saturating_sub(self.height);

        let clamped_x = raw_x.clamp(screen.x, max_x);
        let clamped_y = raw_y.clamp(screen.y, max_y);

        Rect::new(clamped_x, clamped_y, self.width, self.height)
    }

    pub fn render<F>(&self, subview: &mut CanvasSubviewMut<'_>, f: F)
    where
        F: FnOnce(&mut CanvasSubviewMut<'_>),
    {
        draw_border(subview, BorderStyle::ROUNDED, Color::DarkGray, Color::Reset);
        f(subview);
    }
}
