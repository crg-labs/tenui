use tenui_core::{CanvasSubviewMut, Color};

/// Declarative file drop zone widget providing spatial visual affordance during Drag-and-Drop.
#[derive(Debug, Clone)]
pub struct FileDropZone {
    pub label: String,
    pub is_hovered: bool,
    pub accepted_extensions: Vec<String>,
}

impl FileDropZone {
    pub fn new(label: &str) -> Self {
        Self {
            label: label.to_string(),
            is_hovered: false,
            accepted_extensions: Vec::new(),
        }
    }

    pub fn with_accepted_extensions(mut self, extensions: Vec<String>) -> Self {
        self.accepted_extensions = extensions;
        self
    }

    pub fn set_hovered(&mut self, hovered: bool) {
        self.is_hovered = hovered;
    }

    /// Renders visual drop affordances onto the subview:
    /// - If hovered: pulsating dashed border, boosted background luminance, centered drop badge.
    /// - If idle: subtle dashed outline and centered label.
    pub fn render(&self, subview: &mut CanvasSubviewMut<'_>) {
        if self.is_hovered {
            subview.draw_dashed_border(Color::Cyan);
            subview.boost_background_luminance(0.15);
            subview.write_centered_badge("[ ⇩ DROP FILES HERE ]", Color::Black, Color::Cyan);
        } else {
            subview.draw_dashed_border(Color::Rgb(80, 80, 100));
            let text = if self.label.is_empty() {
                "[ Drop Files Here ]"
            } else {
                &self.label
            };
            subview.write_centered_badge(text, Color::Rgb(140, 140, 160), Color::Rgb(35, 35, 50));
        }
    }
}
