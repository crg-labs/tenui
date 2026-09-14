use tenui_core::{Color, Modifier};

use crate::{rich::RichSpan, theme::ThemePalette};

/// Decodes standard HTML entities (&quot;, &amp;, &lt;, &gt;, &copy;, numeric, etc.).
fn decode_html_entity(chars: &[char]) -> Option<(String, usize)> {
    if chars.is_empty() || chars[0] != '&' {
        return None;
    }
    let semi = chars.iter().take(12).position(|&c| c == ';')?;
    let name: String = chars[1..semi].iter().collect();
    let total = semi + 1;
    let decoded = match name.as_str() {
        "quot" => "\"",
        "apos" => "'",
        "amp" => "&",
        "lt" => "<",
        "gt" => ">",
        "nbsp" => " ",
        "copy" => "©",
        "reg" => "®",
        "trade" => "™",
        "mdash" => "—",
        "ndash" => "–",
        "hellip" => "…",
        "times" => "×",
        "divide" => "÷",
        "plusmn" => "±",
        "deg" => "°",
        "euro" => "€",
        "pound" => "£",
        "yen" => "¥",
        "cent" => "¢",
        s if s.starts_with('#') => {
            let cp = if s.starts_with("#x") || s.starts_with("#X") {
                u32::from_str_radix(&s[2..], 16).ok()?
            } else {
                s[1..].parse::<u32>().ok()?
            };
            let ch = char::from_u32(cp)?;
            return Some((ch.to_string(), total));
        }
        _ => return None,
    };
    Some((decoded.to_string(), total))
}

/// Parses one line of CommonMark/GFM *inline* markup into styled [`RichSpan`]s, resolving
/// colors from `palette`. Handles inline code (`` `code` ``), links and images
/// (`[label](url)` → `label ↗`, `![alt](url)` → `📷 alt`), bold/italic/bold-italic
/// (`**`, `*`, `***`, and `_`/`__`/`___`), strikethrough (`~~`), and bare `http(s)://` URLs;
/// everything else is emitted as plain text in `default_fg` with `default_mod`.
///
/// Unlike wrapping a raw string before styling, this keeps a link or URL intact as one styled
/// span, so [`crate::rich::wrap_rich_spans`] can break the *line* around it
/// without severing the `[label](url)` delimiters. Math is intentionally out of scope here so
/// `tenui-std` stays independent of `tenui-math`.
pub fn parse_inline_markdown(
    text: &str,
    default_fg: Color,
    default_mod: Modifier,
    palette: &ThemePalette,
) -> Vec<RichSpan> {
    let mut spans: Vec<RichSpan> = Vec::new();
    let mut buf = String::new();

    let flush = |buf: &mut String, spans: &mut Vec<RichSpan>| {
        if !buf.is_empty() {
            spans.push(RichSpan::new(std::mem::take(buf), default_fg, default_mod));
        }
    };

    let chars: Vec<char> = text.chars().collect();
    let len = chars.len();
    let mut i = 0;

    while i < len {
        // Inline code `...`
        if chars[i] == '`' {
            flush(&mut buf, &mut spans);
            i += 1;
            let start = i;
            while i < len && chars[i] != '`' {
                i += 1;
            }
            let code: String = chars[start..i].iter().collect();
            if i < len {
                i += 1;
            }
            spans.push(RichSpan::new(code, palette.accent, default_mod));
            continue;
        }

        // Escaped dollar \$ -> literal $
        if chars[i] == '\\' && i + 1 < len && chars[i + 1] == '$' {
            buf.push('$');
            i += 2;
            continue;
        }

        // Display math $$...$$ inline or in table cell
        if chars[i] == '$' && i + 1 < len && chars[i + 1] == '$' {
            let mut j = i + 2;
            while j + 1 < len && !(chars[j] == '$' && chars[j + 1] == '$') {
                if chars[j] == '\\' && j + 2 < len {
                    j += 1;
                }
                j += 1;
            }
            if j + 1 < len && chars[j] == '$' && chars[j + 1] == '$' {
                flush(&mut buf, &mut spans);
                let raw: String = chars[i + 2..j].iter().collect();
                let unicode = tenui_math::latex_to_unicode(&raw);
                spans.push(RichSpan::new(
                    unicode,
                    palette.accent,
                    default_mod | Modifier::BOLD | Modifier::ITALIC,
                ));
                i = j + 2;
                continue;
            }
        }

        // Inline math $...$
        if chars[i] == '$' {
            let mut j = i + 1;
            while j < len && chars[j] != '$' {
                if chars[j] == '\\' && j + 1 < len {
                    j += 1;
                }
                j += 1;
            }
            if j < len && chars[j] == '$' && j > i + 1 {
                flush(&mut buf, &mut spans);
                let raw: String = chars[i + 1..j].iter().collect();
                let unicode = tenui_math::latex_to_unicode(&raw);
                spans.push(RichSpan::new(unicode, palette.accent, default_mod | Modifier::ITALIC));
                i = j + 1;
                continue;
            }
        }

        // Footnote reference [^label]
        if chars[i] == '[' && i + 1 < len && chars[i + 1] == '^' {
            let start = i + 2;
            let mut j = start;
            while j < len && chars[j] != ']' {
                j += 1;
            }
            if j < len && chars[j] == ']' {
                flush(&mut buf, &mut spans);
                let label: String = chars[start..j].iter().collect();
                let sup = tenui_math::to_superscript_str(&label).unwrap_or_else(|| format!("^{label}"));
                spans.push(RichSpan::new(
                    format!("[{sup}]"),
                    palette.accent,
                    default_mod | Modifier::BOLD,
                ));
                i = j + 1;
                continue;
            }
        }

        // HTML entity &name; or &#123;
        if chars[i] == '&'
            && let Some((entity, count)) = decode_html_entity(&chars[i..])
        {
            buf.push_str(&entity);
            i += count;
            continue;
        }

        // HTML tags and comments
        if chars[i] == '<' {
            // HTML comment <!-- ... -->
            if i + 3 < len && chars[i + 1] == '!' && chars[i + 2] == '-' && chars[i + 3] == '-' {
                let mut j = i + 4;
                while j + 2 < len && !(chars[j] == '-' && chars[j + 1] == '-' && chars[j + 2] == '>') {
                    j += 1;
                }
                if j + 2 < len {
                    i = j + 3;
                } else {
                    i = len;
                }
                continue;
            }

            // Keyboard shortcut badge <kbd>Key</kbd>
            let slice_6: String = chars[i..].iter().take(6).collect::<String>().to_ascii_lowercase();
            if slice_6.starts_with("<kbd>") {
                let mut j = i + 5;
                while j + 5 < len {
                    let end_tag: String = chars[j..j + 6].iter().collect::<String>().to_ascii_lowercase();
                    if end_tag == "</kbd>" {
                        break;
                    }
                    j += 1;
                }
                if j + 5 < len {
                    flush(&mut buf, &mut spans);
                    let inner: String = chars[i + 5..j].iter().collect();
                    spans.push(RichSpan::new(
                        format!("[{}]", inner.trim()),
                        palette.accent,
                        default_mod | Modifier::BOLD,
                    ));
                    i = j + 6;
                    continue;
                }
            }

            // Underline <u>text</u>
            if slice_6.starts_with("<u>") {
                let mut j = i + 3;
                while j + 3 < len {
                    let end_tag: String = chars[j..j + 4].iter().collect::<String>().to_ascii_lowercase();
                    if end_tag == "</u>" {
                        break;
                    }
                    j += 1;
                }
                if j + 3 < len {
                    flush(&mut buf, &mut spans);
                    let inner: String = chars[i + 3..j].iter().collect();
                    spans.extend(parse_inline_markdown(
                        &inner,
                        default_fg,
                        default_mod | Modifier::UNDERLINE,
                        palette,
                    ));
                    i = j + 4;
                    continue;
                }
            }

            // Subscript <sub>text</sub>
            if slice_6.starts_with("<sub>") {
                let mut j = i + 5;
                while j + 5 < len {
                    let end_tag: String = chars[j..j + 6].iter().collect::<String>().to_ascii_lowercase();
                    if end_tag == "</sub>" {
                        break;
                    }
                    j += 1;
                }
                if j + 5 < len {
                    flush(&mut buf, &mut spans);
                    let inner: String = chars[i + 5..j].iter().collect();
                    let sub = tenui_math::to_subscript_str(&inner).unwrap_or(inner);
                    spans.push(RichSpan::new(sub, default_fg, default_mod));
                    i = j + 6;
                    continue;
                }
            }

            // Superscript <sup>text</sup>
            if slice_6.starts_with("<sup>") {
                let mut j = i + 5;
                while j + 5 < len {
                    let end_tag: String = chars[j..j + 6].iter().collect::<String>().to_ascii_lowercase();
                    if end_tag == "</sup>" {
                        break;
                    }
                    j += 1;
                }
                if j + 5 < len {
                    flush(&mut buf, &mut spans);
                    let inner: String = chars[i + 5..j].iter().collect();
                    let sup = tenui_math::to_superscript_str(&inner).unwrap_or(inner);
                    spans.push(RichSpan::new(sup, palette.accent, default_mod));
                    i = j + 6;
                    continue;
                }
            }

            // Line break <br>, <br/>, <br />
            if slice_6.starts_with("<br>") {
                buf.push('\n');
                i += 4;
                continue;
            } else if slice_6.starts_with("<br/>") {
                buf.push('\n');
                i += 5;
                continue;
            } else if slice_6.starts_with("<br />") {
                buf.push('\n');
                i += 6;
                continue;
            }

            // Autolinks <http://...> / <https://...> / <mailto:...> or general HTML tag stripping
            if let Some(close) = chars[i..].iter().position(|&c| c == '>') {
                let tag_inner: String = chars[i + 1..i + close].iter().collect();
                if tag_inner.starts_with("http://")
                    || tag_inner.starts_with("https://")
                    || tag_inner.starts_with("mailto:")
                {
                    flush(&mut buf, &mut spans);
                    spans.push(RichSpan::new(
                        format!("{tag_inner} ↗"),
                        palette.accent,
                        default_mod | Modifier::UNDERLINE,
                    ));
                    i += close + 1;
                    continue;
                } else if tag_inner.starts_with('/')
                    || tag_inner.chars().all(|c| {
                        c.is_alphanumeric()
                            || c == '-'
                            || c == '_'
                            || c.is_whitespace()
                            || c == '='
                            || c == '"'
                            || c == '\''
                    })
                {
                    i += close + 1;
                    continue;
                }
            }
        }

        // Link [label](url) or image ![alt](url)
        if chars[i] == '[' || (chars[i] == '!' && i + 1 < len && chars[i + 1] == '[') {
            let is_img = chars[i] == '!';
            let bracket_start = if is_img { i + 1 } else { i };
            let mut j = bracket_start + 1;
            let mut depth = 1;
            while j < len && depth > 0 {
                match chars[j] {
                    '[' => depth += 1,
                    ']' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    j += 1;
                }
            }
            if j + 1 < len && depth == 0 && chars[j] == ']' && chars[j + 1] == '(' {
                let paren_start = j + 2;
                let mut k = paren_start;
                let mut pdepth = 1;
                while k < len && pdepth > 0 {
                    match chars[k] {
                        '(' => pdepth += 1,
                        ')' => pdepth -= 1,
                        _ => {}
                    }
                    if pdepth > 0 {
                        k += 1;
                    }
                }
                if k < len && pdepth == 0 && chars[k] == ')' {
                    let label: String = chars[bracket_start + 1..j].iter().collect();
                    let url: String = chars[paren_start..k].iter().collect();
                    flush(&mut buf, &mut spans);
                    if is_img {
                        let alt = if label.is_empty() { "Image" } else { label.trim() };
                        spans.push(RichSpan::new(
                            format!("📷 {alt}"),
                            palette.fg_muted,
                            default_mod | Modifier::ITALIC,
                        ));
                    } else {
                        let clean = label.replace("**", "").replace('*', "");
                        let shown = if clean.trim().is_empty() {
                            url.trim()
                        } else {
                            clean.trim()
                        };
                        spans.push(RichSpan::new(
                            format!("{shown} ↗"),
                            palette.accent,
                            default_mod | Modifier::UNDERLINE,
                        ));
                    }
                    i = k + 1;
                    continue;
                }
            }
        }

        // Bold-italic ***...*** / ___...___
        if (chars[i] == '*' && i + 2 < len && chars[i + 1] == '*' && chars[i + 2] == '*')
            || (chars[i] == '_' && i + 2 < len && chars[i + 1] == '_' && chars[i + 2] == '_')
        {
            let d = chars[i];
            let mut j = i + 3;
            while j + 2 < len && !(chars[j] == d && chars[j + 1] == d && chars[j + 2] == d) {
                j += 1;
            }
            if j + 2 < len && chars[j] == d && chars[j + 1] == d && chars[j + 2] == d {
                flush(&mut buf, &mut spans);
                let inner: String = chars[i + 3..j].iter().collect();
                spans.extend(parse_inline_markdown(
                    &inner,
                    palette.primary,
                    default_mod | Modifier::BOLD | Modifier::ITALIC,
                    palette,
                ));
                i = j + 3;
                continue;
            }
        }

        // Bold **...** / __...__
        if (chars[i] == '*' && i + 1 < len && chars[i + 1] == '*')
            || (chars[i] == '_' && i + 1 < len && chars[i + 1] == '_')
        {
            let d = chars[i];
            let mut j = i + 2;
            while j + 1 < len && !(chars[j] == d && chars[j + 1] == d) {
                j += 1;
            }
            if j + 1 < len && chars[j] == d && chars[j + 1] == d {
                flush(&mut buf, &mut spans);
                let inner: String = chars[i + 2..j].iter().collect();
                spans.extend(parse_inline_markdown(
                    &inner,
                    palette.primary,
                    default_mod | Modifier::BOLD,
                    palette,
                ));
                i = j + 2;
                continue;
            }
        }

        // Strikethrough ~~...~~
        if chars[i] == '~' && i + 1 < len && chars[i + 1] == '~' {
            let mut j = i + 2;
            while j + 1 < len && !(chars[j] == '~' && chars[j + 1] == '~') {
                j += 1;
            }
            if j + 1 < len && chars[j] == '~' && chars[j + 1] == '~' {
                flush(&mut buf, &mut spans);
                let inner: String = chars[i + 2..j].iter().collect();
                spans.extend(parse_inline_markdown(
                    &inner,
                    palette.fg_muted,
                    default_mod | Modifier::STRIKETHROUGH,
                    palette,
                ));
                i = j + 2;
                continue;
            }
        }

        // Italic *...* / _..._
        if (chars[i] == '*' || chars[i] == '_') && i + 1 < len && chars[i + 1] != ' ' && chars[i + 1] != chars[i] {
            let d = chars[i];
            let mut j = i + 1;
            while j < len && chars[j] != d && chars[j] != '\n' {
                j += 1;
            }
            if j < len && chars[j] == d && chars[j - 1] != ' ' {
                flush(&mut buf, &mut spans);
                let inner: String = chars[i + 1..j].iter().collect();
                spans.extend(parse_inline_markdown(
                    &inner,
                    default_fg,
                    default_mod | Modifier::ITALIC,
                    palette,
                ));
                i = j + 1;
                continue;
            }
        }

        // Bare http(s):// URL at a word boundary
        if (chars[i] == 'h') && (i == 0 || chars[i - 1].is_whitespace() || chars[i - 1] == '<' || chars[i - 1] == '(') {
            let rem: String = chars[i..].iter().take(8).collect();
            if rem.starts_with("http://") || rem.starts_with("https://") {
                flush(&mut buf, &mut spans);
                let start = i;
                while i < len && !chars[i].is_whitespace() && chars[i] != '>' && chars[i] != ')' && chars[i] != ']' {
                    i += 1;
                }
                let url: String = chars[start..i].iter().collect();
                spans.push(RichSpan::new(
                    format!("{url} ↗"),
                    palette.accent,
                    default_mod | Modifier::UNDERLINE,
                ));
                continue;
            }
        }

        buf.push(chars[i]);
        i += 1;
    }

    flush(&mut buf, &mut spans);
    if spans.is_empty() {
        spans.push(RichSpan::new(String::new(), default_fg, default_mod));
    }
    spans
}
