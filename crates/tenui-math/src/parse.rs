//! LaTeX mathematical expression parser and Unicode converter for `tenui-math`.

use crate::typeset::MathNode;

/// Converts a single ASCII character to its superscript Unicode equivalent.
pub fn to_superscript_char(c: char) -> Option<char> {
    match c {
        '0' => Some('⁰'),
        '1' => Some('¹'),
        '2' => Some('²'),
        '3' => Some('³'),
        '4' => Some('⁴'),
        '5' => Some('⁵'),
        '6' => Some('⁶'),
        '7' => Some('⁷'),
        '8' => Some('⁸'),
        '9' => Some('⁹'),
        '+' => Some('⁺'),
        '-' | '−' => Some('⁻'),
        '=' => Some('⁼'),
        '(' => Some('⁽'),
        ')' => Some('⁾'),
        'a' => Some('ᵃ'),
        'b' => Some('ᵇ'),
        'c' => Some('ᶜ'),
        'd' => Some('ᵈ'),
        'e' => Some('ᵉ'),
        'f' => Some('ᶠ'),
        'g' => Some('ᵍ'),
        'h' => Some('ʰ'),
        'i' => Some('ⁱ'),
        'j' => Some('ʲ'),
        'k' => Some('ᵏ'),
        'l' => Some('ˡ'),
        'm' => Some('ᵐ'),
        'n' => Some('ⁿ'),
        'o' => Some('ᵒ'),
        'p' => Some('ᵖ'),
        'r' => Some('ʳ'),
        's' => Some('ˢ'),
        't' => Some('ᵗ'),
        'u' => Some('ᵘ'),
        'v' => Some('ᵛ'),
        'w' => Some('ʷ'),
        'x' => Some('ˣ'),
        'y' => Some('ʸ'),
        'z' => Some('ᶻ'),
        'A' => Some('ᴬ'),
        'B' => Some('ᴮ'),
        'D' => Some('ᴰ'),
        'E' => Some('ᴱ'),
        'G' => Some('ᴳ'),
        'H' => Some('ᴴ'),
        'I' => Some('ᴵ'),
        'J' => Some('ᴶ'),
        'K' => Some('ᴷ'),
        'L' => Some('ᴸ'),
        'M' => Some('ᴹ'),
        'N' => Some('ᴺ'),
        'O' => Some('ᴼ'),
        'P' => Some('ᴾ'),
        'R' => Some('ᴿ'),
        'T' => Some('ᵀ'),
        'U' => Some('ᵁ'),
        'W' => Some('ᵂ'),
        _ => None,
    }
}

/// Converts a single ASCII character to its subscript Unicode equivalent.
pub fn to_subscript_char(c: char) -> Option<char> {
    match c {
        '0' => Some('₀'),
        '1' => Some('₁'),
        '2' => Some('₂'),
        '3' => Some('₃'),
        '4' => Some('₄'),
        '5' => Some('₅'),
        '6' => Some('₆'),
        '7' => Some('₇'),
        '8' => Some('₈'),
        '9' => Some('₉'),
        '+' => Some('₊'),
        '-' | '−' => Some('₋'),
        '=' => Some('₌'),
        '(' => Some('₍'),
        ')' => Some('₎'),
        'a' => Some('ₐ'),
        'e' => Some('ₑ'),
        'h' => Some('ₕ'),
        'i' => Some('ᵢ'),
        'j' => Some('ⱼ'),
        'k' => Some('ₖ'),
        'l' => Some('ₗ'),
        'm' => Some('ₘ'),
        'n' => Some('ₙ'),
        'o' => Some('ₒ'),
        'p' => Some('ₚ'),
        'r' => Some('ᵣ'),
        's' => Some('ₛ'),
        't' => Some('ₜ'),
        'u' => Some('ᵤ'),
        'v' => Some('ᵥ'),
        'x' => Some('ₓ'),
        _ => None,
    }
}

/// Attempts to convert an entire string to Unicode superscript characters.
pub fn to_superscript_str(s: &str) -> Option<String> {
    let mut res = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        let sup = to_superscript_char(c)?;
        res.push(sup);
    }
    Some(res)
}

/// Attempts to convert an entire string to Unicode subscript characters.
pub fn to_subscript_str(s: &str) -> Option<String> {
    let mut res = String::with_capacity(s.len() * 2);
    for c in s.chars() {
        let sub = to_subscript_char(c)?;
        res.push(sub);
    }
    Some(res)
}

/// Maps a LaTeX command name (without backslash) to a single mathematical symbol char.
pub fn latex_command_to_char(cmd: &str) -> Option<char> {
    match cmd {
        // Greek lowercase
        "alpha" => Some('α'),
        "beta" => Some('β'),
        "gamma" => Some('γ'),
        "delta" => Some('δ'),
        "epsilon" | "varepsilon" => Some('ε'),
        "zeta" => Some('ζ'),
        "eta" => Some('η'),
        "theta" | "vartheta" => Some('θ'),
        "iota" => Some('ι'),
        "kappa" => Some('κ'),
        "lambda" => Some('λ'),
        "mu" => Some('μ'),
        "nu" => Some('ν'),
        "xi" => Some('ξ'),
        "pi" | "varpi" => Some('π'),
        "rho" | "varrho" => Some('ρ'),
        "sigma" | "varsigma" => Some('σ'),
        "tau" => Some('τ'),
        "upsilon" => Some('υ'),
        "phi" | "varphi" => Some('φ'),
        "chi" => Some('χ'),
        "psi" => Some('ψ'),
        "omega" => Some('ω'),

        // Greek uppercase
        "Gamma" => Some('Γ'),
        "Delta" => Some('Δ'),
        "Theta" => Some('Θ'),
        "Lambda" => Some('Λ'),
        "Xi" => Some('Ξ'),
        "Pi" => Some('Π'),
        "Sigma" => Some('Σ'),
        "Upsilon" => Some('Υ'),
        "Phi" => Some('Φ'),
        "Psi" => Some('Ψ'),
        "Omega" => Some('Ω'),

        // Constants and operators
        "infty" => Some('∞'),
        "partial" => Some('∂'),
        "nabla" => Some('∇'),
        "pm" => Some('±'),
        "mp" => Some('∓'),
        "times" => Some('×'),
        "div" => Some('÷'),
        "cdot" => Some('·'),
        "circ" => Some('∘'),
        "bullet" => Some('•'),
        "ast" | "star" => Some('★'),
        "dag" | "dagger" => Some('†'),
        "ddag" | "ddagger" => Some('‡'),
        "ell" => Some('ℓ'),
        "hbar" => Some('ℏ'),
        "Re" => Some('ℜ'),
        "Im" => Some('ℑ'),
        "aleph" => Some('ℵ'),

        // Relations
        "leq" | "le" => Some('≤'),
        "geq" | "ge" => Some('≥'),
        "neq" | "ne" => Some('≠'),
        "approx" => Some('≈'),
        "equiv" => Some('≡'),
        "sim" => Some('~'),
        "simeq" => Some('≃'),
        "cong" => Some('≅'),
        "propto" => Some('∝'),
        "ll" => Some('≪'),
        "gg" => Some('≫'),
        "perp" => Some('⊥'),

        // Sets and logic
        "in" => Some('∈'),
        "notin" => Some('∉'),
        "subset" => Some('⊂'),
        "supset" => Some('⊃'),
        "subseteq" => Some('⊆'),
        "supseteq" => Some('⊇'),
        "cup" => Some('∪'),
        "cap" => Some('∩'),
        "setminus" => Some('∖'),
        "forall" => Some('∀'),
        "exists" => Some('∃'),
        "nexists" => Some('∄'),
        "emptyset" | "varnothing" => Some('∅'),

        // Arrows
        "to" | "rightarrow" => Some('→'),
        "leftarrow" | "gets" => Some('←'),
        "Rightarrow" | "implies" => Some('⇒'),
        "Leftarrow" => Some('⇐'),
        "Leftrightarrow" | "iff" => Some('⇔'),
        "mapsto" => Some('↦'),
        "uparrow" => Some('↑'),
        "downarrow" => Some('↓'),

        // Big operators
        "sum" => Some('∑'),
        "prod" => Some('∏'),
        "coprod" => Some('∐'),
        "int" => Some('∫'),
        "iint" => Some('∬'),
        "iiint" => Some('∭'),
        "oint" => Some('∮'),

        _ => None,
    }
}

/// Token representation for LaTeX parsing.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Command(String),
    Group(Vec<Token>),
    BracketGroup(Vec<Token>),
    Char(char),
    Sup,
    Sub,
    Amp,
    DoubleBackslash,
    Space,
}

/// Maximum brace/bracket nesting depth the tokenizer will descend into. Groups nested deeper
/// than this are dropped rather than recursed into. Because the parser's recursion follows the
/// tokenizer's group nesting, this single bound also caps parser recursion — so hostile input
/// like `{{{{…}}}}` (e.g. from an LLM) cannot overflow the stack. Real math never approaches it.
const MAX_NEST_DEPTH: usize = 64;

/// Tokenizes a LaTeX string into a nested stream of `Token`s.
fn tokenize_latex(input: &str) -> Vec<Token> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    tokenize_slice(&chars, &mut pos, 0)
}

fn tokenize_slice(chars: &[char], pos: &mut usize, nest_depth: usize) -> Vec<Token> {
    let mut tokens = Vec::new();
    if nest_depth > MAX_NEST_DEPTH {
        // Refuse to descend further; skip to the end of this slice so the caller still advances.
        *pos = chars.len();
        return tokens;
    }

    while *pos < chars.len() {
        let c = chars[*pos];
        match c {
            ' ' | '\t' | '\r' | '\n' => {
                *pos += 1;
            }
            '{' => {
                *pos += 1;
                let mut depth = 1;
                let start = *pos;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '{' {
                        depth += 1;
                    } else if chars[*pos] == '}' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let inner = &chars[start..*pos];
                if *pos < chars.len() && chars[*pos] == '}' {
                    *pos += 1;
                }
                let mut inner_pos = 0;
                tokens.push(Token::Group(tokenize_slice(inner, &mut inner_pos, nest_depth + 1)));
            }
            '[' => {
                *pos += 1;
                let mut depth = 1;
                let start = *pos;
                while *pos < chars.len() && depth > 0 {
                    if chars[*pos] == '[' {
                        depth += 1;
                    } else if chars[*pos] == ']' {
                        depth -= 1;
                    }
                    if depth > 0 {
                        *pos += 1;
                    }
                }
                let inner = &chars[start..*pos];
                if *pos < chars.len() && chars[*pos] == ']' {
                    *pos += 1;
                }
                let mut inner_pos = 0;
                tokens.push(Token::BracketGroup(tokenize_slice(
                    inner,
                    &mut inner_pos,
                    nest_depth + 1,
                )));
            }
            '}' | ']' => {
                *pos += 1;
            }
            '^' => {
                tokens.push(Token::Sup);
                *pos += 1;
            }
            '_' => {
                tokens.push(Token::Sub);
                *pos += 1;
            }
            '&' => {
                tokens.push(Token::Amp);
                *pos += 1;
            }
            '\\' => {
                *pos += 1;
                if *pos >= chars.len() {
                    break;
                }
                let next_c = chars[*pos];
                if next_c == '\\' {
                    tokens.push(Token::DoubleBackslash);
                    *pos += 1;
                } else if next_c.is_alphabetic() {
                    let start = *pos;
                    while *pos < chars.len() && chars[*pos].is_alphabetic() {
                        *pos += 1;
                    }
                    let cmd: String = chars[start..*pos].iter().collect();
                    tokens.push(Token::Command(cmd));
                } else {
                    *pos += 1;
                    match next_c {
                        ',' | ';' | ':' | ' ' => tokens.push(Token::Space),
                        '!' => {} // Negative thin space (ignored)
                        '{' => tokens.push(Token::Char('{')),
                        '}' => tokens.push(Token::Char('}')),
                        '%' => tokens.push(Token::Char('%')),
                        '_' => tokens.push(Token::Char('_')),
                        '&' => tokens.push(Token::Char('&')),
                        '[' => tokens.push(Token::Char('[')),
                        ']' => tokens.push(Token::Char(']')),
                        '(' => tokens.push(Token::Char('(')),
                        ')' => tokens.push(Token::Char(')')),
                        other => tokens.push(Token::Char(other)),
                    }
                }
            }
            other => {
                tokens.push(Token::Char(other));
                *pos += 1;
            }
        }
    }

    tokens
}

/// Strips outer math environment delimiters (`\[ ... \]`, `\( ... \)`, `$$ ... $$`, `$ ... $`, `\begin{equation}`).
pub fn strip_outer_math_delimiters(input: &str) -> &str {
    let mut s = input.trim();

    // Check for \[ ... \]
    if s.starts_with("\\[") && s.ends_with("\\]") && s.len() >= 4 {
        s = &s[2..s.len() - 2];
        s = s.trim();
    }
    // Check for \( ... \)
    if s.starts_with("\\(") && s.ends_with("\\)") && s.len() >= 4 {
        s = &s[2..s.len() - 2];
        s = s.trim();
    }
    // Check for $$ ... $$
    if s.starts_with("$$") && s.ends_with("$$") && s.len() >= 4 {
        s = &s[2..s.len() - 2];
        s = s.trim();
    }
    // Check for single $ ... $
    if s.starts_with('$') && s.ends_with('$') && s.len() >= 2 && !s.starts_with("$$") {
        s = &s[1..s.len() - 1];
        s = s.trim();
    }

    // Environments: \begin{equation} ... \end{equation}
    for env in &["equation", "equation*", "align", "align*", "gather", "gather*"] {
        let b_tag = format!("\\begin{{{}}}", env);
        let e_tag = format!("\\end{{{}}}", env);
        if s.starts_with(&b_tag) && s.ends_with(&e_tag) {
            s = &s[b_tag.len()..s.len() - e_tag.len()];
            s = s.trim();
        }
    }

    s
}

/// Parses a LaTeX mathematical expression into a `MathNode` AST.
pub fn parse_latex(input: &str) -> MathNode {
    let clean_str = strip_outer_math_delimiters(input);
    let tokens = tokenize_latex(clean_str);
    let mut pos = 0;
    let nodes = parse_token_sequence(&tokens, &mut pos, false);
    normalize_nodes(nodes)
}

fn parse_token_sequence(tokens: &[Token], pos: &mut usize, stop_at_relation: bool) -> Vec<MathNode> {
    let mut nodes: Vec<MathNode> = Vec::new();

    while *pos < tokens.len() {
        let token = &tokens[*pos];

        if stop_at_relation {
            match token {
                Token::Char('=') | Token::Char('<') | Token::Char('>') => break,
                Token::Command(cmd) if is_relation_command(cmd) => break,
                Token::Amp | Token::DoubleBackslash => break,
                _ => {}
            }
        }

        match token {
            Token::Command(cmd) => {
                *pos += 1;
                match cmd.as_str() {
                    // Formatting directives to ignore
                    "displaystyle" | "textstyle" | "scriptstyle" | "scriptscriptstyle" | "limits" | "nolimits" => {}

                    // Delimiter hints \left and \right
                    "left" | "right" => {
                        if *pos < tokens.len() {
                            match &tokens[*pos] {
                                Token::Char('.') => {
                                    *pos += 1; // \left. is invisible delimiter
                                }
                                Token::Char(delim) => {
                                    nodes.push(MathNode::symbol(*delim));
                                    *pos += 1;
                                }
                                _ => {}
                            }
                        }
                    }

                    // Text mode inside math
                    "text" | "mathrm" | "mathbf" | "mathit" | "operatorname" | "textbf" | "textit" => {
                        let text_content = if *pos < tokens.len() {
                            if let Token::Group(group) = &tokens[*pos] {
                                *pos += 1;
                                tokens_to_plain_string(group)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        nodes.push(MathNode::text(text_content));
                    }

                    // Fraction \frac{num}{den}
                    "frac" | "dfrac" | "tfrac" => {
                        let num = if *pos < tokens.len() {
                            parse_single_arg(tokens, pos)
                        } else {
                            MathNode::text("?")
                        };
                        let den = if *pos < tokens.len() {
                            parse_single_arg(tokens, pos)
                        } else {
                            MathNode::text("?")
                        };
                        nodes.push(MathNode::fraction(num, den));
                    }

                    // Square root \sqrt[n]{x}
                    "sqrt" => {
                        // Optional [n]
                        let degree = if *pos < tokens.len() {
                            if let Token::BracketGroup(bg) = &tokens[*pos] {
                                *pos += 1;
                                Some(tokens_to_plain_string(bg))
                            } else {
                                None
                            }
                        } else {
                            None
                        };
                        let inner = if *pos < tokens.len() {
                            parse_single_arg(tokens, pos)
                        } else {
                            MathNode::text("")
                        };

                        if let Some(deg) = degree {
                            let sup_deg = to_superscript_str(&deg).unwrap_or(deg);
                            nodes.push(MathNode::Row(vec![
                                MathNode::text(sup_deg),
                                MathNode::symbol('√'),
                                inner,
                            ]));
                        } else {
                            nodes.push(MathNode::Row(vec![MathNode::symbol('√'), inner]));
                        }
                    }

                    // Integral \int with optional limits
                    "int" | "iint" | "iiint" | "oint" => {
                        let (lower, upper) = parse_sub_and_sup_limits(tokens, pos);
                        // Integrand runs until end of input or relation '='
                        let integrand_nodes = parse_token_sequence(tokens, pos, true);
                        let integrand = normalize_nodes(integrand_nodes);

                        nodes.push(MathNode::integral(lower, upper, integrand));
                    }

                    // Summation \sum and Product \prod with optional limits
                    "sum" | "prod" | "coprod" => {
                        let (lower, upper) = parse_sub_and_sup_limits(tokens, pos);
                        let sym = latex_command_to_char(cmd).unwrap_or('∑');
                        let base_node = MathNode::symbol(sym);

                        // If both lower and upper are simple text, convert to unicode or use supsub
                        let base_with_limits = if lower.is_some() || upper.is_some() {
                            MathNode::supsub(base_node, upper, lower)
                        } else {
                            base_node
                        };
                        nodes.push(base_with_limits);
                    }

                    // Matrices
                    "begin" => {
                        let env_name = if *pos < tokens.len() {
                            if let Token::Group(g) = &tokens[*pos] {
                                *pos += 1;
                                tokens_to_plain_string(g)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };

                        if env_name.ends_with("matrix") || env_name == "array" {
                            let matrix_node = parse_matrix_contents(tokens, pos, &env_name);
                            nodes.push(matrix_node);
                        }
                    }

                    "end" => {
                        if *pos < tokens.len()
                            && let Token::Group(_) = &tokens[*pos]
                        {
                            *pos += 1;
                        }
                    }

                    // Standard symbol lookup
                    _ => {
                        if let Some(ch) = latex_command_to_char(cmd) {
                            nodes.push(MathNode::symbol(ch));
                        } else {
                            // Unrecognized command, keep as text
                            nodes.push(MathNode::text(format!("\\{}", cmd)));
                        }
                    }
                }
            }

            Token::Sup => {
                *pos += 1;
                if *pos < tokens.len() {
                    let sup_node = parse_single_arg(tokens, pos);
                    attach_superscript(&mut nodes, sup_node);
                }
            }

            Token::Sub => {
                *pos += 1;
                if *pos < tokens.len() {
                    let sub_node = parse_single_arg(tokens, pos);
                    attach_subscript(&mut nodes, sub_node);
                }
            }

            Token::Group(inner_tokens) => {
                *pos += 1;
                let mut inner_pos = 0;
                let inner_nodes = parse_token_sequence(inner_tokens, &mut inner_pos, false);
                nodes.push(normalize_nodes(inner_nodes));
            }

            Token::BracketGroup(inner_tokens) => {
                *pos += 1;
                let mut inner_pos = 0;
                let inner_nodes = parse_token_sequence(inner_tokens, &mut inner_pos, false);
                nodes.push(MathNode::symbol('['));
                nodes.push(normalize_nodes(inner_nodes));
                nodes.push(MathNode::symbol(']'));
            }

            Token::Char(c) => {
                *pos += 1;
                let ch = *c;
                if ch == '=' {
                    nodes.push(MathNode::text(" = "));
                } else if ch == '+' {
                    nodes.push(MathNode::text(" + "));
                } else if ch == '-' || ch == '−' {
                    nodes.push(MathNode::text(" - "));
                } else if ch == '*' {
                    nodes.push(MathNode::symbol('·'));
                } else {
                    nodes.push(MathNode::symbol(ch));
                }
            }

            Token::Space => {
                *pos += 1;
                nodes.push(MathNode::text(" "));
            }

            Token::Amp | Token::DoubleBackslash => {
                *pos += 1;
            }
        }
    }

    nodes
}

fn parse_single_arg(tokens: &[Token], pos: &mut usize) -> MathNode {
    if *pos >= tokens.len() {
        return MathNode::text("");
    }

    match &tokens[*pos] {
        Token::Group(inner) => {
            *pos += 1;
            let mut inner_pos = 0;
            let nodes = parse_token_sequence(inner, &mut inner_pos, false);
            normalize_nodes(nodes)
        }
        Token::Command(cmd) => {
            let cmd_clone = cmd.clone();
            *pos += 1;
            if let Some(ch) = latex_command_to_char(&cmd_clone) {
                MathNode::symbol(ch)
            } else {
                MathNode::text(format!("\\{}", cmd_clone))
            }
        }
        Token::Char(c) => {
            let ch = *c;
            *pos += 1;
            MathNode::symbol(ch)
        }
        Token::Space => {
            *pos += 1;
            MathNode::text(" ")
        }
        _ => {
            *pos += 1;
            MathNode::text("")
        }
    }
}

fn parse_sub_and_sup_limits(tokens: &[Token], pos: &mut usize) -> (Option<MathNode>, Option<MathNode>) {
    let mut lower = None;
    let mut upper = None;

    for _ in 0..2 {
        if *pos < tokens.len() {
            if tokens[*pos] == Token::Sub && lower.is_none() {
                *pos += 1;
                lower = Some(parse_single_arg(tokens, pos));
            } else if tokens[*pos] == Token::Sup && upper.is_none() {
                *pos += 1;
                upper = Some(parse_single_arg(tokens, pos));
            }
        }
    }

    (lower, upper)
}

fn attach_superscript(nodes: &mut Vec<MathNode>, sup_node: MathNode) {
    let last = match nodes.pop() {
        Some(n) => n,
        None => MathNode::text(""),
    };

    // If sup is simple text that can be converted to unicode superscripts
    if let MathNode::Text(sup_s) = &sup_node {
        if let Some(unicode_sup) = to_superscript_str(sup_s) {
            match last {
                MathNode::Text(mut base_s) => {
                    base_s.push_str(&unicode_sup);
                    nodes.push(MathNode::Text(base_s));
                    return;
                }
                MathNode::Symbol(c) => {
                    let mut s = c.to_string();
                    s.push_str(&unicode_sup);
                    nodes.push(MathNode::Text(s));
                    return;
                }
                _ => {}
            }
        }
    } else if let MathNode::Symbol(c) = sup_node
        && let Some(unicode_c) = to_superscript_char(c)
    {
        match last {
            MathNode::Text(mut base_s) => {
                base_s.push(unicode_c);
                nodes.push(MathNode::Text(base_s));
                return;
            }
            MathNode::Symbol(base_c) => {
                let mut s = base_c.to_string();
                s.push(unicode_c);
                nodes.push(MathNode::Text(s));
                return;
            }
            _ => {}
        }
    }

    // Fall back to general MathNode::supsub
    match last {
        MathNode::SuperscriptSubscript { base, sub, sup: None } => {
            nodes.push(MathNode::SuperscriptSubscript {
                base,
                sub,
                sup: Some(Box::new(sup_node)),
            });
        }
        other => {
            nodes.push(MathNode::supsub(other, Some(sup_node), None));
        }
    }
}

fn attach_subscript(nodes: &mut Vec<MathNode>, sub_node: MathNode) {
    let last = match nodes.pop() {
        Some(n) => n,
        None => MathNode::text(""),
    };

    // If sub is simple text that can be converted to unicode subscripts
    if let MathNode::Text(sub_s) = &sub_node {
        if let Some(unicode_sub) = to_subscript_str(sub_s) {
            match last {
                MathNode::Text(mut base_s) => {
                    base_s.push_str(&unicode_sub);
                    nodes.push(MathNode::Text(base_s));
                    return;
                }
                MathNode::Symbol(c) => {
                    let mut s = c.to_string();
                    s.push_str(&unicode_sub);
                    nodes.push(MathNode::Text(s));
                    return;
                }
                _ => {}
            }
        }
    } else if let MathNode::Symbol(c) = sub_node
        && let Some(unicode_c) = to_subscript_char(c)
    {
        match last {
            MathNode::Text(mut base_s) => {
                base_s.push(unicode_c);
                nodes.push(MathNode::Text(base_s));
                return;
            }
            MathNode::Symbol(base_c) => {
                let mut s = base_c.to_string();
                s.push(unicode_c);
                nodes.push(MathNode::Text(s));
                return;
            }
            _ => {}
        }
    }

    // Fall back to general MathNode::supsub
    match last {
        MathNode::SuperscriptSubscript { base, sub: None, sup } => {
            nodes.push(MathNode::SuperscriptSubscript {
                base,
                sub: Some(Box::new(sub_node)),
                sup,
            });
        }
        other => {
            nodes.push(MathNode::supsub(other, None, Some(sub_node)));
        }
    }
}

fn parse_matrix_contents(tokens: &[Token], pos: &mut usize, env_name: &str) -> MathNode {
    let mut rows: Vec<Vec<MathNode>> = Vec::new();
    let mut current_row: Vec<MathNode> = Vec::new();
    let mut current_cell_tokens: Vec<Token> = Vec::new();

    let flush_cell = |cell_tokens: &mut Vec<Token>, row: &mut Vec<MathNode>| {
        let mut c_pos = 0;
        let cell_nodes = parse_token_sequence(cell_tokens, &mut c_pos, false);
        row.push(normalize_nodes(cell_nodes));
        cell_tokens.clear();
    };

    while *pos < tokens.len() {
        match &tokens[*pos] {
            Token::Command(cmd) if cmd == "end" => {
                *pos += 1;
                if *pos < tokens.len()
                    && let Token::Group(g) = &tokens[*pos]
                {
                    let name = tokens_to_plain_string(g);
                    if name == *env_name {
                        *pos += 1;
                        break;
                    }
                }
            }
            Token::Amp => {
                *pos += 1;
                flush_cell(&mut current_cell_tokens, &mut current_row);
            }
            Token::DoubleBackslash => {
                *pos += 1;
                flush_cell(&mut current_cell_tokens, &mut current_row);
                rows.push(std::mem::take(&mut current_row));
            }
            other => {
                current_cell_tokens.push(other.clone());
                *pos += 1;
            }
        }
    }

    if !current_cell_tokens.is_empty() || !current_row.is_empty() {
        flush_cell(&mut current_cell_tokens, &mut current_row);
        rows.push(current_row);
    }

    MathNode::matrix(rows)
}

fn normalize_nodes(mut nodes: Vec<MathNode>) -> MathNode {
    if nodes.is_empty() {
        return MathNode::text("");
    }
    if nodes.len() == 1 {
        return nodes.remove(0);
    }

    // Coalesce adjacent Text and Symbol nodes into clean string runs
    let mut coalesced: Vec<MathNode> = Vec::new();
    for n in nodes {
        match n {
            MathNode::Text(t) => {
                if let Some(MathNode::Text(last_t)) = coalesced.last_mut() {
                    last_t.push_str(&t);
                } else {
                    coalesced.push(MathNode::Text(t));
                }
            }
            MathNode::Symbol(c) => {
                if let Some(MathNode::Text(last_t)) = coalesced.last_mut() {
                    last_t.push(c);
                } else {
                    coalesced.push(MathNode::Text(c.to_string()));
                }
            }
            other => {
                coalesced.push(other);
            }
        }
    }

    if coalesced.len() == 1 {
        coalesced.remove(0)
    } else {
        MathNode::Row(coalesced)
    }
}

fn tokens_to_plain_string(tokens: &[Token]) -> String {
    let mut s = String::new();
    for t in tokens {
        match t {
            Token::Char(c) => s.push(*c),
            Token::Command(cmd) => {
                if let Some(ch) = latex_command_to_char(cmd) {
                    s.push(ch);
                } else {
                    s.push_str(cmd);
                }
            }
            Token::Group(g) | Token::BracketGroup(g) => s.push_str(&tokens_to_plain_string(g)),
            Token::Space => s.push(' '),
            Token::Sup => s.push('^'),
            Token::Sub => s.push('_'),
            Token::Amp => s.push('&'),
            Token::DoubleBackslash => s.push_str("\\\\"),
        }
    }
    s
}

fn is_relation_command(cmd: &str) -> bool {
    matches!(
        cmd,
        "leq" | "le" | "geq" | "ge" | "neq" | "ne" | "approx" | "equiv" | "sim" | "simeq" | "cong" | "propto"
    )
}

/// Converts an inline LaTeX expression into a clean single-line Unicode string.
pub fn latex_to_unicode(input: &str) -> String {
    let clean = strip_outer_math_delimiters(input);
    let tokens = tokenize_latex(clean);
    let mut out = String::new();
    let mut pos = 0;

    while pos < tokens.len() {
        match &tokens[pos] {
            Token::Command(cmd) => {
                pos += 1;
                match cmd.as_str() {
                    "displaystyle" | "textstyle" | "scriptstyle" | "limits" | "nolimits" => {}
                    "left" | "right" => {
                        if pos < tokens.len()
                            && let Token::Char(c) = tokens[pos]
                        {
                            if c != '.' {
                                out.push(c);
                            }
                            pos += 1;
                        }
                    }
                    "frac" | "dfrac" => {
                        let num = if pos < tokens.len() {
                            if let Token::Group(g) = &tokens[pos] {
                                pos += 1;
                                tokens_to_plain_string(g)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        let den = if pos < tokens.len() {
                            if let Token::Group(g) = &tokens[pos] {
                                pos += 1;
                                tokens_to_plain_string(g)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        out.push_str(&format!("({}/{})", num, den));
                    }
                    "sqrt" => {
                        let inner = if pos < tokens.len() {
                            if let Token::Group(g) = &tokens[pos] {
                                pos += 1;
                                tokens_to_plain_string(g)
                            } else {
                                String::new()
                            }
                        } else {
                            String::new()
                        };
                        out.push_str(&format!("√({})", inner));
                    }
                    "text" | "mathrm" | "mathbf" | "mathit" => {
                        if pos < tokens.len()
                            && let Token::Group(g) = &tokens[pos]
                        {
                            pos += 1;
                            out.push_str(&tokens_to_plain_string(g));
                        }
                    }
                    _ => {
                        if let Some(ch) = latex_command_to_char(cmd) {
                            out.push(ch);
                        } else {
                            out.push_str(cmd);
                        }
                    }
                }
            }
            Token::Sup => {
                pos += 1;
                if pos < tokens.len() {
                    let sup_str = match &tokens[pos] {
                        Token::Group(g) => {
                            pos += 1;
                            tokens_to_plain_string(g)
                        }
                        Token::Char(c) => {
                            let ch = *c;
                            pos += 1;
                            ch.to_string()
                        }
                        _ => {
                            pos += 1;
                            String::new()
                        }
                    };
                    if let Some(unicode_sup) = to_superscript_str(&sup_str) {
                        out.push_str(&unicode_sup);
                    } else {
                        out.push('^');
                        out.push_str(&sup_str);
                    }
                }
            }
            Token::Sub => {
                pos += 1;
                if pos < tokens.len() {
                    let sub_str = match &tokens[pos] {
                        Token::Group(g) => {
                            pos += 1;
                            tokens_to_plain_string(g)
                        }
                        Token::Char(c) => {
                            let ch = *c;
                            pos += 1;
                            ch.to_string()
                        }
                        _ => {
                            pos += 1;
                            String::new()
                        }
                    };
                    if let Some(unicode_sub) = to_subscript_str(&sub_str) {
                        out.push_str(&unicode_sub);
                    } else {
                        out.push('_');
                        out.push_str(&sub_str);
                    }
                }
            }
            Token::Char(c) => {
                let ch = *c;
                pos += 1;
                if ch == '=' {
                    out.push_str(" = ");
                } else if ch == '+' {
                    out.push_str(" + ");
                } else if ch == '-' || ch == '−' {
                    out.push_str(" - ");
                } else {
                    out.push(ch);
                }
            }
            Token::Group(g) => {
                pos += 1;
                out.push_str(&tokens_to_plain_string(g));
            }
            Token::BracketGroup(g) => {
                pos += 1;
                out.push('[');
                out.push_str(&tokens_to_plain_string(g));
                out.push(']');
            }
            Token::Space => {
                pos += 1;
                out.push(' ');
            }
            Token::Amp => {
                pos += 1;
                out.push_str("  ");
            }
            Token::DoubleBackslash => {
                pos += 1;
                out.push_str(" ; ");
            }
        }
    }

    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_integral_equation() {
        let input = r"\[ \displaystyle I=\int x^{2}\,e^{x}\,dx . \]";
        let node = parse_latex(input);

        match &node {
            MathNode::Row(items) => {
                assert_eq!(items.len(), 2);
                match &items[0] {
                    MathNode::Text(s) => assert_eq!(s, "I = "),
                    other => panic!("Expected text, got {:?}", other),
                }
                match &items[1] {
                    MathNode::Integral { integrand, .. } => {
                        // Integrand contains x² eˣ dx .
                        match &**integrand {
                            MathNode::Text(s) => {
                                assert!(s.contains("x²"), "s was: {}", s);
                                assert!(s.contains("eˣ"), "s was: {}", s);
                                assert!(s.contains("dx"), "s was: {}", s);
                            }
                            other => panic!("Expected text integrand, got {:?}", other),
                        }
                    }
                    other => panic!("Expected integral, got {:?}", other),
                }
            }
            other => panic!("Expected Row, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_integration_by_parts() {
        let input = r"\int u\,dv = uv-\int v\,du";
        let node = parse_latex(input);

        match &node {
            MathNode::Row(items) => {
                assert_eq!(items.len(), 3);
                assert!(matches!(&items[0], MathNode::Integral { .. }));
                match &items[1] {
                    MathNode::Text(s) => assert_eq!(s, " = uv - "),
                    other => panic!("Expected text, got {:?}", other),
                }
                assert!(matches!(&items[2], MathNode::Integral { .. }));
            }
            other => panic!("Expected Row, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_fraction_and_greek() {
        let input = r"\frac{\alpha + \beta}{\gamma}";
        let node = parse_latex(input);

        match node {
            MathNode::Fraction(num, den) => {
                match *num {
                    MathNode::Text(s) => assert_eq!(s, "α + β"),
                    other => panic!("Unexpected num: {:?}", other),
                }
                match *den {
                    MathNode::Text(s) => assert_eq!(s, "γ"),
                    MathNode::Symbol(c) => assert_eq!(c, 'γ'),
                    other => panic!("Unexpected den: {:?}", other),
                }
            }
            other => panic!("Expected fraction, got {:?}", other),
        }
    }

    #[test]
    fn test_latex_to_unicode() {
        let input = r"\( \displaystyle I = \int x^2\,e^x\,dx \)";
        let res = latex_to_unicode(input);
        assert!(res.contains('∫'));
        assert!(res.contains("x²"));
        assert!(res.contains("eˣ"));
    }

    #[test]
    fn test_parse_and_typeset_render() {
        use tenui_core::{Buffer, Color, Rect};

        use crate::typeset::MathTypesetter;

        let input = r"\[ \displaystyle I=\int x^{2}\,e^{x}\,dx . \]";
        let node = parse_latex(input);
        let typesetter = MathTypesetter;
        let (w, h) = typesetter.measure(&node);
        assert!(w >= 10);
        assert_eq!(h, 1);

        let mut buffer = Buffer::empty(Rect::new(0, 0, w, h));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, w, h));
        typesetter.render(&mut subview, 0, 0, &node, Color::White);

        // Verify integral glyph is clean single-line ∫
        assert_eq!(subview.get(4, 0).unwrap().symbol.as_str(), "∫");

        // Verify "I = " is on line y=0
        assert_eq!(subview.get(0, 0).unwrap().symbol.as_str(), "I");
        assert_eq!(subview.get(2, 0).unwrap().symbol.as_str(), "=");
    }

    #[test]
    fn test_parse_and_typeset_render_with_limits() {
        use tenui_core::{Buffer, Color, Rect};

        use crate::typeset::MathTypesetter;

        let input = r"\[ \int_{0}^{\infty} f(x)\,dx \]";
        let node = parse_latex(input);
        let typesetter = MathTypesetter;
        let (w, h) = typesetter.measure(&node);
        assert!(w >= 8);
        assert_eq!(h, 3);

        let mut buffer = Buffer::empty(Rect::new(0, 0, w, h));
        let mut subview = buffer.subview_mut(Rect::new(0, 0, w, h));
        typesetter.render(&mut subview, 0, 0, &node, Color::White);

        // Verify integral multi-row glyphs with limits
        assert_eq!(subview.get(0, 0).unwrap().symbol.as_str(), "⌠");
        assert_eq!(subview.get(0, 1).unwrap().symbol.as_str(), "│");
        assert_eq!(subview.get(0, 2).unwrap().symbol.as_str(), "⌡");
    }

    #[test]
    fn test_deeply_nested_input_does_not_overflow() {
        // Pathological untrusted input: thousands of nested groups. The tokenizer depth cap
        // must keep this from recursing the stack to death; it should parse without panicking.
        let depth = 5_000;
        let hostile = format!("{}x{}", "{".repeat(depth), "}".repeat(depth));
        let _ = parse_latex(&hostile);

        let frac = format!("{}1{}", "\\frac{".repeat(depth), "}{2}".repeat(depth));
        let _ = parse_latex(&frac);
    }
}
