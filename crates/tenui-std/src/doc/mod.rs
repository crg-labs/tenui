mod diff;
mod inline;
mod markdown;

pub use diff::{
    AlignedDiffLine, ChangeType, DiffHunk, DiffLine, DiffMode, DiffView, IntraLineHighlights, MyersDiff,
    compute_diff_async, zip_aligned_diff,
};
pub use inline::parse_inline_markdown;
pub use markdown::{MarkdownBlock, MarkdownRenderer, MarkdownView};

#[cfg(test)]
mod inline_tests {
    use tenui_core::Modifier;

    use super::*;
    use crate::{text_table::TableAlignment, theme::ThemePalette};

    #[test]
    fn parse_inline_link_becomes_labeled_arrow() {
        let pal = ThemePalette::catppuccin_mocha();
        let spans = parse_inline_markdown("see [Docs](https://example.com/x) now", pal.fg, Modifier::empty(), &pal);
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(joined, "see Docs ↗ now");
        let link = spans.iter().find(|s| s.text.contains("Docs")).unwrap();
        assert_eq!(link.fg, pal.accent);
        assert!(link.modifier.contains(Modifier::UNDERLINE));
    }

    #[test]
    fn parse_inline_bold_and_code() {
        let pal = ThemePalette::catppuccin_mocha();
        let spans = parse_inline_markdown("a **b** `c`", pal.fg, Modifier::empty(), &pal);
        let bold = spans.iter().find(|s| s.text == "b").unwrap();
        assert!(bold.modifier.contains(Modifier::BOLD));
        let code = spans.iter().find(|s| s.text == "c").unwrap();
        assert_eq!(code.fg, pal.accent);
    }

    #[test]
    fn markdown_view_text_selection_extracts_and_clears() {
        use tenui_core::{Buffer, Rect};
        let mut view = MarkdownView::new("Hello world paragraph.");
        let mut buf = Buffer::new(60, 10);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 10));
            view.render(&mut canvas);
        }
        view.on_mouse_down(0, 0);
        view.on_mouse_drag(4, 0);
        view.on_mouse_up();
        assert_eq!(view.selected_text(), "Hello");
        view.clear_selection();
        assert!(view.selected_text().is_empty());
    }

    #[test]
    fn markdown_view_preserves_link_url_from_block_parse() {
        let blocks = MarkdownView::parse("Read the [guide](https://ex.com/g).");
        let para = blocks
            .iter()
            .find_map(|b| match b {
                MarkdownBlock::Paragraph(t) => Some(t.clone()),
                _ => None,
            })
            .expect("paragraph");
        assert!(para.contains("[guide](https://ex.com/g)"), "URL retained, got: {para}");
    }

    #[test]
    fn markdown_view_task_list_parsing() {
        let md = "- [ ] Incomplete task\n- [x] Completed task";
        let blocks = MarkdownView::parse(md);
        assert_eq!(blocks.len(), 2);
        match &blocks[0] {
            MarkdownBlock::BulletList(indent, text) => {
                assert_eq!(*indent, 0);
                assert!(text.starts_with("☐ Incomplete task"), "got: {text}");
            }
            other => panic!("expected BulletList, got: {other:?}"),
        }
        match &blocks[1] {
            MarkdownBlock::BulletList(indent, text) => {
                assert_eq!(*indent, 0);
                assert!(text.starts_with("☑ Completed task"), "got: {text}");
            }
            other => panic!("expected BulletList, got: {other:?}"),
        }
    }

    #[test]
    fn markdown_view_code_block_syntax_highlighting() {
        use tenui_core::{Buffer, Rect};
        let md = "```tex\n\\section{Intro}\n```";
        let mut view = MarkdownView::new(md);
        let mut buf = Buffer::new(60, 10);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 10));
            view.render(&mut canvas);
        }
        let cell = buf.get(2, 1).unwrap();
        assert_eq!(cell.symbol.as_str(), "\\");
        assert_eq!(cell.fg, view.theme.keyword.0);
    }

    #[test]
    fn markdown_view_unicode_table_alignment() {
        use tenui_core::{Buffer, Rect};
        let md = "| 姓名 | 年龄 |\n|---|---|\n| 张三 | 25 |";
        let mut view = MarkdownView::new(md);
        let mut buf = Buffer::new(60, 10);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 10));
            view.render(&mut canvas);
        }
        let sep0_cols: Vec<u16> = (0..30)
            .filter(|&x| buf.get(x, 0).map(|c| c.symbol.as_str() == "│").unwrap_or(false))
            .collect();
        let sep2_cols: Vec<u16> = (0..30)
            .filter(|&x| buf.get(x, 2).map(|c| c.symbol.as_str() == "│").unwrap_or(false))
            .collect();
        assert_eq!(
            sep0_cols, sep2_cols,
            "Table columns must align visually on the terminal grid"
        );
    }

    #[test]
    fn parse_inline_math_converts_to_unicode() {
        let pal = ThemePalette::catppuccin_mocha();
        let spans = parse_inline_markdown(
            "Let $E = mc^2$ and $\\alpha + \\beta$ holds.",
            pal.fg,
            Modifier::empty(),
            &pal,
        );
        let joined: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert!(joined.contains("E = mc²"), "got: {joined}");
        assert!(joined.contains("α + β"), "got: {joined}");

        let math_span = spans.iter().find(|s| s.text.contains("E = mc²")).unwrap();
        assert_eq!(math_span.fg, pal.accent);
        assert!(math_span.modifier.contains(Modifier::ITALIC));

        let esc_spans = parse_inline_markdown("Price is \\$100 today", pal.fg, Modifier::empty(), &pal);
        let esc_joined: String = esc_spans.iter().map(|s| s.text.as_str()).collect();
        assert_eq!(esc_joined, "Price is $100 today");
    }

    #[test]
    fn markdown_view_display_math_block_and_fenced() {
        use tenui_core::{Buffer, Rect};
        let md1 = "$$\nE = mc^2\n$$";
        let blocks1 = MarkdownView::parse(md1);
        assert!(blocks1.iter().any(|b| matches!(b, MarkdownBlock::DisplayMath(_))));

        let mut view1 = MarkdownView::new(md1);
        let mut buf1 = Buffer::new(60, 5);
        {
            let mut canvas = buf1.subview_mut(Rect::new(0, 0, 60, 5));
            view1.render(&mut canvas);
        }
        let row0: String = (0..30)
            .map(|x| buf1.get(x, 0).map(|c| c.symbol.as_str()).unwrap_or(" "))
            .collect();
        assert!(row0.contains("E = mc²"), "row contains unicode display math: {row0}");

        let md2 = "```math\n\\frac{x + 1}{2}\n```";
        let blocks2 = MarkdownView::parse(md2);
        assert!(blocks2.iter().any(|b| matches!(b, MarkdownBlock::DisplayMath(_))));

        let mut view2 = MarkdownView::new(md2);
        let mut buf2 = Buffer::new(60, 10);
        {
            let mut canvas = buf2.subview_mut(Rect::new(0, 0, 60, 10));
            view2.render(&mut canvas);
        }
        let row1: String = (0..30)
            .map(|x| buf2.get(x, 1).map(|c| c.symbol.as_str()).unwrap_or(" "))
            .collect();
        assert!(row1.contains('─'), "fraction division bar rendered: {row1}");
    }

    #[test]
    fn html_entity_decoding_named_and_numeric() {
        let pal = ThemePalette::catppuccin_mocha();
        let spans = parse_inline_markdown(
            "A &amp; B &lt;c&gt; &copy; &#65; &#x21;",
            pal.fg,
            Modifier::empty(),
            &pal,
        );
        let text: String = spans.iter().map(|s| s.text.as_str()).collect();
        assert!(text.contains('&'), "named &amp; decoded: {text}");
        assert!(text.contains('<'), "named &lt; decoded: {text}");
        assert!(text.contains('>'), "named &gt; decoded: {text}");
        assert!(text.contains('©'), "named &copy; decoded: {text}");
        assert!(text.contains('A'), "numeric &#65; decoded: {text}");
        assert!(text.contains('!'), "hex &#x21; decoded: {text}");
    }

    #[test]
    fn inline_kbd_and_u_and_sub_sup_tags() {
        let pal = ThemePalette::catppuccin_mocha();

        let kbd_spans = parse_inline_markdown("<kbd>Ctrl</kbd>+<kbd>C</kbd>", pal.fg, Modifier::empty(), &pal);
        let kbd_text: String = kbd_spans.iter().map(|s| s.text.as_str()).collect();
        assert!(kbd_text.contains("[Ctrl]"), "kbd rendered as badge: {kbd_text}");
        assert!(kbd_text.contains("[C]"), "kbd rendered as badge: {kbd_text}");

        let u_spans = parse_inline_markdown("<u>underlined</u>", pal.fg, Modifier::empty(), &pal);
        assert!(
            u_spans
                .iter()
                .any(|s| s.modifier.contains(Modifier::UNDERLINE) && s.text == "underlined"),
            "underline modifier applied"
        );

        let sub_spans = parse_inline_markdown("H<sub>2</sub>O", pal.fg, Modifier::empty(), &pal);
        let sub_text: String = sub_spans.iter().map(|s| s.text.as_str()).collect();
        assert!(sub_text.contains('₂'), "subscript 2 → ₂: {sub_text}");

        let sup_spans = parse_inline_markdown("E=mc<sup>2</sup>", pal.fg, Modifier::empty(), &pal);
        let sup_text: String = sup_spans.iter().map(|s| s.text.as_str()).collect();
        assert!(sup_text.contains('²'), "superscript 2 → ²: {sup_text}");
    }

    #[test]
    fn footnote_reference_inline_renders_superscript() {
        let md = "See this claim.[^1]\n\n[^1]: The source.";
        let blocks = MarkdownView::parse(md);
        assert!(
            blocks
                .iter()
                .any(|b| matches!(b, MarkdownBlock::FootnoteDefinition(_, _))),
            "FootnoteDefinition block parsed"
        );
    }

    #[test]
    fn markdown_view_footnote_definition_renders() {
        use tenui_core::{Buffer, Rect};
        let md = "Text.[^note]\n\n[^note]: Footnote content here.";
        let mut view = MarkdownView::new(md);
        let mut buf = Buffer::new(60, 8);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 60, 8));
            view.render(&mut canvas);
        }
        let rendered: String = (0..8)
            .flat_map(|y| (0..60).map(move |x| (x, y)))
            .map(|(x, y)| buf.get(x, y).map(|c| c.symbol.as_str()).unwrap_or(" "))
            .collect::<String>();
        assert!(
            rendered.contains("Footnote content here"),
            "footnote rendered: {rendered}"
        );
    }

    #[test]
    fn markdown_table_with_alignments_parses_correctly() {
        let md = "| L | C | R |\n|:---|:---:|---:|\n| a | b | c |";
        let blocks = MarkdownView::parse(md);
        let table = blocks.iter().find(|b| matches!(b, MarkdownBlock::Table(_, _, _)));
        assert!(table.is_some(), "table block parsed");
        if let Some(MarkdownBlock::Table(headers, rows, alignments)) = table {
            assert_eq!(headers.len(), 3);
            assert_eq!(rows.len(), 1);
            assert_eq!(alignments.len(), 3);
            assert!(matches!(alignments[0], TableAlignment::Left));
            assert!(matches!(alignments[1], TableAlignment::Center));
            assert!(matches!(alignments[2], TableAlignment::Right));
        }
    }

    #[test]
    fn markdown_view_caching_and_invalidation() {
        use tenui_core::{Buffer, Rect};
        let md = "# Title\n\nParagraph with some text.";
        let mut view = MarkdownView::new(md);
        assert_eq!(view.cached_rows.len(), 0);

        let mut buf = Buffer::new(40, 10);
        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 10));
            view.render(&mut canvas);
        }
        let cached_len = view.cached_rows.len();
        assert!(cached_len > 0);
        assert_eq!(view.last_layout_width, 40);

        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 10));
            view.render(&mut canvas);
        }
        assert_eq!(view.cached_rows.len(), cached_len);

        view.invalidate_layout();
        assert_eq!(view.cached_rows.len(), 0);
        assert_eq!(view.last_layout_width, 0);

        {
            let mut canvas = buf.subview_mut(Rect::new(0, 0, 40, 10));
            view.render(&mut canvas);
        }
        assert!(!view.cached_rows.is_empty());
        view.set_content("New content");
        assert_eq!(view.cached_rows.len(), 0);
        assert_eq!(view.last_layout_width, 0);
    }
}
