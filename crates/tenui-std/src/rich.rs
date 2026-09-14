//! Styled text spans and style-preserving line wrapping.
//!
//! Tenui's core wrap helpers (e.g. [`Buffer::write_str_wrapped`](tenui_core::Buffer))
//! operate on plain `&str`. That is fine for uniform text, but syntax highlighting and inline
//! Markdown produce runs of differently-styled text — and wrapping the raw string *before*
//! styling severs multi-line tokens (a long string literal, a URL, a `[label](url)` link)
//! across the break, dropping their styling.
//!
//! A [`RichSpan`] carries its own foreground color and [`Modifier`], and [`wrap_rich_spans`] /
//! [`wrap_rich_spans_cont`] wrap a whole run of them at once, so styling always follows the
//! text onto continuation lines. Adjacent spans that share a style are coalesced.

use tenui_core::{Color, Modifier};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

/// A run of text sharing a single foreground color and set of style modifiers.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichSpan {
    pub text: String,
    pub fg: Color,
    pub modifier: Modifier,
}

impl RichSpan {
    /// Creates a span from any string-like value.
    pub fn new(text: impl Into<String>, fg: Color, modifier: Modifier) -> Self {
        Self {
            text: text.into(),
            fg,
            modifier,
        }
    }

    /// The span's display width in terminal cells (wide glyphs count as 2).
    pub fn width(&self) -> usize {
        UnicodeWidthStr::width(self.text.as_str())
    }
}

/// Appends `text` to `line`, merging into the last span when it shares the same style so the
/// result stays as few spans as possible.
fn push_styled(line: &mut Vec<RichSpan>, text: &str, fg: Color, modifier: Modifier) {
    if text.is_empty() {
        return;
    }
    if let Some(last) = line.last_mut()
        && last.fg == fg
        && last.modifier == modifier
    {
        last.text.push_str(text);
        return;
    }
    line.push(RichSpan::new(text, fg, modifier));
}

/// Word-wraps a styled run to at most `max_cols` display columns, preserving each span's
/// style. Breaks at whitespace where possible and hard-splits any single word wider than the
/// line; leading whitespace at the start of a wrapped line is dropped. Always returns at least
/// one (possibly empty) line.
///
/// ```
/// use tenui_std::rich::{wrap_rich_spans, RichSpan};
/// use tenui_core::{Color, Modifier};
///
/// let spans = vec![
///     RichSpan::new("hello ", Color::White, Modifier::empty()),
///     RichSpan::new("bold world", Color::Red, Modifier::BOLD),
/// ];
/// // "hello bold world" wraps to 3 lines at width 8: "hello" / "bold" / "world".
/// let lines = wrap_rich_spans(&spans, 8);
/// assert_eq!(lines.len(), 3);
/// // Styling is retained on the wrapped line:
/// assert_eq!(lines[1][0].fg, Color::Red);
/// ```
pub fn wrap_rich_spans(spans: &[RichSpan], max_cols: usize) -> Vec<Vec<RichSpan>> {
    if max_cols == 0 {
        return vec![spans.to_vec()];
    }

    // Split every span into whitespace / non-whitespace tokens, carrying style along.
    struct Token<'a> {
        text: &'a str,
        width: usize,
        is_ws: bool,
        fg: Color,
        modifier: Modifier,
    }
    let mut tokens: Vec<Token> = Vec::new();
    for span in spans {
        let bytes = span.text.as_bytes();
        let mut start = 0;
        let mut run_ws: Option<bool> = None;
        let mut idx = 0;
        for ch in span.text.chars() {
            let is_ws = ch.is_whitespace();
            match run_ws {
                Some(prev) if prev != is_ws => {
                    let slice = &span.text[start..idx];
                    tokens.push(Token {
                        text: slice,
                        width: UnicodeWidthStr::width(slice),
                        is_ws: prev,
                        fg: span.fg,
                        modifier: span.modifier,
                    });
                    start = idx;
                }
                _ => {}
            }
            run_ws = Some(is_ws);
            idx += ch.len_utf8();
        }
        if start < bytes.len() {
            let slice = &span.text[start..];
            tokens.push(Token {
                text: slice,
                width: UnicodeWidthStr::width(slice),
                is_ws: run_ws.unwrap_or(false),
                fg: span.fg,
                modifier: span.modifier,
            });
        }
    }

    // Trims trailing whitespace off a completed line (so a break-forcing space doesn't dangle
    // at the visible edge), then moves it into `lines`.
    fn flush(lines: &mut Vec<Vec<RichSpan>>, line: &mut Vec<RichSpan>) {
        while let Some(last) = line.last_mut() {
            let trimmed = last.text.trim_end_matches(char::is_whitespace);
            if trimmed.len() == last.text.len() {
                break;
            }
            last.text.truncate(trimmed.len());
            if last.text.is_empty() {
                line.pop();
            } else {
                break;
            }
        }
        lines.push(std::mem::take(line));
    }

    let mut lines: Vec<Vec<RichSpan>> = Vec::new();
    let mut line: Vec<RichSpan> = Vec::new();
    let mut w = 0usize;

    for token in tokens {
        if token.is_ws {
            // Collapse leading whitespace; keep interior whitespace when it fits.
            if w > 0 && w + token.width <= max_cols {
                push_styled(&mut line, token.text, token.fg, token.modifier);
                w += token.width;
            }
            continue;
        }
        if token.width > max_cols {
            // Word longer than a line: flush, then hard-split by character.
            if !line.is_empty() {
                flush(&mut lines, &mut line);
                w = 0;
            }
            for ch in token.text.chars() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if w + cw > max_cols && !line.is_empty() {
                    flush(&mut lines, &mut line);
                    w = 0;
                }
                let mut b = [0u8; 4];
                push_styled(&mut line, ch.encode_utf8(&mut b), token.fg, token.modifier);
                w += cw;
            }
            continue;
        }
        if w + token.width > max_cols && !line.is_empty() {
            flush(&mut lines, &mut line);
            w = 0;
        }
        push_styled(&mut line, token.text, token.fg, token.modifier);
        w += token.width;
    }

    if !line.is_empty() || lines.is_empty() {
        flush(&mut lines, &mut line);
    }
    lines
}

/// Wraps a styled run for preformatted/code content: unlike [`wrap_rich_spans`] it does not
/// collapse whitespace (indentation is significant) and prefixes every continuation line with
/// `cont` (styled `cont_fg`) so soft-wrapped code reads as a continuation, not a new line.
/// Always returns at least one line.
pub fn wrap_rich_spans_cont(spans: &[RichSpan], max_cols: usize, cont: &str, cont_fg: Color) -> Vec<Vec<RichSpan>> {
    if max_cols == 0 {
        return vec![spans.to_vec()];
    }
    let cont_w = UnicodeWidthStr::width(cont);
    let mut lines: Vec<Vec<RichSpan>> = Vec::new();
    let mut line: Vec<RichSpan> = Vec::new();
    let mut w = 0usize;

    for span in spans {
        let mut rest = span.text.as_str();
        while !rest.is_empty() {
            // Continuation lines lose room to the marker; keep at least a little content width.
            let limit = if lines.is_empty() {
                max_cols
            } else {
                max_cols.saturating_sub(cont_w).max(4)
            };
            let avail = limit.saturating_sub(w);
            if avail == 0 {
                lines.push(std::mem::take(&mut line));
                push_styled(&mut line, cont, cont_fg, Modifier::empty());
                w = cont_w;
                continue;
            }
            let mut take = 0usize;
            let mut take_w = 0usize;
            for ch in rest.chars() {
                let cw = UnicodeWidthChar::width(ch).unwrap_or(1);
                if take_w + cw > avail && take > 0 {
                    break;
                }
                take += ch.len_utf8();
                take_w += cw;
                if take_w >= avail {
                    break;
                }
            }
            push_styled(&mut line, &rest[..take], span.fg, span.modifier);
            w += take_w;
            rest = &rest[take..];
            if w >= limit && !rest.is_empty() {
                lines.push(std::mem::take(&mut line));
                push_styled(&mut line, cont, cont_fg, Modifier::empty());
                w = cont_w;
            }
        }
    }

    if !line.is_empty() || lines.is_empty() {
        lines.push(line);
    }
    lines
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plain(line: &[RichSpan]) -> String {
        line.iter().map(|s| s.text.as_str()).collect()
    }

    #[test]
    fn wraps_at_whitespace_and_keeps_style() {
        let spans = vec![
            RichSpan::new("the quick ", Color::White, Modifier::empty()),
            RichSpan::new("brown fox", Color::Red, Modifier::BOLD),
        ];
        let lines = wrap_rich_spans(&spans, 10);
        assert_eq!(lines.len(), 2);
        assert_eq!(plain(&lines[0]), "the quick");
        assert_eq!(plain(&lines[1]), "brown fox");
        assert_eq!(lines[1][0].fg, Color::Red);
        assert!(lines[1][0].modifier.contains(Modifier::BOLD));
    }

    #[test]
    fn adjacent_same_style_spans_merge() {
        let spans = vec![
            RichSpan::new("foo", Color::White, Modifier::empty()),
            RichSpan::new("bar", Color::White, Modifier::empty()),
        ];
        let lines = wrap_rich_spans(&spans, 40);
        assert_eq!(lines.len(), 1);
        assert_eq!(lines[0].len(), 1, "same-style spans coalesce");
        assert_eq!(lines[0][0].text, "foobar");
    }

    #[test]
    fn hard_splits_overlong_word() {
        let spans = vec![RichSpan::new("abcdefghij", Color::White, Modifier::empty())];
        let lines = wrap_rich_spans(&spans, 4);
        assert_eq!(lines.len(), 3);
        assert_eq!(plain(&lines[0]), "abcd");
        assert_eq!(plain(&lines[2]), "ij");
    }

    #[test]
    fn cont_prefixes_continuation_rows() {
        let spans = vec![RichSpan::new("abcdefghij", Color::White, Modifier::empty())];
        let lines = wrap_rich_spans_cont(&spans, 6, "> ", Color::Blue);
        assert!(lines.len() >= 2);
        assert_eq!(lines[1][0].text, "> ");
        assert_eq!(lines[1][0].fg, Color::Blue);
    }

    #[test]
    fn empty_input_yields_one_empty_line() {
        assert_eq!(wrap_rich_spans(&[], 10), vec![Vec::<RichSpan>::new()]);
        assert_eq!(
            wrap_rich_spans_cont(&[], 10, "> ", Color::Blue),
            vec![Vec::<RichSpan>::new()]
        );
    }
}
