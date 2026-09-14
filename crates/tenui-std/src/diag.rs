use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Diagnostic severity level.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSeverity {
    Error,
    Warning,
    Help,
    Note,
}

impl DiagnosticSeverity {
    pub fn name(&self) -> &'static str {
        match self {
            DiagnosticSeverity::Error => "error",
            DiagnosticSeverity::Warning => "warning",
            DiagnosticSeverity::Help => "help",
            DiagnosticSeverity::Note => "note",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            DiagnosticSeverity::Error => Color::Rgb(243, 139, 168),   // Red
            DiagnosticSeverity::Warning => Color::Rgb(249, 226, 175), // Yellow
            DiagnosticSeverity::Help => Color::Rgb(166, 227, 161),    // Green
            DiagnosticSeverity::Note => Color::Rgb(137, 180, 250),    // Blue
        }
    }
}

/// A labeled span pointing to a location in the source code.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosticLabel {
    pub line: usize,      // 1-based
    pub col_start: usize, // 1-based
    pub col_end: usize,   // 1-based
    pub message: String,
    pub is_primary: bool,
}

impl DiagnosticLabel {
    pub fn primary(line: usize, col_start: usize, col_end: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col_start,
            col_end,
            message: message.into(),
            is_primary: true,
        }
    }

    pub fn secondary(line: usize, col_start: usize, col_end: usize, message: impl Into<String>) -> Self {
        Self {
            line,
            col_start,
            col_end,
            message: message.into(),
            is_primary: false,
        }
    }
}

/// A compiler-grade diagnostic report.
#[derive(Debug, Clone)]
pub struct Diagnostic {
    pub code: Option<String>,
    pub severity: DiagnosticSeverity,
    pub message: String,
    pub source_file: String,
    pub source_text: String,
    pub labels: Vec<DiagnosticLabel>,
    pub notes: Vec<String>,
    pub help: Option<String>,
    pub context_lines: usize,
}

impl Diagnostic {
    pub fn error(code: Option<impl Into<String>>, message: impl Into<String>) -> Self {
        Self {
            code: code.map(|c| c.into()),
            severity: DiagnosticSeverity::Error,
            message: message.into(),
            source_file: String::new(),
            source_text: String::new(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            context_lines: 2,
        }
    }

    pub fn warning(code: Option<impl Into<String>>, message: impl Into<String>) -> Self {
        Self {
            code: code.map(|c| c.into()),
            severity: DiagnosticSeverity::Warning,
            message: message.into(),
            source_file: String::new(),
            source_text: String::new(),
            labels: Vec::new(),
            notes: Vec::new(),
            help: None,
            context_lines: 2,
        }
    }

    pub fn with_source(mut self, file: impl Into<String>, text: impl Into<String>) -> Self {
        self.source_file = file.into();
        self.source_text = text.into();
        self
    }

    pub fn with_label(mut self, label: DiagnosticLabel) -> Self {
        self.labels.push(label);
        self
    }

    pub fn with_note(mut self, note: impl Into<String>) -> Self {
        self.notes.push(note.into());
        self
    }

    pub fn with_help(mut self, help: impl Into<String>) -> Self {
        self.help = Some(help.into());
        self
    }
}

/// Interactive DiagnosticView widget supporting cycling between error sites and expanding context lines.
#[derive(Debug, Clone)]
pub struct DiagnosticView {
    pub diagnostics: Vec<Diagnostic>,
    pub selected_idx: usize,
    pub expanded_context: bool,
    pub scroll_y: usize,
}

impl DiagnosticView {
    pub fn new(diagnostics: Vec<Diagnostic>) -> Self {
        Self {
            diagnostics,
            selected_idx: 0,
            expanded_context: false,
            scroll_y: 0,
        }
    }

    pub fn next_diagnostic(&mut self) {
        if !self.diagnostics.is_empty() {
            self.selected_idx = (self.selected_idx + 1) % self.diagnostics.len();
            self.scroll_y = 0;
        }
    }

    pub fn prev_diagnostic(&mut self) {
        if !self.diagnostics.is_empty() {
            self.selected_idx = if self.selected_idx == 0 {
                self.diagnostics.len() - 1
            } else {
                self.selected_idx - 1
            };
            self.scroll_y = 0;
        }
    }

    pub fn toggle_context(&mut self) {
        self.expanded_context = !self.expanded_context;
    }

    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 || self.diagnostics.is_empty() {
            return;
        }

        // Clear canvas
        for y in 0..canvas.height() {
            for x in 0..canvas.width() {
                canvas.set_char(x, y, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }
        }

        let diag = &self.diagnostics[self.selected_idx];
        let mut rendered_lines: Vec<DiagRenderLine> = Vec::new();

        // 1. Severity header: e.g. "error[E0382]: use of moved value: `data`"
        let code_str = match &diag.code {
            Some(c) => format!("[{}]", c),
            None => String::new(),
        };
        let header = format!("{}{}: {}", diag.severity.name(), code_str, diag.message);
        rendered_lines.push(DiagRenderLine {
            text: header,
            fg: diag.severity.color(),
            modifier: Modifier::BOLD,
        });

        // 2. Location pointer: "  --> src/main.rs:14:9"
        let first_label = diag
            .labels
            .iter()
            .find(|l| l.is_primary)
            .or_else(|| diag.labels.first());
        let loc_str = if let Some(lbl) = first_label {
            format!("  --> {}:{}:{}", diag.source_file, lbl.line, lbl.col_start)
        } else {
            format!("  --> {}", diag.source_file)
        };
        rendered_lines.push(DiagRenderLine {
            text: loc_str,
            fg: Color::Rgb(137, 180, 250),
            modifier: Modifier::empty(),
        });

        // Gutter line: "   │"
        rendered_lines.push(DiagRenderLine {
            text: "   │".to_string(),
            fg: Color::DarkGray,
            modifier: Modifier::empty(),
        });

        // 3. Source code snippet with annotated squiggly underlines
        let source_lines: Vec<&str> = diag.source_text.lines().collect();
        let context_radius = if self.expanded_context {
            diag.context_lines + 3
        } else {
            diag.context_lines
        };

        if let Some(lbl) = first_label {
            let target_line = lbl.line; // 1-based
            let start_l = target_line.saturating_sub(context_radius).max(1);
            let end_l = (target_line + context_radius).min(source_lines.len());

            for cur_l in start_l..=end_l {
                let line_text = source_lines.get(cur_l - 1).unwrap_or(&"");
                let line_num_str = format!("{:>3} │ ", cur_l);
                rendered_lines.push(DiagRenderLine {
                    text: format!("{}{}", line_num_str, line_text),
                    fg: Color::Rgb(205, 214, 244),
                    modifier: Modifier::empty(),
                });

                // Find labels on this line
                let line_labels: Vec<&DiagnosticLabel> = diag.labels.iter().filter(|l| l.line == cur_l).collect();
                for l in line_labels {
                    let indent = "   │ ".len() + l.col_start.saturating_sub(1);
                    let underline_len = (l.col_end.saturating_sub(l.col_start) + 1).max(1);
                    let squiggly = if l.is_primary {
                        "^".repeat(underline_len)
                    } else {
                        "-".repeat(underline_len)
                    };
                    let underline_line = format!(
                        "{}{}{}",
                        " ".repeat(indent),
                        squiggly,
                        if l.message.is_empty() {
                            "".into()
                        } else {
                            format!(" {}", l.message)
                        }
                    );
                    rendered_lines.push(DiagRenderLine {
                        text: underline_line,
                        fg: if l.is_primary {
                            diag.severity.color()
                        } else {
                            Color::Rgb(137, 180, 250)
                        },
                        modifier: Modifier::BOLD,
                    });
                }
            }
        }

        rendered_lines.push(DiagRenderLine {
            text: "   │".to_string(),
            fg: Color::DarkGray,
            modifier: Modifier::empty(),
        });

        // 4. Notes
        for note in &diag.notes {
            rendered_lines.push(DiagRenderLine {
                text: format!("   = note: {}", note),
                fg: Color::Rgb(180, 190, 254),
                modifier: Modifier::empty(),
            });
        }

        // 5. Help
        if let Some(h) = &diag.help {
            rendered_lines.push(DiagRenderLine {
                text: format!("   = help: {}", h),
                fg: Color::Rgb(166, 227, 161),
                modifier: Modifier::empty(),
            });
        }

        // Navigation footer
        rendered_lines.push(DiagRenderLine {
            text: format!(
                "─── Diagnostic {}/{}  [←/→] Cycle  [Space] Toggle Context ───",
                self.selected_idx + 1,
                self.diagnostics.len()
            ),
            fg: Color::DarkGray,
            modifier: Modifier::ITALIC,
        });

        // Blit to canvas
        for y in 0..h {
            let row_idx = self.scroll_y + y;
            if row_idx >= rendered_lines.len() {
                break;
            }
            let line = &rendered_lines[row_idx];
            for (cx, ch) in line.text.chars().enumerate() {
                if cx < w {
                    canvas.set_char(cx as u16, y as u16, ch, line.fg, Color::Reset, line.modifier);
                }
            }
        }
    }
}

struct DiagRenderLine {
    text: String,
    fg: Color,
    modifier: Modifier,
}
