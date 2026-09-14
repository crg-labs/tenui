use std::collections::{HashMap, HashSet};

use tenui_core::{CanvasSubviewMut, Color, Modifier};

/// Semantic token types emitted by the syntax lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TokenKind {
    Keyword,
    Type,
    StringLiteral,
    Comment,
    Number,
    Function,
    Operator,
    Punctuation,
    PlainText,
}

/// Programming languages supported by the built-in streaming lexer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Language {
    Rust,
    Python,
    Javascript,
    Json,
    Shell,
    Markdown,
    Tex,
    Toml,
    Yaml,
    C,
    Go,
    Sql,
    Html,
    Css,
    #[default]
    PlainText,
}

impl Language {
    pub fn from_extension(ext: &str) -> Self {
        match ext.to_lowercase().as_str() {
            "rs" | "rust" => Language::Rust,
            "py" | "python" => Language::Python,
            "js" | "javascript" | "ts" | "typescript" | "jsx" | "tsx" => Language::Javascript,
            "json" => Language::Json,
            "sh" | "bash" | "zsh" | "shell" => Language::Shell,
            "md" | "markdown" => Language::Markdown,
            "tex" | "latex" | "sty" | "cls" | "dtx" => Language::Tex,
            "toml" => Language::Toml,
            "yaml" | "yml" => Language::Yaml,
            "c" | "h" | "cc" | "cpp" | "cxx" | "hpp" => Language::C,
            "go" | "golang" => Language::Go,
            "sql" => Language::Sql,
            "html" | "htm" | "xhtml" => Language::Html,
            "css" | "scss" | "sass" | "less" => Language::Css,
            _ => Language::PlainText,
        }
    }

    /// Returns the canonical name of the language for display (e.g. in a `CodeView` label badge).
    pub fn as_str(&self) -> &'static str {
        match self {
            Language::Rust => "rust",
            Language::Python => "python",
            Language::Javascript => "javascript",
            Language::Json => "json",
            Language::Shell => "shell",
            Language::Markdown => "markdown",
            Language::Tex => "tex",
            Language::Toml => "toml",
            Language::Yaml => "yaml",
            Language::C => "c",
            Language::Go => "go",
            Language::Sql => "sql",
            Language::Html => "html",
            Language::Css => "css",
            Language::PlainText => "text",
        }
    }
}

/// Syntax color and style configuration.
#[derive(Debug, Clone)]
pub struct SyntaxTheme {
    pub keyword: (Color, Modifier),
    pub r#type: (Color, Modifier),
    pub string: (Color, Modifier),
    pub comment: (Color, Modifier),
    pub number: (Color, Modifier),
    pub function: (Color, Modifier),
    pub operator: (Color, Modifier),
    pub punctuation: (Color, Modifier),
    pub plain: (Color, Modifier),
    pub gutter_bg: Color,
    pub gutter_fg: Color,
    pub blame_fg: Color,
    pub indent_guide_fg: Color,
    pub fold_icon_fg: Color,
}

impl Default for SyntaxTheme {
    fn default() -> Self {
        Self::dark()
    }
}

impl SyntaxTheme {
    pub fn dark() -> Self {
        Self {
            keyword: (Color::Rgb(203, 166, 247), Modifier::BOLD),        // Mauve
            r#type: (Color::Rgb(249, 226, 175), Modifier::empty()),      // Yellow
            string: (Color::Rgb(166, 227, 161), Modifier::empty()),      // Green
            comment: (Color::Rgb(108, 112, 134), Modifier::ITALIC),      // Overlay0
            number: (Color::Rgb(250, 179, 135), Modifier::empty()),      // Peach
            function: (Color::Rgb(137, 180, 250), Modifier::empty()),    // Blue
            operator: (Color::Rgb(148, 226, 213), Modifier::empty()),    // Teal
            punctuation: (Color::Rgb(147, 153, 178), Modifier::empty()), // Subtext0
            plain: (Color::Rgb(205, 214, 244), Modifier::empty()),       // Text
            gutter_bg: Color::Rgb(24, 24, 37),                           // Mantle
            gutter_fg: Color::Rgb(108, 112, 134),                        // Overlay0
            blame_fg: Color::Rgb(88, 91, 112),                           // Surface2
            indent_guide_fg: Color::Rgb(49, 50, 68),                     // Surface0
            fold_icon_fg: Color::Rgb(180, 190, 254),                     // Lavender
        }
    }

    pub fn light() -> Self {
        Self {
            keyword: (Color::Rgb(136, 57, 239), Modifier::BOLD),
            r#type: (Color::Rgb(223, 142, 29), Modifier::empty()),
            string: (Color::Rgb(64, 160, 43), Modifier::empty()),
            comment: (Color::Rgb(156, 160, 176), Modifier::ITALIC),
            number: (Color::Rgb(254, 100, 11), Modifier::empty()),
            function: (Color::Rgb(30, 102, 245), Modifier::empty()),
            operator: (Color::Rgb(23, 146, 153), Modifier::empty()),
            punctuation: (Color::Rgb(124, 127, 147), Modifier::empty()),
            plain: (Color::Rgb(76, 79, 105), Modifier::empty()),
            gutter_bg: Color::Rgb(230, 233, 239),
            gutter_fg: Color::Rgb(156, 160, 176),
            blame_fg: Color::Rgb(172, 176, 190),
            indent_guide_fg: Color::Rgb(204, 208, 218),
            fold_icon_fg: Color::Rgb(114, 135, 253),
        }
    }

    pub fn style_for(&self, kind: TokenKind) -> (Color, Modifier) {
        match kind {
            TokenKind::Keyword => self.keyword,
            TokenKind::Type => self.r#type,
            TokenKind::StringLiteral => self.string,
            TokenKind::Comment => self.comment,
            TokenKind::Number => self.number,
            TokenKind::Function => self.function,
            TokenKind::Operator => self.operator,
            TokenKind::Punctuation => self.punctuation,
            TokenKind::PlainText => self.plain,
        }
    }

    /// Derives a `SyntaxTheme` from a [`crate::theme::ThemePalette`] so fenced code blocks always match the
    /// prose theme. Maps semantic palette roles to token categories.
    pub fn from_palette(p: &crate::theme::ThemePalette) -> Self {
        Self {
            keyword: (p.secondary, Modifier::BOLD),
            r#type: (p.warning, Modifier::empty()),
            string: (p.success, Modifier::empty()),
            comment: (p.fg_muted, Modifier::ITALIC),
            number: (p.accent, Modifier::empty()),
            function: (p.primary, Modifier::empty()),
            operator: (p.accent, Modifier::empty()),
            punctuation: (p.fg_muted, Modifier::empty()),
            plain: (p.fg, Modifier::empty()),
            gutter_bg: p.surface,
            gutter_fg: p.fg_muted,
            blame_fg: p.border,
            indent_guide_fg: p.border,
            fold_icon_fg: p.primary,
        }
    }
}

/// Token span representing `[start_col..end_col]` on a single line.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TokenSpan {
    pub start: usize,
    pub end: usize,
    pub kind: TokenKind,
}

/// Zero-allocation streaming line tokenizer.
pub struct LineTokenizer;

impl LineTokenizer {
    /// Tokenizes a single line of text for the given language into an iterator/visitor.
    pub fn tokenize_line<F>(lang: Language, line: &str, mut callback: F)
    where
        F: FnMut(TokenSpan),
    {
        if line.is_empty() {
            return;
        }

        match lang {
            Language::Rust => Self::tokenize_rust(line, &mut callback),
            Language::Python => Self::tokenize_python(line, &mut callback),
            Language::Javascript => Self::tokenize_js(line, &mut callback),
            Language::Json => Self::tokenize_json(line, &mut callback),
            Language::Shell => Self::tokenize_shell(line, &mut callback),
            Language::Markdown => Self::tokenize_markdown(line, &mut callback),
            Language::Tex => Self::tokenize_tex(line, &mut callback),
            Language::Toml => Self::tokenize_toml(line, &mut callback),
            Language::Yaml => Self::tokenize_yaml(line, &mut callback),
            Language::C => Self::tokenize_c(line, &mut callback),
            Language::Go => Self::tokenize_go(line, &mut callback),
            Language::Sql => Self::tokenize_sql(line, &mut callback),
            Language::Html => Self::tokenize_html(line, &mut callback),
            Language::Css => Self::tokenize_css(line, &mut callback),
            Language::PlainText => {
                callback(TokenSpan {
                    start: 0,
                    end: line.len(),
                    kind: TokenKind::PlainText,
                });
            }
        }
    }

    fn tokenize_rust<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        // Arrays must be sorted for binary_search.
        const RUST_KEYWORDS: &[&str] = &[
            "Self", "as", "async", "await", "break", "const", "continue", "crate", "dyn", "else", "enum", "extern",
            "false", "fn", "for", "if", "impl", "in", "let", "loop", "match", "mod", "move", "mut", "pub", "ref",
            "return", "self", "static", "struct", "super", "trait", "true", "type", "unsafe", "use", "where", "while",
        ];
        const RUST_TYPES: &[&str] = &[
            "Arc", "Box", "Err", "None", "Ok", "Option", "Rc", "Result", "Self", "Some", "String", "Vec", "bool",
            "char", "f32", "f64", "i128", "i16", "i32", "i64", "i8", "isize", "str", "u128", "u16", "u32", "u64", "u8",
            "usize",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // Whitespace
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Line comment
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // String or Char literal
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Numbers
            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers or keywords
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if RUST_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if RUST_TYPES.binary_search(&word).is_ok() || word.starts_with(char::is_uppercase) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            // Operators & Punctuation
            let start = i;
            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'&' | b'|' | b'^' | b'!' => {
                    TokenKind::Operator
                }
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b':' | b',' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            i += char_width(b);
            emit(TokenSpan { start, end: i, kind });
        }
    }

    fn tokenize_python<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        const PY_KEYWORDS: &[&str] = &[
            "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "False", "None",
            "True", "and", "as", "assert", "async", "await", "break", "class", "continue", "def", "del", "elif",
            "else", "except", "finally", "for", "from", "global", "if", "import", "in", "is", "lambda", "nonlocal",
            "not", "or", "pass", "raise", "return", "try", "while", "with", "yield",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            if b == b'#' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.' || bytes[i] == b'_') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if PY_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if word.starts_with(char::is_uppercase) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let start = i;
            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'&' | b'|' | b'^' | b'~' => {
                    TokenKind::Operator
                }
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b':' | b',' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            i += char_width(b);
            emit(TokenSpan { start, end: i, kind });
        }
    }

    fn tokenize_js<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        const JS_KEYWORDS: &[&str] = &[
            "async",
            "await",
            "break",
            "case",
            "catch",
            "class",
            "const",
            "continue",
            "debugger",
            "default",
            "delete",
            "do",
            "else",
            "export",
            "extends",
            "false",
            "finally",
            "for",
            "function",
            "if",
            "import",
            "in",
            "instanceof",
            "let",
            "new",
            "null",
            "return",
            "static",
            "super",
            "switch",
            "this",
            "throw",
            "true",
            "try",
            "typeof",
            "undefined",
            "var",
            "void",
            "while",
            "with",
            "yield",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            if b == b'"' || b == b'\'' || b == b'`' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'.') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            if b.is_ascii_alphabetic() || b == b'_' || b == b'$' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'$') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if JS_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if word.starts_with(char::is_uppercase) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let start = i;
            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'&' | b'|' | b'^' | b'!' | b'?' => {
                    TokenKind::Operator
                }
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b':' | b',' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            i += char_width(b);
            emit(TokenSpan { start, end: i, kind });
        }
    }

    fn tokenize_json<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            if b == b'"' {
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == b'"' {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                // Check if followed by colon -> object key (treat as keyword/type)
                let mut j = i;
                while j < len && bytes[j].is_ascii_whitespace() {
                    j += 1;
                }
                let kind = if j < len && bytes[j] == b':' {
                    TokenKind::Keyword
                } else {
                    TokenKind::StringLiteral
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            if b.is_ascii_digit() || b == b'-' {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_digit()
                        || bytes[i] == b'.'
                        || bytes[i] == b'e'
                        || bytes[i] == b'E'
                        || bytes[i] == b'+'
                        || bytes[i] == b'-')
                {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            if b.is_ascii_alphabetic() {
                let start = i;
                while i < len && bytes[i].is_ascii_alphabetic() {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = match word {
                    "true" | "false" | "null" => TokenKind::Keyword,
                    _ => TokenKind::PlainText,
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let start = i;
            let kind = match b {
                b'{' | b'}' | b'[' | b']' | b':' | b',' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            i += char_width(b);
            emit(TokenSpan { start, end: i, kind });
        }
    }

    fn tokenize_shell<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        const SH_KEYWORDS: &[&str] = &[
            "case", "do", "done", "elif", "else", "esac", "fi", "for", "function", "if", "in", "select", "then",
            "time", "until", "while",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            if b == b'#' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                while i < len && bytes[i] != quote {
                    if bytes[i] == b'\\' && i + 1 < len {
                        i += 1;
                    }
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            if b == b'$' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Type,
                });
                continue;
            }

            if b.is_ascii_alphanumeric() || b == b'_' || b == b'-' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if SH_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if word.chars().all(|c| c.is_ascii_digit()) {
                    TokenKind::Number
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let start = i;
            let kind = match b {
                b'|' | b'&' | b';' | b'<' | b'>' | b'=' => TokenKind::Operator,
                _ => TokenKind::Punctuation,
            };
            i += char_width(b);
            emit(TokenSpan { start, end: i, kind });
        }
    }

    fn tokenize_markdown<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        if line.starts_with('#') {
            emit(TokenSpan {
                start: 0,
                end: line.len(),
                kind: TokenKind::Keyword,
            });
            return;
        }

        if line.starts_with("```") {
            emit(TokenSpan {
                start: 0,
                end: line.len(),
                kind: TokenKind::Comment,
            });
            return;
        }

        emit(TokenSpan {
            start: 0,
            end: line.len(),
            kind: TokenKind::PlainText,
        });
    }

    fn tokenize_tex<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        const TEX_KEYWORDS: &[&str] = &[
            "\\begin",
            "\\end",
            "\\documentclass",
            "\\usepackage",
            "\\input",
            "\\include",
            "\\def",
            "\\newcommand",
            "\\renewcommand",
            "\\newenvironment",
            "\\renewenvironment",
            "\\let",
            "\\setlength",
            "\\settowidth",
            "\\part",
            "\\part*",
            "\\chapter",
            "\\chapter*",
            "\\section",
            "\\section*",
            "\\subsection",
            "\\subsection*",
            "\\subsubsection",
            "\\subsubsection*",
            "\\paragraph",
            "\\subparagraph",
            "\\item",
            "\\caption",
            "\\label",
            "\\ref",
            "\\eqref",
            "\\cite",
            "\\pageref",
            "\\footnote",
            "\\bibliography",
            "\\bibliographystyle",
            "\\tableofcontents",
            "\\geometry",
            "\\title",
            "\\author",
            "\\date",
            "\\maketitle",
        ];

        const TEX_TYPES: &[&str] = &[
            "document",
            "article",
            "report",
            "book",
            "letter",
            "equation",
            "equation*",
            "align",
            "align*",
            "gather",
            "gather*",
            "figure",
            "table",
            "itemize",
            "enumerate",
            "description",
            "tabular",
            "array",
            "matrix",
            "pmatrix",
            "bmatrix",
            "vmatrix",
            "cases",
            "center",
            "flushleft",
            "flushright",
            "minipage",
            "proof",
            "theorem",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // 1. Whitespace
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // 2. Comments (% to EOL; note escaped \\% is handled by the backslash branch below)
            if b == b'%' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // 3. Control sequences and escaped symbols (starting with '\\')
            if b == b'\\' {
                let start = i;
                i += 1;
                if i >= len {
                    emit(TokenSpan {
                        start,
                        end: len,
                        kind: TokenKind::Punctuation,
                    });
                    break;
                }

                if bytes[i].is_ascii_alphabetic() || bytes[i] == b'@' {
                    while i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'@') {
                        i += 1;
                    }
                    if i < len && bytes[i] == b'*' {
                        i += 1;
                    }
                    let word = &line[start..i];
                    let kind = if TEX_KEYWORDS.contains(&word) {
                        TokenKind::Keyword
                    } else {
                        TokenKind::Function
                    };
                    emit(TokenSpan { start, end: i, kind });
                    continue;
                } else {
                    let esc = bytes[i];
                    i += 1;
                    let kind = match esc {
                        b'[' | b']' | b'(' | b')' => TokenKind::Operator,
                        b'\\' => TokenKind::Keyword,
                        _ => TokenKind::Punctuation,
                    };
                    emit(TokenSpan { start, end: i, kind });
                    continue;
                }
            }

            // 4. Math delimiters ($ or $$)
            if b == b'$' {
                let start = i;
                i += 1;
                if i < len && bytes[i] == b'$' {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Operator,
                });
                continue;
            }

            // 5. Delimiters and Groups ({}, [], ())
            if matches!(b, b'{' | b'}' | b'[' | b']' | b'(' | b')') {
                emit(TokenSpan {
                    start: i,
                    end: i + 1,
                    kind: TokenKind::Punctuation,
                });
                i += 1;
                continue;
            }

            // 6. Special operators (&, ^, _, ~, #, =, +, -, *, /, <, >)
            if matches!(
                b,
                b'&' | b'^' | b'_' | b'~' | b'#' | b'=' | b'+' | b'-' | b'*' | b'/' | b'<' | b'>'
            ) {
                emit(TokenSpan {
                    start: i,
                    end: i + 1,
                    kind: TokenKind::Operator,
                });
                i += 1;
                continue;
            }

            // 7. Numbers and dimension units (e.g. 12, 3.14, 12pt, 1.5em)
            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                // `get` (not a raw slice) so a following multi-byte char can't
                // make `i + 2` land mid-character and panic.
                if let Some(unit) = line.get(i..i + 2)
                    && matches!(
                        unit.to_ascii_lowercase().as_str(),
                        "pt" | "mm" | "cm" | "in" | "em" | "ex" | "pc" | "bp" | "dd" | "cc" | "sp"
                    )
                {
                    i += 2;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // 8. Multi-byte UTF-8 character handling
            if !b.is_ascii() {
                let start = i;
                if let Some(ch) = line[i..].chars().next() {
                    i += ch.len_utf8();
                } else {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // 9. Words / Identifiers (check if known environment / type name)
            if b.is_ascii_alphabetic() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'*') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if TEX_TYPES.contains(&word) {
                    TokenKind::Type
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            // 10. Standard punctuation (, ; : .)
            if matches!(b, b',' | b';' | b':' | b'.') {
                emit(TokenSpan {
                    start: i,
                    end: i + 1,
                    kind: TokenKind::Punctuation,
                });
                i += 1;
                continue;
            }

            // Fallback plain character
            emit(TokenSpan {
                start: i,
                end: i + 1,
                kind: TokenKind::PlainText,
            });
            i += 1;
        }
    }

    fn tokenize_toml<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // Whitespace
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Comment
            if b == b'#' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // String
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                // triple-quote
                let triple = i + 1 < len && bytes[i] == quote && bytes[i + 1] == quote;
                if triple {
                    i += 2;
                }
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' && !triple {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        if triple && i + 1 < len && bytes[i] == quote && bytes[i + 1] == quote {
                            i += 2;
                        }
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Section headers [section] or [[array]]
            if b == b'[' {
                let start = i;
                while i < len && bytes[i] != b']' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                // closing ]] for array tables
                if i < len && bytes[i] == b']' {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Keyword,
                });
                continue;
            }

            // Number
            if b.is_ascii_digit() || (b == b'-' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
                let start = i;
                i += 1;
                while i < len
                    && (bytes[i].is_ascii_alphanumeric()
                        || matches!(bytes[i], b'.' | b'_' | b'-' | b'+' | b':' | b'T' | b'Z'))
                {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Key or keyword (true/false)
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_' || bytes[i] == b'-') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = match word {
                    "true" | "false" | "inf" | "nan" => TokenKind::Keyword,
                    _ => TokenKind::Type, // keys are highlighted as types
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let kind = match b {
                b'=' => TokenKind::Operator,
                b',' | b'.' | b'{' | b'}' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_yaml<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            // Whitespace
            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Comment
            if b == b'#' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // Document markers --- / ...
            if b == b'-' || b == b'.' {
                if line[i..].starts_with("---") {
                    emit(TokenSpan {
                        start: i,
                        end: i + 3,
                        kind: TokenKind::Keyword,
                    });
                    i += 3;
                    continue;
                }
                if line[i..].starts_with("...") {
                    emit(TokenSpan {
                        start: i,
                        end: i + 3,
                        kind: TokenKind::Keyword,
                    });
                    i += 3;
                    continue;
                }
                // List item marker
                if b == b'-' && i + 1 < len && bytes[i + 1].is_ascii_whitespace() {
                    emit(TokenSpan {
                        start: i,
                        end: i + 1,
                        kind: TokenKind::Operator,
                    });
                    i += 1;
                    continue;
                }
            }

            // String
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                while i < len && bytes[i] != quote {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Anchor & alias
            if b == b'&' || b == b'*' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Type,
                });
                continue;
            }

            // Tag !tag
            if b == b'!' {
                let start = i;
                i += 1;
                while i < len && !bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Function,
                });
                continue;
            }

            // Key: detection — read until ':' or whitespace
            if b.is_ascii_alphanumeric() || b == b'_' {
                let start = i;
                while i < len && !matches!(bytes[i], b':' | b'\n') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if i < len && bytes[i] == b':' {
                    TokenKind::Keyword // YAML key
                } else {
                    match word.trim() {
                        "true" | "false" | "yes" | "no" | "null" | "~" => TokenKind::Keyword,
                        _ if word.trim().parse::<f64>().is_ok() => TokenKind::Number,
                        _ => TokenKind::PlainText,
                    }
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let kind = match b {
                b':' => TokenKind::Operator,
                b'[' | b']' | b'{' | b'}' | b',' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_c<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        // C/C++ shares ~90% of Rust's tokenizer — same comment, string, number,
        // identifier, operator/punctuation structure. Only keywords differ.
        const C_KEYWORDS: &[&str] = &[
            "auto",
            "break",
            "case",
            "char",
            "const",
            "continue",
            "default",
            "do",
            "double",
            "else",
            "enum",
            "extern",
            "float",
            "for",
            "goto",
            "if",
            "inline",
            "int",
            "long",
            "register",
            "restrict",
            "return",
            "short",
            "signed",
            "sizeof",
            "static",
            "struct",
            "switch",
            "typedef",
            "union",
            "unsigned",
            "void",
            "volatile",
            "while",
            // C++ extras
            "bool",
            "catch",
            "class",
            "constexpr",
            "delete",
            "explicit",
            "export",
            "false",
            "friend",
            "mutable",
            "namespace",
            "new",
            "nullptr",
            "operator",
            "private",
            "protected",
            "public",
            "template",
            "this",
            "throw",
            "true",
            "try",
            "typename",
            "using",
            "virtual",
            "override",
            "final",
            "auto",
        ];
        const C_TYPES: &[&str] = &[
            "int8_t",
            "int16_t",
            "int32_t",
            "int64_t",
            "uint8_t",
            "uint16_t",
            "uint32_t",
            "uint64_t",
            "size_t",
            "ptrdiff_t",
            "intptr_t",
            "uintptr_t",
            "FILE",
            "NULL",
            "EOF",
            "stdin",
            "stdout",
            "stderr",
            "string",
            "vector",
            "map",
            "set",
            "pair",
            "unique_ptr",
            "shared_ptr",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Line comment //
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // Preprocessor directive
            if b == b'#' {
                let start = i;
                i += 1;
                while i < len && bytes[i].is_ascii_alphanumeric() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Function,
                });
                continue;
            }

            // String or char literal
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Numbers (including hex 0x, octal 0, float)
            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_' | b'x' | b'X')) {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers, keywords, types
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if C_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if C_TYPES.binary_search(&word).is_ok() || word.starts_with(char::is_uppercase) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'&' | b'|' | b'^' | b'!' | b'~' | b'?' => {
                    TokenKind::Operator
                }
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b':' | b',' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_go<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        const GO_KEYWORDS: &[&str] = &[
            "break",
            "case",
            "chan",
            "const",
            "continue",
            "default",
            "defer",
            "else",
            "fallthrough",
            "for",
            "func",
            "go",
            "goto",
            "if",
            "import",
            "interface",
            "map",
            "package",
            "range",
            "return",
            "select",
            "struct",
            "switch",
            "type",
            "var",
        ];
        const GO_TYPES: &[&str] = &[
            "any",
            "bool",
            "byte",
            "comparable",
            "complex128",
            "complex64",
            "error",
            "float32",
            "float64",
            "int",
            "int16",
            "int32",
            "int64",
            "int8",
            "nil",
            "rune",
            "string",
            "uint",
            "uint16",
            "uint32",
            "uint64",
            "uint8",
            "uintptr",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Line comment //
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'/' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // Raw string literal `...`
            if b == b'`' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b'`' {
                    i += 1;
                }
                if i < len {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Standard string or char literal
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Numbers
            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'.' | b'_')) {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers, keywords, types
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if GO_KEYWORDS.binary_search(&word).is_ok() {
                    TokenKind::Keyword
                } else if GO_TYPES.binary_search(&word).is_ok() || word.starts_with(char::is_uppercase) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            // Short declaration := or channel <-
            if (b == b':' && i + 1 < len && bytes[i + 1] == b'=') || (b == b'<' && i + 1 < len && bytes[i + 1] == b'-')
            {
                emit(TokenSpan {
                    start: i,
                    end: i + 2,
                    kind: TokenKind::Operator,
                });
                i += 2;
                continue;
            }

            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'&' | b'|' | b'^' | b'!' | b'~' => {
                    TokenKind::Operator
                }
                b'{' | b'}' | b'[' | b']' | b'(' | b')' | b';' | b':' | b',' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_sql<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        // SQL keywords are case-insensitive, so we match with
        // `eq_ignore_ascii_case` rather than the sorted binary_search the
        // case-sensitive languages above use.
        const SQL_KEYWORDS: &[&str] = &[
            "ADD",
            "ALL",
            "ALTER",
            "AND",
            "AS",
            "ASC",
            "BEGIN",
            "BETWEEN",
            "BY",
            "CASCADE",
            "CASE",
            "CHECK",
            "COMMIT",
            "CONSTRAINT",
            "CREATE",
            "CROSS",
            "DEFAULT",
            "DELETE",
            "DESC",
            "DISTINCT",
            "DROP",
            "ELSE",
            "END",
            "EXISTS",
            "FOREIGN",
            "FROM",
            "FULL",
            "GRANT",
            "GROUP",
            "HAVING",
            "IN",
            "INDEX",
            "INNER",
            "INSERT",
            "INTO",
            "IS",
            "JOIN",
            "KEY",
            "LEFT",
            "LIKE",
            "LIMIT",
            "NOT",
            "NULL",
            "OFFSET",
            "ON",
            "OR",
            "ORDER",
            "OUTER",
            "PRIMARY",
            "REFERENCES",
            "REVOKE",
            "RIGHT",
            "ROLLBACK",
            "SELECT",
            "SET",
            "TABLE",
            "THEN",
            "TRANSACTION",
            "TRIGGER",
            "UNION",
            "UNIQUE",
            "UPDATE",
            "USING",
            "VALUES",
            "VIEW",
            "WHEN",
            "WHERE",
            "WITH",
        ];
        const SQL_TYPES: &[&str] = &[
            "BIGINT",
            "BLOB",
            "BOOLEAN",
            "CHAR",
            "DATE",
            "DATETIME",
            "DECIMAL",
            "DOUBLE",
            "FLOAT",
            "INT",
            "INTEGER",
            "JSON",
            "JSONB",
            "NUMERIC",
            "REAL",
            "SERIAL",
            "SMALLINT",
            "TEXT",
            "TIME",
            "TIMESTAMP",
            "UUID",
            "VARCHAR",
        ];

        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Line comment --
            if b == b'-' && i + 1 < len && bytes[i + 1] == b'-' {
                emit(TokenSpan {
                    start: i,
                    end: len,
                    kind: TokenKind::Comment,
                });
                break;
            }

            // Block comment /* ... */ (unterminated runs to end of line)
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Comment,
                });
                continue;
            }

            // String literal '...' (doubled '' escapes) or quoted identifier "..."
            if b == b'\'' || b == b'"' {
                let quote = b;
                let start = i;
                i += 1;
                while i < len {
                    if bytes[i] == quote {
                        if i + 1 < len && bytes[i + 1] == quote {
                            i += 2;
                            continue;
                        }
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // Numbers
            if b.is_ascii_digit() {
                let start = i;
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers, keywords, types
            if b.is_ascii_alphabetic() || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start..i];
                let kind = if SQL_KEYWORDS.iter().any(|kw| kw.eq_ignore_ascii_case(word)) {
                    TokenKind::Keyword
                } else if SQL_TYPES.iter().any(|t| t.eq_ignore_ascii_case(word)) {
                    TokenKind::Type
                } else if i < len && bytes[i] == b'(' {
                    TokenKind::Function
                } else {
                    TokenKind::PlainText
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let kind = match b {
                b'+' | b'-' | b'*' | b'/' | b'%' | b'=' | b'<' | b'>' | b'!' | b'|' => TokenKind::Operator,
                b'(' | b')' | b',' | b';' | b'.' => TokenKind::Punctuation,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_css<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;

        while i < len {
            let b = bytes[i];

            if b.is_ascii_whitespace() {
                let start = i;
                while i < len && bytes[i].is_ascii_whitespace() {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::PlainText,
                });
                continue;
            }

            // Block comment /* ... */ (unterminated runs to end of line)
            if b == b'/' && i + 1 < len && bytes[i + 1] == b'*' {
                let start = i;
                i += 2;
                while i + 1 < len && !(bytes[i] == b'*' && bytes[i + 1] == b'/') {
                    i += 1;
                }
                i = (i + 2).min(len);
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Comment,
                });
                continue;
            }

            // String literal
            if b == b'"' || b == b'\'' {
                let quote = b;
                let start = i;
                i += 1;
                let mut escaped = false;
                while i < len {
                    if escaped {
                        escaped = false;
                    } else if bytes[i] == b'\\' {
                        escaped = true;
                    } else if bytes[i] == quote {
                        i += 1;
                        break;
                    }
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::StringLiteral,
                });
                continue;
            }

            // At-rule: @media, @import, @keyframes
            if b == b'@' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Keyword,
                });
                continue;
            }

            // Hex color (#fff / #ffffff) or id selector (#name)
            if b == b'#' {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                let word = &line[start + 1..i];
                let is_hex_color = matches!(word.len(), 3 | 4 | 6 | 8) && word.bytes().all(|c| c.is_ascii_hexdigit());
                let kind = if is_hex_color {
                    TokenKind::Number
                } else {
                    TokenKind::Type
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            // Class selector .name (but .5em is a number, handled below)
            if b == b'.'
                && i + 1 < len
                && (bytes[i + 1].is_ascii_alphabetic() || bytes[i + 1] == b'-' || bytes[i + 1] == b'_')
            {
                let start = i;
                i += 1;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Type,
                });
                continue;
            }

            // Numbers with optional unit: 12px, 1.5em, .5rem, 100%
            if b.is_ascii_digit() || (b == b'.' && i + 1 < len && bytes[i + 1].is_ascii_digit()) {
                let start = i;
                while i < len && (bytes[i].is_ascii_digit() || bytes[i] == b'.') {
                    i += 1;
                }
                while i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'%') {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Number,
                });
                continue;
            }

            // Identifiers: property names, values, element selectors, functions
            if b.is_ascii_alphabetic() || b == b'-' || b == b'_' {
                let start = i;
                while i < len && (bytes[i].is_ascii_alphanumeric() || bytes[i] == b'-' || bytes[i] == b'_') {
                    i += 1;
                }
                let kind = if i < len && bytes[i] == b'(' {
                    TokenKind::Function // rgb(, url(, calc(
                } else if i < len && bytes[i] == b':' {
                    TokenKind::Keyword // property name: color:, margin:
                } else {
                    TokenKind::Type // element selector or value keyword
                };
                emit(TokenSpan { start, end: i, kind });
                continue;
            }

            let kind = match b {
                b'{' | b'}' | b'(' | b')' | b';' | b':' | b',' => TokenKind::Punctuation,
                b'>' | b'+' | b'~' | b'*' | b'=' | b'!' => TokenKind::Operator,
                _ => TokenKind::PlainText,
            };
            let w = char_width(bytes[i]);
            emit(TokenSpan {
                start: i,
                end: i + w,
                kind,
            });
            i += w;
        }
    }

    fn tokenize_html<F: FnMut(TokenSpan)>(line: &str, emit: &mut F) {
        let bytes = line.as_bytes();
        let len = bytes.len();
        let mut i = 0;
        // Tags don't carry across lines: like the block-comment handling in the
        // other tokenizers, an unterminated tag simply ends with the line.
        let mut in_tag = false;

        while i < len {
            let b = bytes[i];

            // Comment <!-- ... --> (unterminated runs to end of line)
            if !in_tag && b == b'<' && line[i..].starts_with("<!--") {
                let start = i;
                i += 4;
                while i + 2 < len && !(bytes[i] == b'-' && bytes[i + 1] == b'-' && bytes[i + 2] == b'>') {
                    i += 1;
                }
                i = if i + 2 < len { i + 3 } else { len };
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Comment,
                });
                continue;
            }

            // Tag open: `<` or `</` followed by a tag name
            if !in_tag && b == b'<' {
                let start = i;
                i += 1;
                if i < len && bytes[i] == b'/' {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Punctuation,
                });
                if i < len && (bytes[i].is_ascii_alphabetic() || bytes[i] == b'!') {
                    let ns = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'-' | b'_' | b'!' | b':'))
                    {
                        i += 1;
                    }
                    emit(TokenSpan {
                        start: ns,
                        end: i,
                        kind: TokenKind::Keyword,
                    });
                }
                in_tag = true;
                continue;
            }

            if in_tag {
                if b.is_ascii_whitespace() {
                    let start = i;
                    while i < len && bytes[i].is_ascii_whitespace() {
                        i += 1;
                    }
                    emit(TokenSpan {
                        start,
                        end: i,
                        kind: TokenKind::PlainText,
                    });
                    continue;
                }
                if b == b'>' {
                    emit(TokenSpan {
                        start: i,
                        end: i + 1,
                        kind: TokenKind::Punctuation,
                    });
                    i += 1;
                    in_tag = false;
                    continue;
                }
                if b == b'/' {
                    emit(TokenSpan {
                        start: i,
                        end: i + 1,
                        kind: TokenKind::Punctuation,
                    });
                    i += 1;
                    continue;
                }
                if b == b'=' {
                    emit(TokenSpan {
                        start: i,
                        end: i + 1,
                        kind: TokenKind::Operator,
                    });
                    i += 1;
                    continue;
                }
                // Attribute value
                if b == b'"' || b == b'\'' {
                    let quote = b;
                    let start = i;
                    i += 1;
                    while i < len && bytes[i] != quote {
                        i += 1;
                    }
                    if i < len {
                        i += 1;
                    }
                    emit(TokenSpan {
                        start,
                        end: i,
                        kind: TokenKind::StringLiteral,
                    });
                    continue;
                }
                // Attribute name
                if b.is_ascii_alphabetic() || b == b'_' {
                    let start = i;
                    while i < len && (bytes[i].is_ascii_alphanumeric() || matches!(bytes[i], b'-' | b'_' | b':')) {
                        i += 1;
                    }
                    emit(TokenSpan {
                        start,
                        end: i,
                        kind: TokenKind::Type,
                    });
                    continue;
                }
                let w = char_width(bytes[i]);
                emit(TokenSpan {
                    start: i,
                    end: i + w,
                    kind: TokenKind::PlainText,
                });
                i += w;
                continue;
            }

            // Entity: &amp; &#123;
            if b == b'&' {
                let start = i;
                i += 1;
                while i < len && bytes[i] != b';' && !bytes[i].is_ascii_whitespace() && bytes[i] != b'<' {
                    i += 1;
                }
                if i < len && bytes[i] == b';' {
                    i += 1;
                }
                emit(TokenSpan {
                    start,
                    end: i,
                    kind: TokenKind::Type,
                });
                continue;
            }

            // Text content: a run up to the next `<` or `&` keeps UTF-8 boundaries valid.
            let start = i;
            while i < len && bytes[i] != b'<' && bytes[i] != b'&' {
                i += 1;
            }
            emit(TokenSpan {
                start,
                end: i,
                kind: TokenKind::PlainText,
            });
        }
    }
}

/// Byte length of the UTF-8 character whose leading byte is `byte`.
///
/// Tokenizers walk `line.as_bytes()` and must never emit a [`TokenSpan`] that ends
/// mid-character: downstream renderers slice `&line[span.start..span.end]`, which
/// panics on a non-char boundary. Input is always a `&str` (valid UTF-8), so the
/// leading byte fully determines the width.
#[inline]
fn char_width(byte: u8) -> usize {
    match byte {
        0x00..=0x7F => 1,
        0xC0..=0xDF => 2,
        0xE0..=0xEF => 3,
        _ => 4,
    }
}

/// Rich CodeView widget providing zero-allocation streaming viewport tokenization,
/// line numbers, folding markers, Git blame gutter, and indentation guides.
#[derive(Debug, Clone)]
pub struct CodeView<'a> {
    pub text: &'a str,
    pub language: Language,
    pub show_line_numbers: bool,
    pub show_folding: bool,
    pub show_blame: bool,
    pub show_indent_guides: bool,
    pub indent_size: usize,
    pub folded_lines: HashSet<usize>,
    pub blame_info: HashMap<usize, String>,
    pub line_offset: usize,
    pub scroll_x: usize,
    pub theme: SyntaxTheme,
}

impl<'a> CodeView<'a> {
    pub fn new(text: &'a str, language: Language) -> Self {
        Self {
            text,
            language,
            show_line_numbers: true,
            show_folding: true,
            show_blame: false,
            show_indent_guides: true,
            indent_size: 4,
            folded_lines: HashSet::new(),
            blame_info: HashMap::new(),
            line_offset: 0,
            scroll_x: 0,
            theme: SyntaxTheme::default(),
        }
    }

    pub fn with_theme(mut self, theme: SyntaxTheme) -> Self {
        self.theme = theme;
        self
    }

    pub fn with_blame(mut self, blame: HashMap<usize, String>) -> Self {
        self.blame_info = blame;
        self.show_blame = true;
        self
    }

    pub fn toggle_fold(&mut self, line_idx: usize) {
        if self.folded_lines.contains(&line_idx) {
            self.folded_lines.remove(&line_idx);
        } else {
            self.folded_lines.insert(line_idx);
        }
    }

    /// Viewport-bounded direct cell emission into `canvas`.
    pub fn render(&self, canvas: &mut CanvasSubviewMut<'_>) {
        let width = canvas.width() as usize;
        let height = canvas.height() as usize;
        if width == 0 || height == 0 {
            return;
        }

        let lines: Vec<&str> = self.text.lines().collect();
        let total_lines = lines.len();
        let max_line_digits = total_lines.max(1).to_string().len().max(2);

        // Gutter widths:
        // [LineNumber] [FoldIcon] [GitBlame] │ [Code]
        let line_num_w = if self.show_line_numbers { max_line_digits + 1 } else { 0 };
        let fold_w = if self.show_folding { 2 } else { 0 };
        let blame_w = if self.show_blame { 14 } else { 0 };
        let gutter_total_w = line_num_w + fold_w + blame_w;

        for view_y in 0..height {
            let line_idx = self.line_offset + view_y;
            let y = view_y as u16;

            // Clear row background
            for x in 0..canvas.width() {
                canvas.set_char(x, y, ' ', self.theme.plain.0, Color::Reset, Modifier::empty());
            }

            if line_idx >= total_lines {
                // Tilde empty lines like vi
                if self.show_line_numbers && line_num_w > 0 {
                    canvas.set_char(0, y, '~', self.theme.gutter_fg, self.theme.gutter_bg, Modifier::empty());
                }
                continue;
            }

            let line_str = lines[line_idx];
            let mut curr_col = 0;

            // 1. Line numbers
            if self.show_line_numbers {
                let num_str = format!("{:>width$} ", line_idx + 1, width = max_line_digits);
                for ch in num_str.chars() {
                    if curr_col < width {
                        canvas.set_char(
                            curr_col as u16,
                            y,
                            ch,
                            self.theme.gutter_fg,
                            self.theme.gutter_bg,
                            Modifier::empty(),
                        );
                        curr_col += 1;
                    }
                }
            }

            // 2. Folding marker
            if self.show_folding {
                let is_foldable = line_str.trim_end().ends_with('{') || line_str.trim_end().ends_with(':');
                let fold_char = if is_foldable {
                    if self.folded_lines.contains(&line_idx) {
                        '▶'
                    } else {
                        '▼'
                    }
                } else {
                    ' '
                };
                if curr_col < width {
                    canvas.set_char(
                        curr_col as u16,
                        y,
                        fold_char,
                        self.theme.fold_icon_fg,
                        self.theme.gutter_bg,
                        Modifier::empty(),
                    );
                    curr_col += 1;
                }
                if curr_col < width {
                    canvas.set_char(
                        curr_col as u16,
                        y,
                        ' ',
                        self.theme.gutter_fg,
                        self.theme.gutter_bg,
                        Modifier::empty(),
                    );
                    curr_col += 1;
                }
            }

            // 3. Git blame
            if self.show_blame {
                let blame = self.blame_info.get(&line_idx).map(|s| s.as_str()).unwrap_or("");
                let blame_fmt = format!("{:<13}│", &blame[..blame.len().min(13)]);
                for ch in blame_fmt.chars() {
                    if curr_col < width {
                        canvas.set_char(
                            curr_col as u16,
                            y,
                            ch,
                            self.theme.blame_fg,
                            self.theme.gutter_bg,
                            Modifier::empty(),
                        );
                        curr_col += 1;
                    }
                }
            }

            // 4. Code viewport tokenization & direct cell emission
            let code_start_x = gutter_total_w;
            let code_avail_w = width.saturating_sub(code_start_x);
            if code_avail_w == 0 {
                continue;
            }

            // Indentation guide detection on leading whitespace
            let leading_spaces = line_str.chars().take_while(|c| *c == ' ').count();

            LineTokenizer::tokenize_line(self.language, line_str, |token| {
                let (fg, modifier) = self.theme.style_for(token.kind);
                let span_str = &line_str[token.start..token.end];

                for (offset, ch) in span_str.chars().enumerate() {
                    let char_col = token.start + offset;
                    if char_col < self.scroll_x {
                        continue;
                    }
                    let rel_col = char_col - self.scroll_x;
                    if rel_col >= code_avail_w {
                        break;
                    }

                    let dest_x = (code_start_x + rel_col) as u16;

                    // Render indent guide if this is leading whitespace at an indent boundary
                    if self.show_indent_guides
                        && char_col < leading_spaces
                        && char_col > 0
                        && char_col % self.indent_size == 0
                    {
                        canvas.set_char(
                            dest_x,
                            y,
                            '│',
                            self.theme.indent_guide_fg,
                            Color::Reset,
                            Modifier::empty(),
                        );
                    } else {
                        canvas.set_char(dest_x, y, ch, fg, Color::Reset, modifier);
                    }
                }
            });
        }
    }
}

/// Renders a single code line with explicit segregation of line numbers, Git blame gutter, and syntax content.
pub fn render_code_line(
    surface: &mut CanvasSubviewMut<'_>,
    y: u16,
    line_num: usize,
    blame: Option<(&str, &str)>,
    line_content: &str,
    total_lines: usize,
) {
    let num_digits = total_lines.max(1).to_string().len() as u16;

    // 1. Line number at x = 0
    let num_str = format!("{:>width$}", line_num, width = num_digits as usize);
    surface.write_str_clipped(0, y, &num_str, Color::DarkGray, Color::Reset);

    // 2. Git blame starts at x = num_digits + 1
    let blame_width = 16u16;
    if let Some((commit, author)) = blame {
        let commit_sub = &commit[..commit.len().min(7)];
        let author_sub = &author[..author.len().min(7)];
        let blame_str = format!(" {:<7} {:<7}", commit_sub, author_sub);
        surface.write_str_clipped(num_digits, y, &blame_str, Color::DarkGray, Color::Reset);
    }

    // 3. Separator at x = num_digits + blame_width
    let code_offset = num_digits + blame_width + 1;
    surface.write_str_clipped(code_offset.saturating_sub(1), y, "│", Color::DarkGray, Color::Reset);

    // 4. Highlighted tokens start at code_offset
    surface.write_str_clipped(code_offset, y, line_content, Color::White, Color::Reset);
}
