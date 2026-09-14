use std::collections::HashMap;

use tenui_core::{Buffer, Color, Rect};
use tenui_std::{
    chart::{
        BarChart, BarItem, BarOrientation, Candle, CandlestickChart, Heatmap, LinePlot, PlotCanvasType, ScatterPlot,
        Series, StackedBarChart, StackedSegment,
    },
    diag::{Diagnostic, DiagnosticLabel, DiagnosticView},
    doc::{DiffMode, DiffView, MarkdownBlock, MarkdownView, MyersDiff},
    form::{Form, FormField, Wizard, WizardStep},
    pty::TerminalPane,
    syntax::{CodeView, Language, LineTokenizer, SyntaxTheme, TokenKind},
    theme::{ThemePalette, contrast_ratio, ensure_contrast},
};

#[test]
fn test_syntax_line_tokenizer() {
    let code = "fn main() {\n    let x: u32 = 42;\n    // Comment\n    println!(\"hello\");\n}";
    let mut tokens = Vec::new();

    for line in code.lines() {
        LineTokenizer::tokenize_line(Language::Rust, line, |span| {
            tokens.push(span);
        });
    }

    assert!(tokens.iter().any(|t| t.kind == TokenKind::Keyword));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Type));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Number));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::Comment));
    assert!(tokens.iter().any(|t| t.kind == TokenKind::StringLiteral));
}

#[test]
fn test_tex_syntax_tokenizer() {
    assert_eq!(Language::from_extension("tex"), Language::Tex);
    assert_eq!(Language::from_extension("latex"), Language::Tex);
    assert_eq!(Language::from_extension("sty"), Language::Tex);
    assert_eq!(Language::from_extension("cls"), Language::Tex);

    let tex_doc = r#"\documentclass[12pt]{article}
\usepackage{amsmath}
% Preamble comment
\begin{document}
\section{Mathematics}
Let $E = mc^2$ and display:
\[ \int_0^\infty f(x) dx = 1 \]
A table row: a & b \\ % end row
Dimension: \vspace{1.5em} and 100\%
\end{document}"#;

    let mut tokens = Vec::new();
    for line in tex_doc.lines() {
        LineTokenizer::tokenize_line(Language::Tex, line, |span| {
            tokens.push((span.kind, &line[span.start..span.end]));
        });
    }

    // Check keywords
    assert!(
        tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "\\documentclass")
    );
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "\\begin"));
    assert!(
        tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "\\section")
    );
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "\\end"));

    // Check types / environments
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == "article"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == "document"));

    // Check comments
    assert!(
        tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Comment && s.contains("Preamble comment"))
    );

    // Check numbers with units
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "12pt"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "1.5em"));

    // Check math delimiters
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "$"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "\\["));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "\\]"));

    // Check special operators
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "^"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "_"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == "&"));

    // Check escaped percent (is Punctuation, not Comment)
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Punctuation && *s == "\\%"));
}

#[test]
fn test_go_syntax_tokenizer() {
    let go_code = r#"package main
// Line comment
func Process(val int) string {
    msg := `hello world`
    return msg
}"#;

    let mut tokens = Vec::new();
    for line in go_code.lines() {
        LineTokenizer::tokenize_line(Language::Go, line, |span| {
            tokens.push((span.kind, &line[span.start..span.end]));
        });
    }

    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "package"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "func"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "return"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == "int"));
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == "string"));
    assert!(
        tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Comment && s.contains("Line comment"))
    );
    assert!(tokens.iter().any(|(k, s)| *k == TokenKind::Operator && *s == ":="));
    assert!(
        tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::StringLiteral && *s == "`hello world`")
    );
}

#[test]
fn test_toml_yaml_c_syntax_tokenizer() {
    // TOML
    let toml_code = "[package]\nname = \"tenui\"\nversion = 1.0";
    let mut toml_tokens = Vec::new();
    for line in toml_code.lines() {
        LineTokenizer::tokenize_line(Language::Toml, line, |span| {
            toml_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    assert!(
        toml_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "[package]")
    );
    assert!(
        toml_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::StringLiteral && *s == "\"tenui\"")
    );
    assert!(toml_tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "1.0"));

    // YAML
    let yaml_code = "service:\n  name: tenui\n  active: true";
    let mut yaml_tokens = Vec::new();
    for line in yaml_code.lines() {
        LineTokenizer::tokenize_line(Language::Yaml, line, |span| {
            yaml_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    assert!(
        yaml_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "service")
    );
    assert!(
        yaml_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "true")
    );

    // C
    let c_code = "#include <stdio.h>\nint main() {\n    return 0;\n}";
    let mut c_tokens = Vec::new();
    for line in c_code.lines() {
        LineTokenizer::tokenize_line(Language::C, line, |span| {
            c_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    assert!(
        c_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Function && *s == "#include")
    );
    assert!(c_tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "int"));
    assert!(c_tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "return"));
    assert!(c_tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "0"));
}

#[test]
fn test_sql_html_css_syntax_tokenizer() {
    // SQL — keywords are case-insensitive
    assert_eq!(Language::from_extension("sql"), Language::Sql);
    let sql_code = "select Name from Users where id = 10; -- trailing";
    let mut sql_tokens = Vec::new();
    for line in sql_code.lines() {
        LineTokenizer::tokenize_line(Language::Sql, line, |span| {
            sql_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    // lowercase `select`/`from`/`where` still resolve as keywords
    assert!(
        sql_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "select")
    );
    assert!(sql_tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "from"));
    assert!(sql_tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "10"));
    assert!(
        sql_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Comment && s.contains("trailing"))
    );

    // HTML
    assert_eq!(Language::from_extension("html"), Language::Html);
    let html_code = "<!-- note --><a href=\"https://x.io\">Hello</a>";
    let mut html_tokens = Vec::new();
    for line in html_code.lines() {
        LineTokenizer::tokenize_line(Language::Html, line, |span| {
            html_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    assert!(
        html_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Comment && s.contains("note"))
    );
    assert!(html_tokens.iter().any(|(k, s)| *k == TokenKind::Keyword && *s == "a"));
    assert!(html_tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == "href"));
    assert!(
        html_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::StringLiteral && *s == "\"https://x.io\"")
    );
    assert!(
        html_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::PlainText && *s == "Hello")
    );

    // CSS
    assert_eq!(Language::from_extension("css"), Language::Css);
    let css_code = ".btn { color: #fff; margin: 12px; } /* c */";
    let mut css_tokens = Vec::new();
    for line in css_code.lines() {
        LineTokenizer::tokenize_line(Language::Css, line, |span| {
            css_tokens.push((span.kind, &line[span.start..span.end]));
        });
    }
    assert!(css_tokens.iter().any(|(k, s)| *k == TokenKind::Type && *s == ".btn"));
    assert!(
        css_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Keyword && *s == "color")
    );
    assert!(css_tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "#fff"));
    assert!(css_tokens.iter().any(|(k, s)| *k == TokenKind::Number && *s == "12px"));
    assert!(
        css_tokens
            .iter()
            .any(|(k, s)| *k == TokenKind::Comment && s.contains("c"))
    );
}

#[test]
fn framework_tokenizer_handles_tenui_chat_crash_lines() {
    // The exact lines tenui-chat crashed on (agent-tasks/09). Prove the
    // framework tokenizer alone — no call-site wrapper — now slices every span
    // safely AND reconstructs the input losslessly.
    let cases = [
        "fn calculate() -> Result<Value> { … }",
        "assert_eq!(app.theme.name, \"…\");",
        "let prompt = “Hello — World”;",
        "let emoji = 🚀 + 💡; // 这是一个测试",
    ];
    for src in cases {
        let mut rebuilt = String::new();
        LineTokenizer::tokenize_line(Language::Rust, src, |span| {
            assert!(
                src.is_char_boundary(span.start) && src.is_char_boundary(span.end),
                "{src:?}: span splits a char"
            );
            rebuilt.push_str(&src[span.start..span.end]); // the exact slice that panicked
        });
        assert_eq!(rebuilt, src, "tokenizer must cover the whole line losslessly");
    }
}

#[test]
fn tokenizers_emit_char_boundary_spans_for_multibyte_input() {
    // Regression (correctness-dos): the catch-all arms used to emit a one-byte
    // span for a non-ASCII lead byte, so `&line[span.start..span.end]` — exactly
    // how CodeView and MarkdownView slice tokens — panicked mid-character on
    // untrusted source. Every emitted span must land on a char boundary.
    let langs = [
        Language::Rust,
        Language::Python,
        Language::Javascript,
        Language::Json,
        Language::Shell,
        Language::Markdown,
        Language::Tex,
        Language::Toml,
        Language::Yaml,
        Language::C,
        Language::Go,
        Language::Sql,
        Language::Html,
        Language::Css,
        Language::PlainText,
    ];
    // A battery of multi-byte characters (2-, 3-, and 4-byte), including the exact
    // ellipsis that crashed tenui-chat, placed at every risky position: after
    // operators/keywords/numbers, adjacent to YAML markers, and inside strings,
    // comments, and tags. Each is raw-sliced exactly as the original (unsafe)
    // caller did — the panic would fire here if any span split a character.
    let mbs = ['…', '—', '“', '”', 'λ', 'π', '你', '⚠', '🦀', '💡', 'é', '→', '±', '§'];
    for lang in langs {
        for c in mbs {
            let lines = [
                format!("let x = {c};"),
                format!("a{c}b = {c}"),
                format!("1{c} 12em{c} 3.14{c}"),
                format!("--{c} ...{c} #{c}"),
                format!("<a href=\"{c}\">{c}</a>"),
                format!("/* {c} */ // {c}"),
                format!("\"{c}\" '{c}' `{c}`"),
                format!("{c}{c}{c}{c}"),
                format!("SELECT {c} FROM t -- {c}"),
                format!(".btn{c} {{ color: #{c}; }}"),
            ];
            for line in &lines {
                LineTokenizer::tokenize_line(lang, line, |span| {
                    assert!(
                        line.is_char_boundary(span.start),
                        "{lang:?} {line:?}: start {} splits a char",
                        span.start
                    );
                    assert!(
                        line.is_char_boundary(span.end),
                        "{lang:?} {line:?}: end {} splits a char",
                        span.end
                    );
                    // Must not panic — the exact slice the renderers perform.
                    let _ = &line[span.start..span.end];
                });
            }
        }
    }
}

#[test]
fn test_code_view_rendering() {
    let mut buf = Buffer::new(50, 10);
    let rust_code = "fn compute() {\n    let val = 100;\n}";
    let mut blame = HashMap::new();
    blame.insert(0, "e8a109f user".to_string());

    let mut view = CodeView::new(rust_code, Language::Rust)
        .with_theme(SyntaxTheme::dark())
        .with_blame(blame);

    view.toggle_fold(0);
    assert!(view.folded_lines.contains(&0));

    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 50, 10));
        view.render(&mut canvas);
    }

    let row0 = (0..50)
        .map(|x| buf.get(x, 0).map(|c| c.symbol.as_str()).unwrap_or(" "))
        .collect::<String>();
    assert!(row0.contains("1")); // line number
    assert!(row0.contains("e8a109f")); // blame
    assert!(row0.contains("fn")); // code
}

#[test]
fn test_myers_diff_and_intra_highlights() {
    let old_text = "fn calculate(x: i32) -> i32 {\n    x + 1\n}";
    let new_text = "fn calculate(x: i32) -> i64 {\n    x + 2\n}";

    let hunks = MyersDiff::diff(old_text, new_text);
    assert_eq!(hunks.len(), 1);
    let hunk = &hunks[0];

    assert!(
        hunk.lines
            .iter()
            .any(|l| l.change == tenui_std::doc::ChangeType::Delete)
    );
    assert!(
        hunk.lines
            .iter()
            .any(|l| l.change == tenui_std::doc::ChangeType::Insert)
    );

    let mut buf = Buffer::new(80, 10);
    let mut diff_view = DiffView::new(old_text, new_text);

    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 80, 10));
        diff_view.render(&mut canvas);
    }

    // Test Split layout
    diff_view.set_mode(DiffMode::Split);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 80, 10));
        diff_view.render(&mut canvas);
    }

    let row = (0..80)
        .map(|x| buf.get(x, 0).map(|c| c.symbol.as_str()).unwrap_or(" "))
        .collect::<String>();
    assert!(row.contains("│")); // separator
}

#[test]
fn test_markdown_viewer() {
    let md = "# Title\n\nThis is **bold** paragraph.\n\n- Item 1\n- Item 2\n\n> Quote\n\n```rust\nlet a = 1;\n```\n\n| Col1 | Col2 |\n|---|---|\n| A | B |";
    let blocks = MarkdownView::parse(md);

    assert!(blocks.iter().any(|b| matches!(b, MarkdownBlock::Header(1, _))));
    assert!(blocks.iter().any(|b| matches!(b, MarkdownBlock::BulletList(_, _))));
    assert!(blocks.iter().any(|b| matches!(b, MarkdownBlock::Blockquote(_))));
    assert!(blocks.iter().any(|b| matches!(b, MarkdownBlock::CodeBlock(_, _))));
    assert!(blocks.iter().any(|b| matches!(b, MarkdownBlock::Table(_, _, _))));

    let mut buf = Buffer::new(60, 25);
    let mut view = MarkdownView::new(md);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 25));
        view.render(&mut canvas);
    }
}

#[test]
fn test_charting_suite() {
    let mut buf = Buffer::new(60, 20);

    // 1. LinePlot (Braille and HalfBlock)
    let s1 = Series::new(
        "sin",
        Color::Green,
        vec![(0.0, 0.0), (1.0, 0.8), (2.0, 0.2), (3.0, 1.0)],
    );
    let plot = LinePlot::new()
        .add_series(s1.clone())
        .with_canvas_type(PlotCanvasType::Braille);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        plot.render(&mut canvas);
    }

    let plot_hb = LinePlot::new()
        .add_series(s1)
        .with_canvas_type(PlotCanvasType::HalfBlock);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        plot_hb.render(&mut canvas);
    }

    // 2. ScatterPlot
    let scatter = ScatterPlot::new().add_series(Series::new("pts", Color::Yellow, vec![(1.0, 2.0), (3.0, 4.0)]));
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        scatter.render(&mut canvas);
    }

    // 3. BarChart (Horizontal & Vertical with 1/8th micro-stepping)
    let b1 = BarItem {
        label: "Rust".into(),
        value: 85.5,
        color: Color::Red,
    };
    let b2 = BarItem {
        label: "Go".into(),
        value: 42.2,
        color: Color::Cyan,
    };
    let bar_h = BarChart::new(vec![b1.clone(), b2.clone()]).with_orientation(BarOrientation::Horizontal);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        bar_h.render(&mut canvas);
    }

    let bar_v = BarChart::new(vec![b1, b2]).with_orientation(BarOrientation::Vertical);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        bar_v.render(&mut canvas);
    }

    // 4. StackedBarChart
    let stacked = StackedBarChart::new(vec![(
        "2024".into(),
        vec![
            StackedSegment {
                value: 10.0,
                color: Color::Blue,
            },
            StackedSegment {
                value: 20.0,
                color: Color::Green,
            },
        ],
    )]);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        stacked.render(&mut canvas);
    }

    // 5. CandlestickChart
    let candle = Candle {
        open: 100.0,
        high: 120.0,
        low: 95.0,
        close: 115.0,
        volume: 5000.0,
    };
    let cchart = CandlestickChart::new(vec![candle]);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        cchart.render(&mut canvas);
    }

    // 6. Heatmap
    let heatmap = Heatmap::new(vec![vec![1.0, 5.0, 10.0], vec![2.0, 8.0, 4.0]]);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 20));
        heatmap.render(&mut canvas);
    }
}

#[test]
fn test_declarative_form_and_wizard() {
    let mut form = Form::new()
        .field(FormField::text("endpoint", "Cluster URL").with_validator(|v| {
            if v.starts_with("http") {
                Ok(())
            } else {
                Err("URL must start with http".into())
            }
        }))
        .field(FormField::number("replicas", "Worker Replicas").with_range(1.0..=64.0))
        .field(FormField::checkbox("mtls", "Enable Mutual TLS", true));

    // Initially endpoint is empty -> validation should fail
    assert!(!form.validate());

    // Fix values
    form.set_value("endpoint", "https://prod.cluster.local");
    form.set_value("replicas", "8");
    assert!(form.validate());

    let mut buf = Buffer::new(40, 15);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 15));
        form.render(&mut canvas);
    }

    // Multi-step Wizard flow
    let step1 = WizardStep::new("step1", "Endpoints", form);
    let step2_form = Form::new().field(FormField::text("admin", "Admin Email"));
    let step2 = WizardStep::new("step2", "Review", step2_form);

    let mut wizard = Wizard::new(vec![step1, step2]);
    assert_eq!(wizard.current_step, 0);

    let adv1 = wizard.advance();
    assert!(adv1.is_ok());
    assert_eq!(wizard.current_step, 1);

    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 15));
        wizard.render(&mut canvas);
    }

    wizard.back();
    assert_eq!(wizard.current_step, 0);
}

#[test]
fn test_diagnostic_view() {
    let diag = Diagnostic::error(Some("E0382"), "use of moved value: `data`")
        .with_source(
            "src/main.rs",
            "fn main() {\n    let data = vec![1, 2];\n    drop(data);\n    println!(\"{:?}\", data);\n}",
        )
        .with_label(DiagnosticLabel::secondary(3, 10, 14, "value moved here"))
        .with_label(DiagnosticLabel::primary(4, 22, 26, "value used here after move"))
        .with_note("this error originates in a macro")
        .with_help("consider cloning the value");

    let mut view = DiagnosticView::new(vec![diag]);
    let mut buf = Buffer::new(60, 15);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 15));
        view.render(&mut canvas);
    }

    view.toggle_context();
    assert!(view.expanded_context);
}

#[test]
fn test_theme_store_and_wcag_auditor() {
    let mocha = ThemePalette::catppuccin_mocha();
    let latte = ThemePalette::catppuccin_latte();
    let gruvbox = ThemePalette::gruvbox();
    let tokyo = ThemePalette::tokyo_night();
    let nord = ThemePalette::nord();
    let solarized = ThemePalette::solarized_dark();
    let monokai = ThemePalette::monokai();

    assert!(mocha.is_dark);
    assert!(!latte.is_dark);
    assert!(gruvbox.is_dark);
    assert!(tokyo.is_dark);
    assert!(nord.is_dark);
    assert!(solarized.is_dark);
    assert!(monokai.is_dark);

    // WCAG Contrast calculation
    let bg = Color::Rgb(0, 0, 0);
    let fg = Color::Rgb(255, 255, 255);
    let ratio = contrast_ratio(fg, bg);
    assert!(ratio >= 20.0);

    // Low contrast pair
    let low_fg = Color::Rgb(20, 20, 20);
    let low_ratio = contrast_ratio(low_fg, bg);
    assert!(low_ratio < 4.5);

    // Automatic luminance boosting to >= 4.5
    let boosted_fg = ensure_contrast(low_fg, bg, 4.5);
    let boosted_ratio = contrast_ratio(boosted_fg, bg);
    assert!(boosted_ratio >= 4.5);
}

#[test]
fn test_terminal_pane_vt100_parser() {
    let mut pane = TerminalPane::new(20, 5);

    // Write "Hello\r\nWorld"
    pane.process_bytes(b"Hello\r\nWorld");
    assert_eq!(pane.cursor_x, 5);
    assert_eq!(pane.cursor_y, 1);

    // Cursor position CUP \x1b[3;4H
    pane.process_bytes(b"\x1b[3;4H");
    assert_eq!(pane.cursor_y, 2); // 0-based row 2 is 1-based row 3
    assert_eq!(pane.cursor_x, 3); // 0-based col 3 is 1-based col 4

    // SGR styling: \x1b[1;31m (Bold Red)
    pane.process_bytes(b"\x1b[1;31m!");
    let idx = (pane.cursor_y as usize) * (pane.width as usize) + (pane.cursor_x as usize - 1);
    let cell = pane.cells[idx];
    assert_eq!(cell.symbol.as_str(), "!");
    assert_eq!(cell.fg, Color::Red);
    assert!(cell.modifier.contains(tenui_core::Modifier::BOLD));

    // Clear screen ED 2
    pane.process_bytes(b"\x1b[2J");
    assert!(pane.cells.iter().all(|c| c.symbol.as_str() == " "));

    let mut buf = Buffer::new(20, 5);
    {
        let mut canvas = buf.subview_mut(Rect::new(0, 0, 20, 5));
        pane.render(&mut canvas);
    }
}

#[test]
fn test_aligned_diff_split_padding() {
    use tenui_std::doc::{ChangeType, DiffView};

    let old_text = "line1\nline2\nline3\nline4\ncommon\n";
    let new_text = "new1\nnew2\ncommon\n";

    let view = DiffView::new(old_text, new_text);
    let aligned = DiffView::aligned_split_lines(&view.hunks);

    // First 4 rows: left has deletions line1..line4; right has additions new1..new2, then None, None
    assert_eq!(aligned.len(), 5); // 4 changes + 1 common
    assert_eq!(aligned[0].left.unwrap().change, ChangeType::Delete);
    assert_eq!(aligned[0].right.unwrap().change, ChangeType::Insert);

    assert_eq!(aligned[1].left.unwrap().change, ChangeType::Delete);
    assert_eq!(aligned[1].right.unwrap().change, ChangeType::Insert);

    assert_eq!(aligned[2].left.unwrap().change, ChangeType::Delete);
    assert_eq!(
        aligned[2].right, None,
        "Shorter additions side must be padded with None"
    );

    assert_eq!(aligned[3].left.unwrap().change, ChangeType::Delete);
    assert_eq!(
        aligned[3].right, None,
        "Shorter additions side must be padded with None"
    );

    // Row 4: common line aligned on both sides
    assert_eq!(aligned[4].left.unwrap().change, ChangeType::Equal);
    assert_eq!(aligned[4].right.unwrap().change, ChangeType::Equal);
    assert!(!aligned[4].is_change);
}

#[test]
fn test_contrast_auditor_physical_cell() {
    use tenui_std::contrast_ratio;

    let ratio = contrast_ratio(Color::Rgb(255, 255, 255), Color::Rgb(0, 0, 0));
    assert!(ratio >= 20.0, "White on black must achieve near maximum 21:1 contrast");

    let low_ratio = contrast_ratio(Color::Rgb(20, 20, 20), Color::Rgb(0, 0, 0));
    assert!(low_ratio < 2.0, "Dark gray on black should fail WCAG AA (ratio < 2.0)");
}

#[test]
fn test_markdown_renderer_to_subview() {
    use tenui_std::doc::MarkdownRenderer;

    let mut buf = Buffer::new(40, 10);
    let md = "# Title\n\n> Quote text\n\n```rust\nlet x = 42;\n```";
    {
        let mut subview = buf.subview_mut(Rect::new(0, 0, 40, 10));
        MarkdownRenderer::render_to_subview(&mut subview, md);
    }

    // Verify header prefix was rendered
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), "#");
    // Verify blockquote bar was rendered
    assert_eq!(buf.get(0, 2).unwrap().symbol.as_str(), "▎");
}

#[test]
fn test_code_line_segregation() {
    use tenui_std::syntax::render_code_line;

    let mut buf = Buffer::new(60, 2);
    {
        let mut subview = buf.subview_mut(Rect::new(0, 0, 60, 2));
        render_code_line(&mut subview, 0, 42, Some(("a1b2c3d", "alice")), "let x = 1;", 100);
    }

    // Line number at col 0 (3 digits for 100 max lines: " 42")
    assert_eq!(buf.get(0, 0).unwrap().symbol.as_str(), " ");
    assert_eq!(buf.get(1, 0).unwrap().symbol.as_str(), "4");
    assert_eq!(buf.get(2, 0).unwrap().symbol.as_str(), "2");

    // Separator │ at num_digits + blame_width = 3 + 16 = 19
    assert_eq!(buf.get(19, 0).unwrap().symbol.as_str(), "│");

    // Code starts at col 20
    assert_eq!(buf.get(20, 0).unwrap().symbol.as_str(), "l");
    assert_eq!(buf.get(21, 0).unwrap().symbol.as_str(), "e");
    assert_eq!(buf.get(22, 0).unwrap().symbol.as_str(), "t");
}

#[test]
fn test_font_banner_braille_and_unicode_typography() {
    use tenui_std::{Banner, BannerAlignment, BigFontStyle, BrailleCanvas, UnicodeFont};

    // 1. Banner test
    let banner = Banner::new("TEST")
        .with_fg(Color::LightGreen)
        .with_style(BigFontStyle::HalfBlock)
        .with_alignment(BannerAlignment::Left);

    let (bw, bh) = banner.measure();
    assert_eq!(bw, 19); // 4 chars * 5 - 1 = 19
    assert_eq!(bh, 3);

    let mut buf = Buffer::new(30, 4);
    {
        let mut subview = buf.subview_mut(Rect::new(0, 0, 30, 4));
        banner.render(&mut subview);
    }
    // Verify half-block / full-block characters rendered
    let rendered_chars: String = (0..bw).map(|x| buf.get(x, 0).unwrap().symbol.as_str()).collect();
    assert!(rendered_chars.contains('█') || rendered_chars.contains('▀'));

    // 2. Braille canvas test
    let mut braille = BrailleCanvas::new(15, 3);
    braille.draw_line(0, 0, 10, 10);
    braille.draw_text(2, 2, "OK");
    let braille_str = braille.to_braille_string();
    assert!(braille_str.chars().any(|c| ('\u{2800}'..='\u{28FF}').contains(&c)));

    // 3. UnicodeFont transformations
    let text = "Tenui 123";
    assert_eq!(UnicodeFont::SansBold.transform(text), "𝗧𝗲𝗻𝘂𝗶 𝟭𝟮𝟯");
    assert_eq!(UnicodeFont::Monospace.transform(text), "𝚃𝚎𝚗𝚞𝚒 𝟷𝟸𝟹");
    assert_eq!(UnicodeFont::SmallCaps.transform("abc"), "ᴀʙᴄ");
    assert_eq!(UnicodeFont::Circled.transform("ABC"), "ⒶⒷⒸ");
}

#[test]
fn test_icons_system_and_resolution() {
    use tenui_std::{Icon, IconMode};

    // Check language icons
    assert_eq!(Icon::Rust.as_str(IconMode::NerdFont), "");
    assert_eq!(Icon::Rust.as_str(IconMode::Ascii), "[RS]");
    assert_eq!(Icon::Rust.as_str(IconMode::Unicode), "🦀");

    assert_eq!(Icon::Go.as_str(IconMode::NerdFont), "");
    assert_eq!(Icon::Go.as_str(IconMode::Ascii), "[GO]");

    assert_eq!(Icon::GitBranch.as_str(IconMode::NerdFont), "");
    assert_eq!(Icon::GitBranch.as_str(IconMode::Ascii), "[branch]");

    // Check file extension inference
    assert_eq!(Icon::from_extension("rs"), Icon::Rust);
    assert_eq!(Icon::from_extension("go"), Icon::Go);
    assert_eq!(Icon::from_extension("py"), Icon::Python);
    assert_eq!(Icon::from_extension("dockerfile"), Icon::Docker);
    assert_eq!(Icon::from_extension("zip"), Icon::FileArchive);

    // Path inference
    assert_eq!(Icon::for_path("/home/user/project", true), Icon::Folder);
    assert_eq!(Icon::for_path("main.go", false), Icon::Go);
}
