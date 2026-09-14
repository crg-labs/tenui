use tenui_core::{Buffer, CanvasSubviewMut, Color, Modifier, Rect};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::rich::{RichSpan, wrap_rich_spans};
use crate::selection::DocumentSelection;
use crate::syntax::{Language, LineTokenizer, SyntaxTheme};
use crate::text_table::TableAlignment;
use crate::theme::ThemePalette;

use super::parse_inline_markdown;

/// Block elements in a Markdown document.
#[derive(Debug, Clone, PartialEq)]
pub enum MarkdownBlock {
    Header(usize, String),
    Paragraph(String),
    BulletList(usize, String),
    NumberedList(usize, usize, String),
    Blockquote(String),
    CodeBlock(String, String),                                 // language, code
    DisplayMath(String),                                       // LaTeX display math
    Table(Vec<String>, Vec<Vec<String>>, Vec<TableAlignment>), // headers, rows, alignments
    ThematicBreak,
    FootnoteDefinition(String, String), // label, content
}

/// Streaming CommonMark / GFM Terminal Markdown Viewer.
#[derive(Debug, Clone)]
pub struct MarkdownView {
    pub blocks: Vec<MarkdownBlock>,
    pub scroll_y: usize,
    pub theme: SyntaxTheme,
    /// Palette used for inline prose styling (links, bold, italic, code). Defaults to
    /// Catppuccin Mocha, matching this widget's historical colors.
    pub palette: ThemePalette,
    /// Click-and-drag text selection over the rendered document.
    pub selection: DocumentSelection,
    /// Whether to embed OSC 8 terminal hyperlinks when generating text.
    pub osc8_links: bool,
    /// Plain `(x_offset, text)` of each rendered visual row, captured on the last
    /// [`render`](Self::render) so selection can extract and highlight without re-laying-out.
    rendered_rows: Vec<(u16, String)>,
    pub(super) cached_rows: Vec<RenderedRow>,
    pub(super) last_layout_width: usize,
}

impl MarkdownView {
    pub fn new(markdown: &str) -> Self {
        let blocks = Self::parse(markdown);
        Self {
            blocks,
            scroll_y: 0,
            theme: SyntaxTheme::default(),
            palette: ThemePalette::catppuccin_mocha(),
            selection: DocumentSelection::new(),
            osc8_links: false,
            rendered_rows: Vec::new(),
            cached_rows: Vec::new(),
            last_layout_width: 0,
        }
    }

    /// Explicitly invalidates the cached row layout so the next render recalculates everything.
    pub fn invalidate_layout(&mut self) {
        self.cached_rows.clear();
        self.rendered_rows.clear();
        self.last_layout_width = 0;
    }

    /// Sets the palette used for inline prose styling.
    pub fn with_palette(mut self, palette: ThemePalette) -> Self {
        self.palette = palette;
        self.invalidate_layout();
        self
    }

    /// Enables or disables OSC 8 terminal hyperlinks for links.
    pub fn with_osc8_links(mut self, enabled: bool) -> Self {
        self.osc8_links = enabled;
        self
    }

    /// Replaces the markdown content without rebuilding the widget. Resets scroll position and
    /// clears selection; use this for streaming updates instead of destroying and recreating.
    pub fn set_content(&mut self, markdown: &str) {
        self.blocks = Self::parse(markdown);
        self.scroll_y = 0;
        self.selection.clear();
        self.invalidate_layout();
    }

    /// Sets the syntax theme used for fenced code blocks.
    pub fn with_theme(mut self, theme: SyntaxTheme) -> Self {
        self.theme = theme;
        self.invalidate_layout();
        self
    }

    /// Begins a selection drag at canvas cell (`x`, `y`) relative to this view's render area
    /// (mouse press). Pair with [`on_mouse_drag`](Self::on_mouse_drag) and
    /// [`on_mouse_up`](Self::on_mouse_up).
    pub fn on_mouse_down(&mut self, x: u16, y: u16) {
        self.selection.begin(self.scroll_y + y as usize, x);
    }

    /// Extends the in-progress selection to canvas cell (`x`, `y`) (mouse move while pressed).
    pub fn on_mouse_drag(&mut self, x: u16, y: u16) {
        self.selection.extend(self.scroll_y + y as usize, x);
    }

    /// Ends the selection drag (mouse release); the selection stays for copying until cleared.
    pub fn on_mouse_up(&mut self) {
        self.selection.finish();
    }

    /// Clears any active selection.
    pub fn clear_selection(&mut self) {
        self.selection.clear();
    }

    /// The currently selected text (empty when nothing is selected), reflecting the layout of
    /// the most recent [`render`](Self::render).
    pub fn selected_text(&self) -> String {
        self.selection.extract(&self.rendered_rows)
    }

    /// CommonMark / GFM block parser powered by pulldown-cmark.
    pub fn parse(text: &str) -> Vec<MarkdownBlock> {
        let mut options = pulldown_cmark::Options::empty();
        options.insert(pulldown_cmark::Options::ENABLE_TABLES);
        options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
        options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
        options.insert(pulldown_cmark::Options::ENABLE_MATH);
        options.insert(pulldown_cmark::Options::ENABLE_FOOTNOTES);
        let parser = pulldown_cmark::Parser::new_ext(text, options);

        let mut blocks = Vec::new();
        let mut cur_text = String::new();
        let mut in_heading: Option<usize> = None;
        let mut in_code_block = false;
        let mut code_lang = String::new();
        let mut code_content = String::new();
        let mut list_depth = 0usize;
        let mut list_start_num: Option<usize> = None;
        let mut item_count = 1usize;
        let mut in_table = false;
        let mut table_headers: Vec<String> = Vec::new();
        let mut table_rows: Vec<Vec<String>> = Vec::new();
        let mut table_alignments: Vec<TableAlignment> = Vec::new();
        let mut cur_row: Vec<String> = Vec::new();
        let mut cur_cell = String::new();
        let mut in_footnote: Option<String> = None;
        let mut footnote_text = String::new();
        let mut link_url: Option<String> = None;

        for event in parser {
            match event {
                pulldown_cmark::Event::Start(tag) => match tag {
                    pulldown_cmark::Tag::Heading { level, .. } => {
                        let l = match level {
                            pulldown_cmark::HeadingLevel::H1 => 1,
                            pulldown_cmark::HeadingLevel::H2 => 2,
                            pulldown_cmark::HeadingLevel::H3 => 3,
                            pulldown_cmark::HeadingLevel::H4 => 4,
                            pulldown_cmark::HeadingLevel::H5 => 5,
                            pulldown_cmark::HeadingLevel::H6 => 6,
                        };
                        in_heading = Some(l);
                        cur_text.clear();
                    }
                    pulldown_cmark::Tag::BlockQuote(_) => {
                        cur_text.clear();
                    }
                    pulldown_cmark::Tag::CodeBlock(kind) => {
                        in_code_block = true;
                        code_content.clear();
                        code_lang = match kind {
                            pulldown_cmark::CodeBlockKind::Fenced(lang) => lang.to_string(),
                            pulldown_cmark::CodeBlockKind::Indented => String::new(),
                        };
                    }
                    pulldown_cmark::Tag::List(start) => {
                        list_depth = list_depth.saturating_add(1);
                        list_start_num = start.map(|s| s as usize);
                        item_count = list_start_num.unwrap_or(1);
                    }
                    pulldown_cmark::Tag::Item => {
                        cur_text.clear();
                    }
                    pulldown_cmark::Tag::Table(aligns) => {
                        in_table = true;
                        table_headers.clear();
                        table_rows.clear();
                        table_alignments = aligns
                            .iter()
                            .map(|a| match a {
                                pulldown_cmark::Alignment::Center => TableAlignment::Center,
                                pulldown_cmark::Alignment::Right => TableAlignment::Right,
                                _ => TableAlignment::Left,
                            })
                            .collect();
                    }
                    pulldown_cmark::Tag::TableHead => {
                        cur_row.clear();
                    }
                    pulldown_cmark::Tag::TableRow => {
                        cur_row.clear();
                    }
                    pulldown_cmark::Tag::TableCell => {
                        cur_cell.clear();
                    }
                    pulldown_cmark::Tag::Paragraph => {
                        cur_text.clear();
                    }
                    pulldown_cmark::Tag::Link { dest_url, .. } => {
                        link_url = Some(dest_url.to_string());
                        let target = if in_footnote.is_some() {
                            &mut footnote_text
                        } else if in_table {
                            &mut cur_cell
                        } else {
                            &mut cur_text
                        };
                        target.push('[');
                    }
                    pulldown_cmark::Tag::FootnoteDefinition(name) => {
                        in_footnote = Some(name.to_string());
                        footnote_text.clear();
                    }
                    _ => {}
                },
                pulldown_cmark::Event::End(tag_end) => match tag_end {
                    pulldown_cmark::TagEnd::Heading(_) => {
                        if let Some(level) = in_heading.take() {
                            blocks.push(MarkdownBlock::Header(level, cur_text.trim().to_string()));
                            cur_text.clear();
                        }
                    }
                    pulldown_cmark::TagEnd::BlockQuote(_) => {
                        blocks.push(MarkdownBlock::Blockquote(cur_text.trim().to_string()));
                        cur_text.clear();
                    }
                    pulldown_cmark::TagEnd::CodeBlock => {
                        in_code_block = false;
                        if code_lang.trim() == "math" {
                            let trimmed = code_content.trim();
                            if !trimmed.is_empty() {
                                blocks.push(MarkdownBlock::DisplayMath(trimmed.to_string()));
                            }
                        } else {
                            blocks.push(MarkdownBlock::CodeBlock(code_lang.clone(), code_content.clone()));
                        }
                        code_content.clear();
                    }
                    pulldown_cmark::TagEnd::List(_) => {
                        list_depth = list_depth.saturating_sub(1);
                        list_start_num = None;
                    }
                    pulldown_cmark::TagEnd::Item => {
                        let indent = list_depth.saturating_sub(1);
                        let text = cur_text.trim().to_string();
                        if list_start_num.is_some() {
                            blocks.push(MarkdownBlock::NumberedList(indent, item_count, text));
                            item_count += 1;
                        } else {
                            blocks.push(MarkdownBlock::BulletList(indent, text));
                        }
                        cur_text.clear();
                    }
                    pulldown_cmark::TagEnd::TableCell => {
                        cur_row.push(cur_cell.trim().to_string());
                        cur_cell.clear();
                    }
                    pulldown_cmark::TagEnd::TableHead => {
                        table_headers = cur_row.clone();
                    }
                    pulldown_cmark::TagEnd::TableRow => {
                        table_rows.push(cur_row.clone());
                    }
                    pulldown_cmark::TagEnd::Table => {
                        in_table = false;
                        blocks.push(MarkdownBlock::Table(
                            table_headers.clone(),
                            table_rows.clone(),
                            table_alignments.clone(),
                        ));
                    }
                    pulldown_cmark::TagEnd::Paragraph => {
                        let trimmed = cur_text.trim();
                        if !trimmed.is_empty() {
                            blocks.push(MarkdownBlock::Paragraph(trimmed.to_string()));
                        }
                        cur_text.clear();
                    }
                    pulldown_cmark::TagEnd::Link => {
                        if let Some(url) = link_url.take() {
                            let target = if in_footnote.is_some() {
                                &mut footnote_text
                            } else if in_table {
                                &mut cur_cell
                            } else {
                                &mut cur_text
                            };
                            target.push_str(&format!("]({url})"));
                        }
                    }
                    pulldown_cmark::TagEnd::FootnoteDefinition => {
                        if let Some(name) = in_footnote.take() {
                            let trimmed = footnote_text.trim();
                            if !trimmed.is_empty() {
                                blocks.push(MarkdownBlock::FootnoteDefinition(name, trimmed.to_string()));
                            }
                            footnote_text.clear();
                        }
                    }
                    _ => {}
                },
                pulldown_cmark::Event::Text(t) => {
                    if in_code_block {
                        code_content.push_str(&t);
                    } else if in_footnote.is_some() {
                        footnote_text.push_str(&t);
                    } else if in_table {
                        cur_cell.push_str(&t);
                    } else {
                        cur_text.push_str(&t);
                    }
                }
                pulldown_cmark::Event::Code(c) => {
                    let formatted = format!("`{}`", c);
                    if in_code_block {
                        code_content.push_str(&c);
                    } else if in_footnote.is_some() {
                        footnote_text.push_str(&formatted);
                    } else if in_table {
                        cur_cell.push_str(&formatted);
                    } else {
                        cur_text.push_str(&formatted);
                    }
                }
                pulldown_cmark::Event::Rule => {
                    blocks.push(MarkdownBlock::ThematicBreak);
                }
                pulldown_cmark::Event::SoftBreak | pulldown_cmark::Event::HardBreak => {
                    if in_code_block {
                        code_content.push('\n');
                    } else if in_footnote.is_some() {
                        footnote_text.push(' ');
                    } else if in_table {
                        cur_cell.push(' ');
                    } else {
                        cur_text.push(' ');
                    }
                }
                pulldown_cmark::Event::TaskListMarker(checked) => {
                    let marker = if checked { "☑ " } else { "☐ " };
                    if in_footnote.is_some() {
                        footnote_text.push_str(marker);
                    } else if in_table {
                        cur_cell.push_str(marker);
                    } else {
                        cur_text.push_str(marker);
                    }
                }
                pulldown_cmark::Event::InlineMath(m) => {
                    let target = if in_footnote.is_some() {
                        &mut footnote_text
                    } else if in_table {
                        &mut cur_cell
                    } else {
                        &mut cur_text
                    };
                    target.push_str(&format!("${}$", m));
                }
                pulldown_cmark::Event::DisplayMath(m) => {
                    let trimmed = m.trim();
                    if !trimmed.is_empty() {
                        blocks.push(MarkdownBlock::DisplayMath(trimmed.to_string()));
                    }
                }
                pulldown_cmark::Event::FootnoteReference(name) => {
                    let target = if in_footnote.is_some() {
                        &mut footnote_text
                    } else if in_table {
                        &mut cur_cell
                    } else {
                        &mut cur_text
                    };
                    target.push_str(&format!("[^{}]", name));
                }
                pulldown_cmark::Event::Html(t) | pulldown_cmark::Event::InlineHtml(t) => {
                    let target = if in_footnote.is_some() {
                        &mut footnote_text
                    } else if in_table {
                        &mut cur_cell
                    } else {
                        &mut cur_text
                    };
                    target.push_str(&t);
                }
                #[allow(unreachable_patterns)]
                _ => {}
            }
        }

        blocks
    }

    /// Renders markdown blocks into `canvas`, then paints any active text selection and caches
    /// each rendered row so [`selected_text`](Self::selected_text) can extract it.
    pub fn render(&mut self, canvas: &mut CanvasSubviewMut<'_>) {
        let w = canvas.width() as usize;
        let h = canvas.height() as usize;
        if w == 0 || h == 0 {
            return;
        }

        if self.cached_rows.is_empty() || self.last_layout_width != w {
            let mut rendered_rows: Vec<RenderedRow> = Vec::new();

            for block in &self.blocks {
                match block {
                    MarkdownBlock::Header(level, text) => {
                        let (fg, modifier) = match level {
                            1 => (Color::Rgb(243, 139, 168), Modifier::BOLD | Modifier::UNDERLINE),
                            2 => (Color::Rgb(250, 179, 135), Modifier::BOLD),
                            3 => (Color::Rgb(249, 226, 175), Modifier::BOLD),
                            _ => (Color::Rgb(137, 180, 250), Modifier::BOLD),
                        };
                        let prefix = "#".repeat(*level);
                        let line = format!("{} {}", prefix, text);
                        rendered_rows.push(RenderedRow::Text {
                            text: line,
                            fg,
                            bg: Color::Reset,
                            modifier,
                        });
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::Paragraph(text) => {
                        let spans = parse_inline_markdown(text, self.palette.fg, Modifier::empty(), &self.palette);
                        for line in wrap_rich_spans(&spans, w) {
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: String::new(),
                                prefix_fg: self.palette.fg,
                                prefix_mod: Modifier::empty(),
                                spans: line,
                                bg: Color::Reset,
                            });
                        }
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::BulletList(indent, text) => {
                        let is_task = text.starts_with("☑ ") || text.starts_with("☐ ");
                        let prefix = if is_task {
                            format!("{}  ", "  ".repeat(*indent))
                        } else {
                            format!("{}• ", "  ".repeat(*indent))
                        };
                        let inner_w = w.saturating_sub(prefix.chars().count()).max(1);
                        let spans = parse_inline_markdown(text, self.palette.fg, Modifier::empty(), &self.palette);
                        for (n, line) in wrap_rich_spans(&spans, inner_w).into_iter().enumerate() {
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: if n == 0 {
                                    prefix.clone()
                                } else {
                                    " ".repeat(prefix.chars().count())
                                },
                                prefix_fg: self.palette.accent,
                                prefix_mod: Modifier::empty(),
                                spans: line,
                                bg: Color::Reset,
                            });
                        }
                    }
                    MarkdownBlock::NumberedList(indent, num, text) => {
                        let prefix = format!("{}{}. ", "  ".repeat(*indent), num);
                        let inner_w = w.saturating_sub(prefix.chars().count()).max(1);
                        let spans = parse_inline_markdown(text, self.palette.fg, Modifier::empty(), &self.palette);
                        for (n, line) in wrap_rich_spans(&spans, inner_w).into_iter().enumerate() {
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: if n == 0 {
                                    prefix.clone()
                                } else {
                                    " ".repeat(prefix.chars().count())
                                },
                                prefix_fg: self.palette.accent,
                                prefix_mod: Modifier::empty(),
                                spans: line,
                                bg: Color::Reset,
                            });
                        }
                    }
                    MarkdownBlock::Blockquote(text) => {
                        let inner_w = w.saturating_sub(2).max(1);
                        let spans = parse_inline_markdown(text, self.palette.fg_muted, Modifier::ITALIC, &self.palette);
                        for line in wrap_rich_spans(&spans, inner_w) {
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: "│ ".to_string(),
                                prefix_fg: self.palette.fg_muted,
                                prefix_mod: Modifier::empty(),
                                spans: line,
                                bg: Color::Reset,
                            });
                        }
                    }
                    MarkdownBlock::CodeBlock(lang, code) => {
                        let lang_tag = if lang.is_empty() { "code" } else { lang.as_str() };
                        let language = Language::from_extension(lang_tag);
                        rendered_rows.push(RenderedRow::Text {
                            text: format!("┌─ [{}] ──────────────────", lang_tag),
                            fg: Color::DarkGray,
                            bg: Color::Reset,
                            modifier: Modifier::empty(),
                        });
                        for code_line in code.lines() {
                            let mut spans = Vec::new();
                            LineTokenizer::tokenize_line(language, code_line, |span| {
                                let (fg, modifier) = self.theme.style_for(span.kind);
                                let text = &code_line[span.start..span.end];
                                spans.push(RichSpan::new(text, fg, modifier));
                            });
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: "│ ".to_string(),
                                prefix_fg: Color::DarkGray,
                                prefix_mod: Modifier::empty(),
                                spans,
                                bg: self.theme.gutter_bg,
                            });
                        }
                        rendered_rows.push(RenderedRow::Text {
                            text: "└─────────────────────────".to_string(),
                            fg: Color::DarkGray,
                            bg: Color::Reset,
                            modifier: Modifier::empty(),
                        });
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::DisplayMath(math_src) => {
                        let node = tenui_math::MathNode::from_latex(math_src);
                        let typesetter = tenui_math::MathTypesetter;
                        let (mw, mh) = typesetter.measure(&node);

                        if mh > 1 && mw > 0 {
                            let mut math_buf = Buffer::new(mw, mh);
                            {
                                let mut subview = math_buf.subview_mut(Rect::new(0, 0, mw, mh));
                                typesetter.render_themed(&mut subview, 0, 0, &node, self.palette.accent, Color::Reset);
                            }
                            for my in 0..mh {
                                let row_str: String = (0..mw)
                                    .map(|mx| math_buf.get(mx, my).map(|c| c.symbol.as_str()).unwrap_or(" "))
                                    .collect();
                                rendered_rows.push(RenderedRow::Text {
                                    text: format!("    {}", row_str.trim_end()),
                                    fg: self.palette.accent,
                                    bg: Color::Reset,
                                    modifier: Modifier::empty(),
                                });
                            }
                        } else {
                            let unicode = tenui_math::latex_to_unicode(math_src);
                            rendered_rows.push(RenderedRow::Text {
                                text: format!("    {}", unicode),
                                fg: self.palette.accent,
                                bg: Color::Reset,
                                modifier: Modifier::ITALIC,
                            });
                        }
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::Table(headers, rows, _alignments) => {
                        let col_count = headers.len();
                        if col_count > 0 {
                            let mut col_widths = vec![4; col_count];
                            for (i, h) in headers.iter().enumerate() {
                                col_widths[i] = col_widths[i].max(UnicodeWidthStr::width(h.as_str()));
                            }
                            for row in rows {
                                for (i, cell) in row.iter().enumerate() {
                                    if i < col_count {
                                        col_widths[i] = col_widths[i].max(UnicodeWidthStr::width(cell.as_str()));
                                    }
                                }
                            }

                            let mut header_line = String::from("│ ");
                            for (i, h) in headers.iter().enumerate() {
                                let cell_w = UnicodeWidthStr::width(h.as_str());
                                let pad = col_widths[i].saturating_sub(cell_w);
                                header_line.push_str(h);
                                header_line.push_str(&" ".repeat(pad));
                                header_line.push_str(" │ ");
                            }
                            rendered_rows.push(RenderedRow::Text {
                                text: header_line,
                                fg: Color::Rgb(249, 226, 175),
                                bg: Color::Reset,
                                modifier: Modifier::BOLD,
                            });

                            let mut sep_line = String::from("├─");
                            for (i, w) in col_widths.iter().enumerate() {
                                sep_line.push_str(&"─".repeat(*w));
                                if i + 1 < col_count {
                                    sep_line.push_str("─┼─");
                                } else {
                                    sep_line.push_str("─┤");
                                }
                            }
                            rendered_rows.push(RenderedRow::Text {
                                text: sep_line,
                                fg: Color::DarkGray,
                                bg: Color::Reset,
                                modifier: Modifier::empty(),
                            });

                            for row in rows {
                                let mut row_line = String::from("│ ");
                                for (i, &col_w) in col_widths.iter().enumerate().take(col_count) {
                                    let val = row.get(i).map(|s| s.as_str()).unwrap_or("");
                                    let cell_w = UnicodeWidthStr::width(val);
                                    let pad = col_w.saturating_sub(cell_w);
                                    row_line.push_str(val);
                                    row_line.push_str(&" ".repeat(pad));
                                    row_line.push_str(" │ ");
                                }
                                rendered_rows.push(RenderedRow::Text {
                                    text: row_line,
                                    fg: Color::Rgb(205, 214, 244),
                                    bg: Color::Reset,
                                    modifier: Modifier::empty(),
                                });
                            }
                        }
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::ThematicBreak => {
                        rendered_rows.push(RenderedRow::Text {
                            text: "─".repeat(w.min(40)),
                            fg: Color::DarkGray,
                            bg: Color::Reset,
                            modifier: Modifier::empty(),
                        });
                        rendered_rows.push(RenderedRow::Empty);
                    }
                    MarkdownBlock::FootnoteDefinition(label, content) => {
                        let sup = tenui_math::to_superscript_str(label).unwrap_or_else(|| format!("^{label}"));
                        let prefix = format!("[{sup}] ");
                        let inner_w = w.saturating_sub(prefix.chars().count()).max(1);
                        let spans =
                            parse_inline_markdown(content, self.palette.fg_muted, Modifier::ITALIC, &self.palette);
                        for (n, line) in wrap_rich_spans(&spans, inner_w).into_iter().enumerate() {
                            rendered_rows.push(RenderedRow::Spans {
                                prefix: if n == 0 {
                                    prefix.clone()
                                } else {
                                    " ".repeat(prefix.chars().count())
                                },
                                prefix_fg: self.palette.accent,
                                prefix_mod: Modifier::empty(),
                                spans: line,
                                bg: Color::Reset,
                            });
                        }
                    }
                }
            }

            self.rendered_rows = rendered_rows.iter().map(RenderedRow::plain).collect();
            self.cached_rows = rendered_rows;
            self.last_layout_width = w;
        }

        // Blit visible rows to canvas
        for y in 0..h {
            let row_idx = self.scroll_y + y;
            let cy = y as u16;

            for cx in 0..canvas.width() {
                canvas.set_char(cx, cy, ' ', Color::Reset, Color::Reset, Modifier::empty());
            }

            if row_idx >= self.cached_rows.len() {
                continue;
            }

            match &self.cached_rows[row_idx] {
                RenderedRow::Empty => {}
                RenderedRow::Text { text, fg, bg, modifier } => {
                    let mut cx = 0usize;
                    for ch in text.chars() {
                        let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                        if cx + cw > w {
                            break;
                        }
                        canvas.set_char(cx as u16, cy, ch, *fg, *bg, *modifier);
                        cx += cw;
                    }
                }
                RenderedRow::Spans {
                    prefix,
                    prefix_fg,
                    prefix_mod,
                    spans,
                    bg,
                } => {
                    let mut cx = 0usize;
                    for ch in prefix.chars() {
                        if cx >= w {
                            break;
                        }
                        canvas.set_char(cx as u16, cy, ch, *prefix_fg, *bg, *prefix_mod);
                        cx += UnicodeWidthChar::width(ch).unwrap_or(1);
                    }
                    for span in spans {
                        for ch in span.text.chars() {
                            let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                            if cx + cw > w {
                                break;
                            }
                            canvas.set_char(cx as u16, cy, ch, span.fg, *bg, span.modifier);
                            cx += cw;
                        }
                    }
                }
            }
        }

        self.selection.highlight(
            canvas,
            self.scroll_y,
            &self.rendered_rows,
            self.palette.primary,
            self.palette.bg,
        );
    }
}

#[derive(Debug, Clone)]
pub(super) enum RenderedRow {
    Empty,
    Text {
        text: String,
        fg: Color,
        bg: Color,
        modifier: Modifier,
    },
    /// A prose line rendered from styled inline spans (links, bold, italic, code), optionally
    /// with a fixed-style prefix (bullet, quote bar) drawn ahead of the spans.
    Spans {
        prefix: String,
        prefix_fg: Color,
        prefix_mod: Modifier,
        spans: Vec<RichSpan>,
        bg: Color,
    },
}

impl RenderedRow {
    /// The row's `(x_offset, plain text)` for text selection.
    fn plain(&self) -> (u16, String) {
        match self {
            RenderedRow::Empty => (0, String::new()),
            RenderedRow::Text { text, .. } => (0, text.clone()),
            RenderedRow::Spans { prefix, spans, .. } => {
                let mut s = prefix.clone();
                for span in spans {
                    s.push_str(&span.text);
                }
                (0, s)
            }
        }
    }
}

/// CommonMark AST streaming renderer that maps parser events directly to subview cells.
pub struct MarkdownRenderer;

impl MarkdownRenderer {
    pub fn render_to_subview(surface: &mut CanvasSubviewMut<'_>, markdown_source: &str) {
        let mut options = pulldown_cmark::Options::empty();
        options.insert(pulldown_cmark::Options::ENABLE_TABLES);
        options.insert(pulldown_cmark::Options::ENABLE_TASKLISTS);
        options.insert(pulldown_cmark::Options::ENABLE_STRIKETHROUGH);
        let parser = pulldown_cmark::Parser::new_ext(markdown_source, options);

        let mut cursor_y = 0u16;
        let mut in_code_block = false;
        let mut in_blockquote = false;
        let mut in_heading = false;
        let mut current_heading_level = 1;
        let mut current_modifier = Modifier::empty();

        for event in parser {
            if cursor_y >= surface.height() {
                break;
            }

            match event {
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Heading { level, .. }) => {
                    in_heading = true;
                    current_heading_level = match level {
                        pulldown_cmark::HeadingLevel::H1 => 1,
                        pulldown_cmark::HeadingLevel::H2 => 2,
                        pulldown_cmark::HeadingLevel::H3 => 3,
                        pulldown_cmark::HeadingLevel::H4 => 4,
                        pulldown_cmark::HeadingLevel::H5 => 5,
                        pulldown_cmark::HeadingLevel::H6 => 6,
                    };
                    let prefix = match current_heading_level {
                        1 => "# ",
                        2 => "## ",
                        3 => "### ",
                        4 => "#### ",
                        5 => "##### ",
                        _ => "###### ",
                    };
                    surface.write_str_clipped(0, cursor_y, prefix, Color::Cyan, Color::Reset);
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Heading(_)) => {
                    in_heading = false;
                    cursor_y = cursor_y.saturating_add(1);
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::BlockQuote(_)) => {
                    in_blockquote = true;
                    surface.write_str_clipped(0, cursor_y, "▎ ", Color::Yellow, Color::Reset);
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::BlockQuote(_)) => {
                    in_blockquote = false;
                    cursor_y = cursor_y.saturating_add(1);
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::CodeBlock(_)) => {
                    in_code_block = true;
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::CodeBlock) => {
                    in_code_block = false;
                    cursor_y = cursor_y.saturating_add(1);
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Emphasis) => {
                    current_modifier.insert(Modifier::ITALIC);
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Emphasis) => {
                    current_modifier.remove(Modifier::ITALIC);
                }
                pulldown_cmark::Event::Start(pulldown_cmark::Tag::Strong) => {
                    current_modifier.insert(Modifier::BOLD);
                }
                pulldown_cmark::Event::End(pulldown_cmark::TagEnd::Strong) => {
                    current_modifier.remove(Modifier::BOLD);
                }
                pulldown_cmark::Event::Text(text) => {
                    let (fg, start_x) = if in_code_block {
                        (Color::Green, 4)
                    } else if in_heading {
                        (Color::Cyan, (current_heading_level as u16) + 1)
                    } else if in_blockquote {
                        (Color::DarkGray, 2)
                    } else {
                        (Color::White, 0)
                    };

                    let lines: Vec<&str> = text.lines().collect();
                    for line in lines {
                        if cursor_y >= surface.height() {
                            break;
                        }
                        surface.set_string(start_x, cursor_y, line, fg, Color::Reset, current_modifier);
                        cursor_y = cursor_y.saturating_add(1);
                    }
                }
                pulldown_cmark::Event::Code(code) => {
                    surface.set_string(
                        0,
                        cursor_y,
                        &format!("`{}`", code),
                        Color::Yellow,
                        Color::Reset,
                        Modifier::empty(),
                    );
                }
                pulldown_cmark::Event::Rule => {
                    let w = surface.width();
                    let rule_str = "─".repeat(w as usize);
                    surface.write_str_clipped(0, cursor_y, &rule_str, Color::DarkGray, Color::Reset);
                    cursor_y = cursor_y.saturating_add(2);
                }
                _ => {}
            }
        }
    }
}
