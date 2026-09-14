#![forbid(unsafe_code)]
//! # Tenui Macros (`tenui-macros`)
//!
//! Declarative compile-time layout DSL expanding into zero-cost typed builder calls.
//!
//! `tenui-macros` provides the [`view!`] procedural macro, allowing developers to author
//! nested flexbox hierarchies with declarative syntax and compile-time style expansion.

use proc_macro::{Delimiter, TokenStream, TokenTree};

/// Compile-time declarative UI macro expanding into zero-cost typed builder calls.
///
/// Syntax:
/// ```ignore
/// view!(ui => {
///     row [grow: 1.0] {
///         leaf [grow: 1.0] { |subview| ... };
///         leaf [width: 20.0] { |subview| ... };
///     };
///     column {
///         text [grow: 1.0] ("Status", Color::Green, Color::Reset, Modifier::BOLD);
///     };
/// });
/// ```
#[proc_macro]
pub fn view(input: TokenStream) -> TokenStream {
    let mut iter = input.into_iter();

    // 1. Parse target UI variable name (e.g. `ui`)
    let var_name = match iter.next() {
        Some(TokenTree::Ident(ident)) => ident.to_string(),
        _ => "ui".to_string(),
    };

    // 2. Consume separator `=>` or `,`
    for tt in iter {
        match &tt {
            TokenTree::Punct(p) if p.as_char() == '=' || p.as_char() == '>' || p.as_char() == ',' => {
                continue;
            }
            TokenTree::Group(group) if group.delimiter() == Delimiter::Brace => {
                let generated_body = parse_block(&var_name, group.stream());
                let output: TokenStream = generated_body.parse().expect("valid generated Rust code");
                return output;
            }
            _ => break,
        }
    }

    TokenStream::new()
}

fn parse_block(var_name: &str, stream: TokenStream) -> String {
    let mut out = String::new();
    let mut iter = stream.into_iter().peekable();

    while let Some(tt) = iter.next() {
        match tt {
            TokenTree::Ident(ident) => {
                let tag = ident.to_string();
                let mut style_mods = String::new();

                // Check for optional [style: value] attributes
                if let Some(TokenTree::Group(attr_group)) = iter.peek() {
                    if attr_group.delimiter() == Delimiter::Bracket {
                        let attr_stream = attr_group.stream();
                        style_mods = parse_style_attributes(attr_stream);
                        iter.next(); // consume bracket group
                    }
                }

                // Parse body/arguments
                match tag.as_str() {
                    "row" => {
                        if let Some(TokenTree::Group(body_group)) = iter.next() {
                            let inner = parse_block(var_name, body_group.stream());
                            out.push_str(&format!(
                                "{var}.row(tenui::layout::Style::default(){mods}, |{var}| {{ {inner} }});\n",
                                var = var_name,
                                mods = style_mods,
                                inner = inner
                            ));
                        }
                    }
                    "column" => {
                        if let Some(TokenTree::Group(body_group)) = iter.next() {
                            let inner = parse_block(var_name, body_group.stream());
                            out.push_str(&format!(
                                "{var}.column(tenui::layout::Style::default(){mods}, |{var}| {{ {inner} }});\n",
                                var = var_name,
                                mods = style_mods,
                                inner = inner
                            ));
                        }
                    }
                    "container" => {
                        if let Some(TokenTree::Group(body_group)) = iter.next() {
                            let inner = parse_block(var_name, body_group.stream());
                            out.push_str(&format!(
                                "{var}.container(tenui::layout::Style::default(){mods}, |{var}| {{ {inner} }});\n",
                                var = var_name,
                                mods = style_mods,
                                inner = inner
                            ));
                        }
                    }
                    "leaf" => {
                        if let Some(TokenTree::Group(body_group)) = iter.next() {
                            let paint_content = body_group.stream().to_string();
                            out.push_str(&format!(
                                "{var}.leaf(tenui::layout::Style::default(){mods}, {paint});\n",
                                var = var_name,
                                mods = style_mods,
                                paint = paint_content
                            ));
                        }
                    }
                    "text" => {
                        if let Some(TokenTree::Group(args_group)) = iter.next() {
                            let args = args_group.stream().to_string();
                            out.push_str(&format!(
                                "{var}.text(tenui::layout::Style::default(){mods}, {args});\n",
                                var = var_name,
                                mods = style_mods,
                                args = args
                            ));
                        }
                    }
                    _ => {
                        // Pass through unknown identifiers as raw statements
                        out.push_str(&tag);
                        out.push(' ');
                    }
                }
            }
            TokenTree::Punct(p) => {
                out.push(p.as_char());
                out.push(' ');
            }
            other => {
                out.push_str(&other.to_string());
                out.push(' ');
            }
        }
    }

    out
}

fn parse_style_attributes(stream: TokenStream) -> String {
    let mut mods = String::new();
    let text = stream.to_string();
    // Simple key: val parsing, e.g. "grow: 1.0, width: 20.0"
    for part in text.split(',') {
        let trimmed = part.trim();
        if let Some((k, v)) = trimmed.split_once(':') {
            let key = k.trim();
            let val = v.trim();
            match key {
                "grow" => mods.push_str(&format!(".grow({})", val)),
                "shrink" => mods.push_str(&format!(".shrink({})", val)),
                "width" => mods.push_str(&format!(".width_cells({})", val)),
                "height" => mods.push_str(&format!(".height_cells({})", val)),
                "padding" => mods.push_str(&format!(".padding_all({})", val)),
                "margin" => mods.push_str(&format!(".margin_all({})", val)),
                _ => {}
            }
        }
    }
    mods
}
