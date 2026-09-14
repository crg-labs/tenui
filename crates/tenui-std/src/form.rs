use std::ops::RangeInclusive;

use tenui_core::{CanvasSubviewMut, Color, Modifier, Rect};

/// Kind of form input field.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FormFieldKind {
    Text,
    Password,
    Number,
    Checkbox,
    Select(Vec<String>),
}

/// A validated input field within a Form.
pub type FieldValidator = Box<dyn Fn(&str) -> Result<(), String> + Send + Sync>;

pub struct FormField {
    pub name: String,
    pub label: String,
    pub kind: FormFieldKind,
    pub value: String,
    pub range: Option<RangeInclusive<f64>>,
    pub validator: Option<FieldValidator>,
    pub error_message: Option<String>,
}

impl FormField {
    pub fn text(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind: FormFieldKind::Text,
            value: String::new(),
            range: None,
            validator: None,
            error_message: None,
        }
    }

    pub fn password(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind: FormFieldKind::Password,
            value: String::new(),
            range: None,
            validator: None,
            error_message: None,
        }
    }

    pub fn number(name: impl Into<String>, label: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind: FormFieldKind::Number,
            value: "0".into(),
            range: None,
            validator: None,
            error_message: None,
        }
    }

    pub fn checkbox(name: impl Into<String>, label: impl Into<String>, default: bool) -> Self {
        Self {
            name: name.into(),
            label: label.into(),
            kind: FormFieldKind::Checkbox,
            value: if default { "true".into() } else { "false".into() },
            range: None,
            validator: None,
            error_message: None,
        }
    }

    pub fn select(name: impl Into<String>, label: impl Into<String>, options: Vec<String>) -> Self {
        let initial = options.first().cloned().unwrap_or_default();
        Self {
            name: name.into(),
            label: label.into(),
            kind: FormFieldKind::Select(options),
            value: initial,
            range: None,
            validator: None,
            error_message: None,
        }
    }

    pub fn with_value(mut self, val: impl Into<String>) -> Self {
        self.value = val.into();
        self
    }

    pub fn with_range(mut self, range: RangeInclusive<f64>) -> Self {
        self.range = Some(range);
        self
    }

    pub fn with_validator<F>(mut self, validator: F) -> Self
    where
        F: Fn(&str) -> Result<(), String> + Send + Sync + 'static,
    {
        self.validator = Some(Box::new(validator));
        self
    }

    pub fn validate(&mut self) -> bool {
        self.error_message = None;

        // 1. Range validation for numbers
        if let Some(range) = &self.range {
            if let Ok(num) = self.value.trim().parse::<f64>() {
                if !range.contains(&num) {
                    self.error_message = Some(format!("Must be between {} and {}", range.start(), range.end()));
                    return false;
                }
            } else {
                self.error_message = Some("Invalid number".into());
                return false;
            }
        }

        // 2. Custom validator
        if let Some(val_fn) = self.validator.as_ref()
            && let Err(msg) = val_fn(&self.value)
        {
            self.error_message = Some(msg);
            return false;
        }

        true
    }
}

/// Declarative Form container managing layout, validation, and spatial focus.
#[derive(Default)]
pub struct Form {
    pub fields: Vec<FormField>,
    pub focused_idx: usize,
}

impl Form {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn field(mut self, field: FormField) -> Self {
        self.fields.push(field);
        self
    }

    pub fn add_field(&mut self, field: FormField) {
        self.fields.push(field);
    }

    pub fn get_value(&self, name: &str) -> Option<&str> {
        self.fields.iter().find(|f| f.name == name).map(|f| f.value.as_str())
    }

    pub fn set_value(&mut self, name: &str, val: impl Into<String>) {
        if let Some(f) = self.fields.iter_mut().find(|f| f.name == name) {
            f.value = val.into();
        }
    }

    pub fn next_field(&mut self) {
        if !self.fields.is_empty() {
            self.focused_idx = (self.focused_idx + 1) % self.fields.len();
        }
    }

    pub fn prev_field(&mut self) {
        if !self.fields.is_empty() {
            self.focused_idx = if self.focused_idx == 0 {
                self.fields.len() - 1
            } else {
                self.focused_idx - 1
            };
        }
    }

    pub fn handle_char(&mut self, c: char) {
        if let Some(field) = self.fields.get_mut(self.focused_idx) {
            match &field.kind {
                FormFieldKind::Text | FormFieldKind::Password => {
                    field.value.push(c);
                }
                FormFieldKind::Number => {
                    if c.is_ascii_digit() || c == '.' || (c == '-' && field.value.is_empty()) {
                        field.value.push(c);
                    }
                }
                FormFieldKind::Checkbox => {
                    if c == ' ' {
                        let cur = field.value == "true";
                        field.value = (!cur).to_string();
                    }
                }
                FormFieldKind::Select(opts) => {
                    if (c == ' ' || c == '\n')
                        && let Some(pos) = opts.iter().position(|o| o == &field.value)
                    {
                        let next = (pos + 1) % opts.len();
                        field.value = opts[next].clone();
                    }
                }
            }
        }
    }

    pub fn handle_backspace(&mut self) {
        if let Some(field) = self.fields.get_mut(self.focused_idx) {
            match field.kind {
                FormFieldKind::Text | FormFieldKind::Password | FormFieldKind::Number => {
                    field.value.pop();
                }
                _ => {}
            }
        }
    }

    pub fn validate(&mut self) -> bool {
        let mut all_valid = true;
        for field in &mut self.fields {
            if !field.validate() {
                all_valid = false;
            }
        }
        all_valid
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 {
            return;
        }

        let mut y = 0;
        for (i, field) in self.fields.iter().enumerate() {
            if y >= h {
                break;
            }

            let is_focused = i == self.focused_idx;
            let label_color = if is_focused {
                Color::Rgb(137, 180, 250)
            } else {
                Color::White
            };
            let border_color = if is_focused {
                Color::Rgb(137, 180, 250)
            } else {
                Color::DarkGray
            };

            // Field Label
            let label_str = format!("{}:", field.label);
            for (cx, ch) in label_str.chars().enumerate() {
                if cx < w {
                    canvas.set_char(cx as u16, y as u16, ch, label_color, Color::Reset, Modifier::BOLD);
                }
            }
            y += 1;

            if y >= h {
                break;
            }

            // Input widget representation
            match &field.kind {
                FormFieldKind::Checkbox => {
                    let checked = field.value == "true";
                    let box_str = if checked { "[✓] Enabled" } else { "[ ] Disabled" };
                    for (cx, ch) in box_str.chars().enumerate() {
                        if cx < w {
                            canvas.set_char(cx as u16, y as u16, ch, border_color, Color::Reset, Modifier::empty());
                        }
                    }
                }
                FormFieldKind::Select(_) => {
                    let select_str = format!("[◀ {} ▶]", field.value);
                    for (cx, ch) in select_str.chars().enumerate() {
                        if cx < w {
                            canvas.set_char(cx as u16, y as u16, ch, border_color, Color::Reset, Modifier::empty());
                        }
                    }
                }
                FormFieldKind::Password => {
                    let stars: String = "*".repeat(field.value.len());
                    let input_str = format!("│ {:<width$} │", stars, width = (w.saturating_sub(6)).min(30));
                    for (cx, ch) in input_str.chars().enumerate() {
                        if cx < w {
                            canvas.set_char(cx as u16, y as u16, ch, border_color, Color::Reset, Modifier::empty());
                        }
                    }
                }
                FormFieldKind::Text | FormFieldKind::Number => {
                    let input_str = format!("│ {:<width$} │", field.value, width = (w.saturating_sub(6)).min(30));
                    for (cx, ch) in input_str.chars().enumerate() {
                        if cx < w {
                            canvas.set_char(cx as u16, y as u16, ch, border_color, Color::Reset, Modifier::empty());
                        }
                    }
                }
            }
            y += 1;

            // Error banner if any
            if let Some(err) = &field.error_message
                && y < h
            {
                let err_str = format!("  ⚠ {}", err);
                for (cx, ch) in err_str.chars().enumerate() {
                    if cx < w {
                        canvas.set_char(
                            cx as u16,
                            y as u16,
                            ch,
                            Color::Rgb(243, 139, 168),
                            Color::Reset,
                            Modifier::empty(),
                        );
                    }
                }
                y += 1;
            }

            y += 1; // spacing between fields
        }
    }
}

/// A stage/step in a multi-step Wizard flow.
pub struct WizardStep {
    pub id: String,
    pub title: String,
    pub form: Form,
}

impl WizardStep {
    pub fn new(id: impl Into<String>, title: impl Into<String>, form: Form) -> Self {
        Self {
            id: id.into(),
            title: title.into(),
            form,
        }
    }
}

/// Multi-Step Wizard transactional flow controller.
pub struct Wizard {
    pub steps: Vec<WizardStep>,
    pub current_step: usize,
    pub is_complete: bool,
}

impl Wizard {
    pub fn new(steps: Vec<WizardStep>) -> Self {
        Self {
            steps,
            current_step: 0,
            is_complete: false,
        }
    }

    pub fn current_step_mut(&mut self) -> Option<&mut WizardStep> {
        self.steps.get_mut(self.current_step)
    }

    pub fn advance(&mut self) -> Result<bool, String> {
        if self.steps.is_empty() {
            return Ok(true);
        }

        // Validate active form
        let valid = self.steps[self.current_step].form.validate();
        if !valid {
            return Err("Current step has validation errors".into());
        }

        if self.current_step + 1 < self.steps.len() {
            self.current_step += 1;
            Ok(false)
        } else {
            self.is_complete = true;
            Ok(true)
        }
    }

    pub fn back(&mut self) -> bool {
        if self.current_step > 0 {
            self.current_step -= 1;
            self.is_complete = false;
            true
        } else {
            false
        }
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h < 4 {
            return;
        }

        // Clear canvas
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                canvas.set_char(x, y, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }
        }

        // 1. Wizard step indicator header: [1. Title] ───► [2. Title] ...
        let mut header = String::new();
        for (i, step) in self.steps.iter().enumerate() {
            let indicator = if i == self.current_step {
                format!("▶ [{}. {}] ", i + 1, step.title)
            } else if i < self.current_step {
                format!("✓ [{}. {}] ", i + 1, step.title)
            } else {
                format!("  [{}. {}] ", i + 1, step.title)
            };
            header.push_str(&indicator);
            if i + 1 < self.steps.len() {
                header.push_str("───► ");
            }
        }

        for (cx, ch) in header.chars().enumerate() {
            if cx < w {
                canvas.set_char(
                    cx as u16,
                    0,
                    ch,
                    Color::Rgb(249, 226, 175),
                    Color::Reset,
                    Modifier::BOLD,
                );
            }
        }

        // Divider
        for cx in 0..canvas.width() {
            canvas.set_char(cx, 1, '─', Color::DarkGray, Color::Reset, Modifier::empty());
        }

        // 2. Render active form
        if let Some(step) = self.steps.get(self.current_step) {
            let clip = Rect::new(0, 2, canvas.width(), canvas.height().saturating_sub(3));
            let mut form_subview = canvas.subview_mut(clip);
            step.form.render(&mut form_subview);
        }

        // 3. Bottom controls footer
        let footer = if self.is_complete {
            "✓ Setup Completed Successfully! Press [Enter] to Exit."
        } else {
            "[Enter] Next / Finish   [Esc] Back   [Tab] Next Field"
        };
        let footer_y = (h - 1) as u16;
        for (cx, ch) in footer.chars().enumerate() {
            if cx < w {
                canvas.set_char(cx as u16, footer_y, ch, Color::DarkGray, Color::Reset, Modifier::ITALIC);
            }
        }
    }
}
