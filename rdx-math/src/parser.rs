/// Recursive descent parser: converts a `TokenStream` into a `MathExpr` tree.
use crate::tokenizer::{Token, TokenStream};
use crate::{
    AccentKind, AlignRow, CaseRow, ColumnAlign, Delimiter, FracStyle, LimitStyle, MathExpr,
    MathFont, MathOperator, MathSpace, MathStyle, MatrixDelimiters, OperatorKind, SmashMode,
    symbols,
};

// ─── Public entry point ───────────────────────────────────────────────────────

/// Parse a complete expression from the token stream.
/// Returns a single `MathExpr`; if multiple atoms were found they are wrapped in `Row`.
pub(crate) fn parse_expr(ts: &mut TokenStream) -> MathExpr {
    let items = parse_row(ts);
    flatten_row(items)
}

// ─── Row / atom parsing ───────────────────────────────────────────────────────

/// Parse a sequence of atoms until we hit something that ends a row:
/// `}`, `\end`, `&`, `\\`, or Eof.
pub(crate) fn parse_row(ts: &mut TokenStream) -> Vec<MathExpr> {
    let mut items: Vec<MathExpr> = Vec::new();

    loop {
        ts.skip_whitespace();

        match ts.peek() {
            Token::Eof
            | Token::RBrace
            | Token::End(_)
            | Token::Ampersand
            | Token::DoubleBackslash => break,

            // \right closes a \left group — stop collecting
            Token::Command(cmd) if cmd == "right" => break,

            _ => {
                let atom = parse_atom(ts);
                let with_scripts = parse_scripts(ts, atom);
                items.push(with_scripts);
            }
        }
    }

    items
}

/// Parse a single atom (one "thing"), without trailing scripts.
fn parse_atom(ts: &mut TokenStream) -> MathExpr {
    ts.skip_whitespace();

    match ts.peek().clone() {
        Token::LBrace => parse_group(ts),

        Token::Letter(c) => {
            ts.next();
            MathExpr::Ident {
                value: c.to_string(),
            }
        }

        Token::Digit(c) => {
            // Collect a run of digits + optional decimal point into a Number
            let mut s = String::new();
            s.push(c);
            ts.next();
            loop {
                match ts.peek() {
                    Token::Digit(d) => {
                        s.push(*d);
                        ts.next();
                    }
                    Token::Dot => {
                        // Only absorb the dot if a digit follows (e.g. 3.14)
                        if matches!(ts.peek_ahead(1), Token::Digit(_)) {
                            s.push('.');
                            ts.next(); // dot
                        } else {
                            break;
                        }
                    }
                    _ => break,
                }
            }
            MathExpr::Number { value: s }
        }

        Token::Dot => {
            ts.next();
            MathExpr::Number {
                value: ".".to_string(),
            }
        }

        Token::Plus => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "+".to_string(),
                kind: OperatorKind::Binary,
            })
        }

        Token::Minus => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "-".to_string(),
                kind: OperatorKind::Binary,
            })
        }

        Token::Equals => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "=".to_string(),
                kind: OperatorKind::Relation,
            })
        }

        Token::LessThan => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "<".to_string(),
                kind: OperatorKind::Relation,
            })
        }

        Token::GreaterThan => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: ">".to_string(),
                kind: OperatorKind::Relation,
            })
        }

        Token::Comma => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: ",".to_string(),
                kind: OperatorKind::Punctuation,
            })
        }

        Token::Semicolon => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: ";".to_string(),
                kind: OperatorKind::Punctuation,
            })
        }

        Token::Colon => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: ":".to_string(),
                kind: OperatorKind::Punctuation,
            })
        }

        Token::Bang => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "!".to_string(),
                kind: OperatorKind::Postfix,
            })
        }

        Token::Prime => {
            ts.next();
            // Prime is effectively superscript ′
            MathExpr::Ident {
                value: "′".to_string(),
            }
        }

        Token::Pipe => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "|".to_string(),
                kind: OperatorKind::Binary,
            })
        }

        Token::LParen => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "(".to_string(),
                kind: OperatorKind::Prefix,
            })
        }

        Token::RParen => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: ")".to_string(),
                kind: OperatorKind::Postfix,
            })
        }

        Token::LBracket => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "[".to_string(),
                kind: OperatorKind::Prefix,
            })
        }

        Token::RBracket => {
            ts.next();
            MathExpr::Operator(MathOperator {
                symbol: "]".to_string(),
                kind: OperatorKind::Postfix,
            })
        }

        Token::Tilde => {
            ts.next();
            MathExpr::Space(MathSpace::Thin)
        }

        Token::ThinSpace => {
            ts.next();
            MathExpr::Space(MathSpace::Thin)
        }

        Token::MedSpace => {
            ts.next();
            MathExpr::Space(MathSpace::Medium)
        }

        Token::NegThinSpace => {
            ts.next();
            MathExpr::Space(MathSpace::NegThin)
        }

        Token::Command(cmd) => {
            ts.next();
            parse_command(ts, &cmd)
        }

        Token::Begin(env) => {
            ts.next();
            parse_environment(ts, &env)
        }

        // Dangling ^ or _ without a preceding base — emit error
        Token::Caret => {
            ts.next();
            let script = parse_script_arg(ts);
            MathExpr::Superscript {
                base: Box::new(MathExpr::Error {
                    raw: "^".to_string(),
                    message: "^ without a base".to_string(),
                }),
                script: Box::new(script),
            }
        }

        Token::Underscore => {
            ts.next();
            let script = parse_script_arg(ts);
            MathExpr::Subscript {
                base: Box::new(MathExpr::Error {
                    raw: "_".to_string(),
                    message: "_ without a base".to_string(),
                }),
                script: Box::new(script),
            }
        }

        // These should not appear here as atoms — treat as errors
        Token::DoubleBackslash => {
            let raw = "\\\\".to_string();
            ts.next();
            MathExpr::Error {
                raw: raw.clone(),
                message: "unexpected row separator \\\\".to_string(),
            }
        }

        Token::Ampersand => {
            ts.next();
            MathExpr::Error {
                raw: "&".to_string(),
                message: "unexpected & outside environment".to_string(),
            }
        }

        Token::Whitespace => {
            ts.next();
            MathExpr::Space(MathSpace::Thin)
        }

        Token::Eof | Token::RBrace | Token::End(_) => {
            // Should not reach here: parse_row stops before these.
            MathExpr::Error {
                raw: String::new(),
                message: "unexpected end of input".to_string(),
            }
        }
    }
}

/// After parsing an atom, look ahead for `^` and/or `_` and attach them.
fn parse_scripts(ts: &mut TokenStream, base: MathExpr) -> MathExpr {
    ts.skip_whitespace();

    let has_sub = matches!(ts.peek(), Token::Underscore);
    let has_sup = matches!(ts.peek(), Token::Caret);

    if !has_sub && !has_sup {
        return base;
    }

    let mut sub: Option<Box<MathExpr>> = None;
    let mut sup: Option<Box<MathExpr>> = None;

    // Consume up to two script markers in any order: _...^... or ^..._...
    for _ in 0..2 {
        ts.skip_whitespace();
        match ts.peek().clone() {
            Token::Underscore if sub.is_none() => {
                ts.next();
                sub = Some(Box::new(parse_script_arg(ts)));
            }
            Token::Caret if sup.is_none() => {
                ts.next();
                sup = Some(Box::new(parse_script_arg(ts)));
            }
            _ => break,
        }
    }

    match (sub, sup) {
        (Some(s), None) => MathExpr::Subscript {
            base: Box::new(base),
            script: s,
        },
        (None, Some(s)) => MathExpr::Superscript {
            base: Box::new(base),
            script: s,
        },
        (Some(sub), Some(sup)) => MathExpr::Subsuperscript {
            base: Box::new(base),
            sub,
            sup,
        },
        (None, None) => base, // unreachable but safe
    }
}

/// Parse the argument to `^` or `_`. If the next token is `{`, parse a group; otherwise
/// consume a single atom (single-char unbraced script).
fn parse_script_arg(ts: &mut TokenStream) -> MathExpr {
    ts.skip_whitespace();
    if matches!(ts.peek(), Token::LBrace) {
        parse_group(ts)
    } else {
        parse_atom(ts)
    }
}

/// Parse `{...}` — consume `{`, parse row, consume `}`.
/// Returns the inner contents as a Row (or single item if only one).
pub(crate) fn parse_group(ts: &mut TokenStream) -> MathExpr {
    // Expect LBrace
    if !matches!(ts.peek(), Token::LBrace) {
        return MathExpr::Error {
            raw: String::new(),
            message: "expected { but found something else".to_string(),
        };
    }
    ts.next(); // consume {

    let items = parse_row(ts);

    if matches!(ts.peek(), Token::RBrace) {
        ts.next(); // consume }
    } else {
        // Unmatched brace — still return what we have
        return MathExpr::Error {
            raw: format!("{{{}", exprs_to_raw(&items)),
            message: "unmatched { — missing }".to_string(),
        };
    }

    flatten_row(items)
}

// ─── Command dispatch ─────────────────────────────────────────────────────────

/// Given a command name (already consumed from the stream), produce the appropriate MathExpr.
pub(crate) fn parse_command(ts: &mut TokenStream, cmd: &str) -> MathExpr {
    // ── Spacing ──
    match cmd {
        "quad" => return MathExpr::Space(MathSpace::Quad),
        "qquad" => return MathExpr::Space(MathSpace::QQuad),
        "," => return MathExpr::Space(MathSpace::Thin),
        ";" | ":" => return MathExpr::Space(MathSpace::Medium),
        "!" => return MathExpr::Space(MathSpace::NegThin),
        " " => return MathExpr::Space(MathSpace::Thin),
        _ => {}
    }

    // ── Simple identifiers ──
    match cmd {
        "infty" => {
            return MathExpr::Ident {
                value: "∞".to_string(),
            };
        }
        "partial" => {
            return MathExpr::Ident {
                value: "∂".to_string(),
            };
        }
        "nabla" => {
            return MathExpr::Ident {
                value: "∇".to_string(),
            };
        }
        "ell" => {
            return MathExpr::Ident {
                value: "ℓ".to_string(),
            };
        }
        "hbar" => {
            return MathExpr::Ident {
                value: "ℏ".to_string(),
            };
        }
        "emptyset" => {
            return MathExpr::Ident {
                value: "∅".to_string(),
            };
        }
        "varnothing" => {
            return MathExpr::Ident {
                value: "∅".to_string(),
            };
        }
        "aleph" => {
            return MathExpr::Ident {
                value: "ℵ".to_string(),
            };
        }
        "forall" => {
            return MathExpr::Ident {
                value: "∀".to_string(),
            };
        }
        "exists" => {
            return MathExpr::Ident {
                value: "∃".to_string(),
            };
        }
        _ => {}
    }

    // ── Greek letters ──
    if let Some(sym) = symbols::greek_letter(cmd) {
        return MathExpr::Ident {
            value: sym.to_string(),
        };
    }

    // ── Operators ──
    if let Some((sym, kind)) = symbols::operator(cmd) {
        return MathExpr::Operator(MathOperator {
            symbol: sym.to_string(),
            kind,
        });
    }

    // ── Large operators ──
    if let Some(sym) = symbols::large_operator(cmd) {
        let mut lower: Option<Box<MathExpr>> = None;
        let mut upper: Option<Box<MathExpr>> = None;

        // Consume optional scripts attached to the large operator
        for _ in 0..2 {
            ts.skip_whitespace();
            match ts.peek().clone() {
                Token::Underscore if lower.is_none() => {
                    ts.next();
                    lower = Some(Box::new(parse_script_arg(ts)));
                }
                Token::Caret if upper.is_none() => {
                    ts.next();
                    upper = Some(Box::new(parse_script_arg(ts)));
                }
                _ => break,
            }
        }

        return MathExpr::BigOperator {
            op: MathOperator {
                symbol: sym.to_string(),
                kind: OperatorKind::Large,
            },
            limits: LimitStyle::DisplayLimits,
            lower,
            upper,
        };
    }

    // ── Named operators (\lim, \sin, etc.) ──
    if let Some(name) = symbols::named_operator(cmd) {
        return MathExpr::Operator(MathOperator {
            symbol: name.to_string(),
            kind: OperatorKind::Prefix,
        });
    }

    // ── Fractions ──
    match cmd {
        "frac" => {
            let num = parse_group(ts);
            let den = parse_group(ts);
            return MathExpr::Frac {
                numerator: Box::new(num),
                denominator: Box::new(den),
                style: FracStyle::Auto,
            };
        }
        "dfrac" => {
            let num = parse_group(ts);
            let den = parse_group(ts);
            return MathExpr::Frac {
                numerator: Box::new(num),
                denominator: Box::new(den),
                style: FracStyle::Display,
            };
        }
        "tfrac" => {
            let num = parse_group(ts);
            let den = parse_group(ts);
            return MathExpr::Frac {
                numerator: Box::new(num),
                denominator: Box::new(den),
                style: FracStyle::Text,
            };
        }
        "binom" => {
            // Treat as a display fraction with Paren delimiters via Fenced
            let num = parse_group(ts);
            let den = parse_group(ts);
            // For now represent as fenced frac (semantically: C(n,k))
            return MathExpr::Fenced {
                open: Delimiter::Paren,
                close: Delimiter::Paren,
                body: vec![MathExpr::Frac {
                    numerator: Box::new(num),
                    denominator: Box::new(den),
                    style: FracStyle::Auto,
                }],
            };
        }
        _ => {}
    }

    // ── Square root ──
    if cmd == "sqrt" {
        ts.skip_whitespace();
        // Optional index in [ ]
        let index = if matches!(ts.peek(), Token::LBracket) {
            ts.next(); // [
            let idx_items = parse_until_rbracket(ts);
            let idx = flatten_row(idx_items);
            if matches!(ts.peek(), Token::RBracket) {
                ts.next(); // ]
            }
            Some(Box::new(idx))
        } else {
            None
        };

        let body = parse_group(ts);
        return MathExpr::Sqrt {
            index,
            body: Box::new(body),
        };
    }

    // ── Text / font commands that take one brace argument ──
    if let Some(font) = symbols::font_override_command(cmd) {
        // \text{...} reads the brace content as a raw string to preserve spaces.
        if matches!(font, MathFont::Roman) && matches!(cmd, "text" | "mbox") {
            ts.skip_whitespace();
            let raw = ts.read_raw_brace_string().unwrap_or_default();
            return MathExpr::Text { value: raw };
        }
        let body = parse_group(ts);
        return MathExpr::FontOverride {
            font,
            body: Box::new(body),
        };
    }

    // ── \left ... \right ──
    if cmd == "left" {
        return parse_delimited(ts);
    }

    // ── \right — should have been caught by parse_delimited, but if it leaks ──
    if cmd == "right" {
        // This is a parser error: \right without matching \left
        return MathExpr::Error {
            raw: "\\right".to_string(),
            message: "\\right without matching \\left".to_string(),
        };
    }

    // ── Tier 2: Accents ──
    if let Some(kind) = accent_kind(cmd) {
        let body = parse_group(ts);
        return MathExpr::Accent {
            kind,
            body: Box::new(body),
        };
    }

    // ── Tier 2: Over/under decorations ──
    match cmd {
        "overline" => {
            let body = parse_group(ts);
            return MathExpr::Overline {
                body: Box::new(body),
            };
        }
        "underline" => {
            let body = parse_group(ts);
            return MathExpr::Underline {
                body: Box::new(body),
            };
        }
        "overbrace" => {
            let body = parse_group(ts);
            return MathExpr::Overbrace {
                body: Box::new(body),
                annotation: None,
            };
        }
        "underbrace" => {
            let body = parse_group(ts);
            return MathExpr::Underbrace {
                body: Box::new(body),
                annotation: None,
            };
        }
        "overset" | "stackrel" => {
            let above = parse_group(ts);
            let base = parse_group(ts);
            return MathExpr::Overset {
                over: Box::new(above),
                base: Box::new(base),
            };
        }
        "underset" => {
            let below = parse_group(ts);
            let base = parse_group(ts);
            return MathExpr::Underset {
                under: Box::new(below),
                base: Box::new(base),
            };
        }
        _ => {}
    }

    // ── Tier 2: Style overrides ──
    match cmd {
        "displaystyle" => {
            return MathExpr::StyleOverride {
                style: MathStyle::Display,
                body: Box::new(parse_style_body(ts)),
            };
        }
        "textstyle" => {
            return MathExpr::StyleOverride {
                style: MathStyle::Text,
                body: Box::new(parse_style_body(ts)),
            };
        }
        "scriptstyle" => {
            return MathExpr::StyleOverride {
                style: MathStyle::Script,
                body: Box::new(parse_style_body(ts)),
            };
        }
        "scriptscriptstyle" => {
            return MathExpr::StyleOverride {
                style: MathStyle::ScriptScript,
                body: Box::new(parse_style_body(ts)),
            };
        }
        _ => {}
    }

    // ── Tier 3: Phantoms ──
    match cmd {
        "phantom" => {
            let body = parse_group(ts);
            return MathExpr::Phantom {
                body: Box::new(body),
            };
        }
        "hphantom" => {
            let body = parse_group(ts);
            return MathExpr::HPhantom {
                body: Box::new(body),
            };
        }
        "vphantom" => {
            let body = parse_group(ts);
            return MathExpr::VPhantom {
                body: Box::new(body),
            };
        }
        "smash" => {
            // \smash[t]{...} or \smash{...}
            ts.skip_whitespace();
            let mode = if matches!(ts.peek(), Token::LBracket) {
                ts.next(); // [
                let mode_str = collect_until_rbracket_str(ts);
                if matches!(ts.peek(), Token::RBracket) {
                    ts.next();
                }
                match mode_str.trim() {
                    "t" => SmashMode::Top,
                    "b" => SmashMode::Bottom,
                    _ => SmashMode::Both,
                }
            } else {
                SmashMode::Both
            };
            let body = parse_group(ts);
            return MathExpr::Smash {
                mode,
                body: Box::new(body),
            };
        }
        _ => {}
    }

    // ── Tier 3: Color ──
    match cmd {
        "color" | "textcolor" => {
            let color_group = parse_group(ts);
            let color_name = extract_text_content(&color_group);
            let body = parse_group(ts);
            return MathExpr::Color {
                color: color_name,
                body: Box::new(body),
            };
        }
        _ => {}
    }

    // ── Tier 3: operatorname ──
    if cmd == "operatorname" {
        let name_group = parse_group(ts);
        let name = extract_text_content(&name_group);
        return MathExpr::Operator(MathOperator {
            symbol: name,
            kind: OperatorKind::Prefix,
        });
    }

    // ── Tier 3: mhchem ──
    if cmd == "ce" {
        let body = parse_group(ts);
        let raw = extract_text_content(&body);
        return MathExpr::Chem { value: raw };
    }

    // ── Delimiter literals that appear after \left/\right (handled elsewhere) ──
    // But if they appear standalone (e.g. \langle without \left), emit as Ident.
    match cmd {
        "langle" => {
            return MathExpr::Ident {
                value: "⟨".to_string(),
            };
        }
        "rangle" => {
            return MathExpr::Ident {
                value: "⟩".to_string(),
            };
        }
        "lbrace" | "{" => {
            return MathExpr::Ident {
                value: "{".to_string(),
            };
        }
        "rbrace" | "}" => {
            return MathExpr::Ident {
                value: "}".to_string(),
            };
        }
        "lvert" | "|" => {
            return MathExpr::Ident {
                value: "|".to_string(),
            };
        }
        "rvert" => {
            return MathExpr::Ident {
                value: "|".to_string(),
            };
        }
        "lVert" => {
            return MathExpr::Ident {
                value: "‖".to_string(),
            };
        }
        "rVert" => {
            return MathExpr::Ident {
                value: "‖".to_string(),
            };
        }
        "lceil" => {
            return MathExpr::Ident {
                value: "⌈".to_string(),
            };
        }
        "rceil" => {
            return MathExpr::Ident {
                value: "⌉".to_string(),
            };
        }
        "lfloor" => {
            return MathExpr::Ident {
                value: "⌊".to_string(),
            };
        }
        "rfloor" => {
            return MathExpr::Ident {
                value: "⌋".to_string(),
            };
        }
        _ => {}
    }

    // ── Miscellaneous single-symbol commands ──
    match cmd {
        "ldots" | "dots" => {
            return MathExpr::Ident {
                value: "…".to_string(),
            };
        }
        "cdots" => {
            return MathExpr::Ident {
                value: "⋯".to_string(),
            };
        }
        "vdots" => {
            return MathExpr::Ident {
                value: "⋮".to_string(),
            };
        }
        "ddots" => {
            return MathExpr::Ident {
                value: "⋱".to_string(),
            };
        }
        "prime" => {
            return MathExpr::Ident {
                value: "′".to_string(),
            };
        }
        "circ" => {
            return MathExpr::Operator(MathOperator {
                symbol: "∘".to_string(),
                kind: OperatorKind::Binary,
            });
        }
        "bullet" => {
            return MathExpr::Operator(MathOperator {
                symbol: "•".to_string(),
                kind: OperatorKind::Binary,
            });
        }
        "star" => {
            return MathExpr::Operator(MathOperator {
                symbol: "⋆".to_string(),
                kind: OperatorKind::Binary,
            });
        }
        "perp" => {
            return MathExpr::Ident {
                value: "⊥".to_string(),
            };
        }
        "top" => {
            return MathExpr::Ident {
                value: "⊤".to_string(),
            };
        }
        "angle" => {
            return MathExpr::Ident {
                value: "∠".to_string(),
            };
        }
        "triangle" => {
            return MathExpr::Ident {
                value: "△".to_string(),
            };
        }
        "square" => {
            return MathExpr::Ident {
                value: "□".to_string(),
            };
        }
        "therefore" => {
            return MathExpr::Ident {
                value: "∴".to_string(),
            };
        }
        "because" => {
            return MathExpr::Ident {
                value: "∵".to_string(),
            };
        }
        "checkmark" => {
            return MathExpr::Ident {
                value: "✓".to_string(),
            };
        }
        _ => {}
    }

    // ── Unknown command → Error node (error recovery, never panic) ──
    MathExpr::Error {
        raw: format!("\\{}", cmd),
        message: format!("unknown command: \\{}", cmd),
    }
}

// ─── \left ... \right ─────────────────────────────────────────────────────────

/// Parse `\left<delim> ... \right<delim>`.
/// The `\left` command token has already been consumed.
fn parse_delimited(ts: &mut TokenStream) -> MathExpr {
    ts.skip_whitespace();
    let open = parse_delimiter_token(ts);

    let body_items = parse_row(ts);

    ts.skip_whitespace();

    // Expect \right
    let close = if matches!(ts.peek(), Token::Command(cmd) if cmd == "right") {
        ts.next(); // consume \right
        ts.skip_whitespace();
        parse_delimiter_token(ts)
    } else {
        // Missing \right — error recovery: return what we have wrapped in an Error
        return MathExpr::Error {
            raw: format!("\\left{}", delimiter_to_raw(open)),
            message: "\\left without matching \\right".to_string(),
        };
    };

    MathExpr::Fenced {
        open,
        close,
        body: body_items,
    }
}

/// Read the next token(s) to determine which `Delimiter` is meant.
fn parse_delimiter_token(ts: &mut TokenStream) -> Delimiter {
    match ts.peek().clone() {
        Token::LParen => {
            ts.next();
            Delimiter::Paren
        }
        Token::RParen => {
            ts.next();
            Delimiter::Paren
        }
        Token::LBracket => {
            ts.next();
            Delimiter::Bracket
        }
        Token::RBracket => {
            ts.next();
            Delimiter::Bracket
        }
        Token::Pipe => {
            ts.next();
            Delimiter::Pipe
        }
        Token::Command(cmd) => {
            ts.next();
            match cmd.as_str() {
                "{" | "lbrace" => Delimiter::Brace,
                "}" | "rbrace" => Delimiter::Brace,
                "langle" => Delimiter::Angle,
                "rangle" => Delimiter::Angle,
                "|" | "lVert" | "rVert" | "Vert" => Delimiter::DoublePipe,
                "lvert" | "rvert" | "vert" => Delimiter::Pipe,
                "lceil" | "rceil" => Delimiter::Ceil,
                "lfloor" | "rfloor" => Delimiter::Floor,
                "." => Delimiter::None, // invisible delimiter
                _ => Delimiter::None,
            }
        }
        Token::LBrace => {
            ts.next();
            Delimiter::Brace
        }
        Token::RBrace => {
            ts.next();
            Delimiter::Brace
        }
        _ => {
            // Something unexpected — treat as invisible
            Delimiter::None
        }
    }
}

fn delimiter_to_raw(d: Delimiter) -> &'static str {
    match d {
        Delimiter::Paren => "(",
        Delimiter::Bracket => "[",
        Delimiter::Brace => "\\{",
        Delimiter::Angle => "\\langle",
        Delimiter::Pipe => "|",
        Delimiter::DoublePipe => "\\|",
        Delimiter::Floor => "\\lfloor",
        Delimiter::Ceil => "\\lceil",
        Delimiter::None => ".",
    }
}

// ─── Environment dispatch ─────────────────────────────────────────────────────

/// Parse the body of `\begin{env}...\end{env}`.
/// The `\begin{env}` token has already been consumed.
pub(crate) fn parse_environment(ts: &mut TokenStream, env: &str) -> MathExpr {
    match env {
        // ── Matrices ──
        "matrix" => parse_matrix_env(ts, env, MatrixDelimiters::Plain),
        "pmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::Paren),
        "bmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::Bracket),
        "Bmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::Brace),
        "vmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::Pipe),
        "Vmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::DoublePipe),
        "smallmatrix" => parse_matrix_env(ts, env, MatrixDelimiters::Plain),

        // ── Cases ──
        "cases" | "cases*" => parse_cases_env(ts, env),

        // ── Alignment environments ──
        "align" | "align*" | "aligned" => parse_align_env(ts, env),
        "gather" | "gather*" | "gathered" => parse_gather_env(ts, env),
        "alignat" | "alignat*" => parse_align_env(ts, env),

        // ── Array ──
        "array" => parse_array_env(ts),

        // ── CD (commutative diagram) — Tier 3: emit error ──
        "CD" => {
            let raw = collect_until_end(ts, "CD");
            consume_end(ts, "CD");
            MathExpr::Error {
                raw: format!("\\begin{{CD}}{raw}\\end{{CD}}"),
                message: "commutative diagrams (\\begin{CD}) are not supported".to_string(),
            }
        }

        // ── Unknown environment ──
        _ => {
            let raw = collect_until_end(ts, env);
            consume_end(ts, env);
            MathExpr::Error {
                raw: format!("\\begin{{{env}}}{raw}\\end{{{env}}}"),
                message: format!("unknown environment: {env}"),
            }
        }
    }
}

// ─── Matrix parsing ───────────────────────────────────────────────────────────

fn parse_matrix_env(ts: &mut TokenStream, env: &str, delimiters: MatrixDelimiters) -> MathExpr {
    let rows = parse_matrix_body(ts, env);
    MathExpr::Matrix { rows, delimiters }
}

/// Parse a matrix body properly: rows delimited by `\\`, cells by `&`.
/// Assumes we are positioned right after `\begin{env}`.
fn parse_matrix_body(ts: &mut TokenStream, env: &str) -> Vec<Vec<MathExpr>> {
    let mut all_rows: Vec<Vec<MathExpr>> = Vec::new();
    let mut current_cells: Vec<MathExpr> = Vec::new();
    let mut current_cell: Vec<MathExpr> = Vec::new();

    loop {
        ts.skip_whitespace();
        match ts.peek().clone() {
            Token::Eof => break,
            Token::End(e) if e == env => {
                ts.next(); // consume \end{env}
                break;
            }
            Token::End(_) => break,
            Token::Ampersand => {
                ts.next();
                current_cells.push(flatten_row(std::mem::take(&mut current_cell)));
            }
            Token::DoubleBackslash => {
                ts.next();
                // Finish current cell
                current_cells.push(flatten_row(std::mem::take(&mut current_cell)));
                all_rows.push(std::mem::take(&mut current_cells));
            }
            _ => {
                let atom = parse_atom(ts);
                let scripted = parse_scripts(ts, atom);
                current_cell.push(scripted);
            }
        }
    }

    // Finish last cell and row
    current_cells.push(flatten_row(current_cell));
    if !current_cells.is_empty() {
        // Only add row if not entirely empty
        let non_empty = current_cells
            .iter()
            .any(|e| !matches!(e, MathExpr::Row { children: r } if r.is_empty()));
        if non_empty {
            all_rows.push(current_cells);
        }
    }

    all_rows
}

// ─── Cases parsing ────────────────────────────────────────────────────────────

fn parse_cases_env(ts: &mut TokenStream, env: &str) -> MathExpr {
    let mut rows: Vec<CaseRow> = Vec::new();
    let mut current_expr: Vec<MathExpr> = Vec::new();
    let mut current_cond: Option<Vec<MathExpr>> = None;
    let mut in_condition = false;

    loop {
        ts.skip_whitespace();
        match ts.peek().clone() {
            Token::Eof => break,
            Token::End(e) if e == env => {
                ts.next();
                break;
            }
            Token::End(_) => break,
            Token::Ampersand => {
                ts.next();
                // & separates expr from condition
                if !in_condition {
                    in_condition = true;
                    current_cond = Some(Vec::new());
                }
            }
            Token::DoubleBackslash => {
                ts.next();
                let cond = current_cond.take().map(flatten_row);
                rows.push(CaseRow {
                    expr: flatten_row(std::mem::take(&mut current_expr)),
                    condition: cond,
                });
                in_condition = false;
            }
            _ => {
                let atom = parse_atom(ts);
                let scripted = parse_scripts(ts, atom);
                if in_condition {
                    current_cond.get_or_insert_with(Vec::new).push(scripted);
                } else {
                    current_expr.push(scripted);
                }
            }
        }
    }

    // Last row
    let cond = current_cond.map(flatten_row);
    if !current_expr.is_empty() || cond.is_some() {
        rows.push(CaseRow {
            expr: flatten_row(current_expr),
            condition: cond,
        });
    }

    MathExpr::Cases { rows }
}

// ─── Align parsing ────────────────────────────────────────────────────────────

fn parse_align_env(ts: &mut TokenStream, env: &str) -> MathExpr {
    let rows = parse_align_rows(ts, env);
    let numbered = !env.ends_with('*');
    MathExpr::Align { rows, numbered }
}

fn parse_gather_env(ts: &mut TokenStream, env: &str) -> MathExpr {
    let align_rows = parse_align_rows(ts, env);
    let numbered = !env.ends_with('*');
    // Gather rows are Vec<MathExpr>: flatten each AlignRow's cells into a single MathExpr
    let rows: Vec<MathExpr> = align_rows
        .into_iter()
        .map(|row| flatten_row(row.cells))
        .collect();
    MathExpr::Gather { rows, numbered }
}

fn parse_align_rows(ts: &mut TokenStream, env: &str) -> Vec<AlignRow> {
    let mut result: Vec<AlignRow> = Vec::new();
    let mut current_cells: Vec<MathExpr> = Vec::new();
    let mut current_cell: Vec<MathExpr> = Vec::new();

    loop {
        ts.skip_whitespace();
        match ts.peek().clone() {
            Token::Eof => break,
            Token::End(e) if e == env => {
                ts.next();
                break;
            }
            Token::End(_) => break,
            Token::Ampersand => {
                ts.next();
                current_cells.push(flatten_row(std::mem::take(&mut current_cell)));
            }
            Token::DoubleBackslash => {
                ts.next();
                current_cells.push(flatten_row(std::mem::take(&mut current_cell)));
                result.push(AlignRow {
                    cells: std::mem::take(&mut current_cells),
                    label: None,
                });
            }
            Token::Command(cmd) if cmd == "label" => {
                // \label{...} — consume and store as label of current row
                ts.next();
                let label_body = parse_group(ts);
                let label = extract_text_content(&label_body);
                // We store it on the next \\ boundary; for simplicity attach to next row push
                current_cells.push(flatten_row(std::mem::take(&mut current_cell)));
                result.push(AlignRow {
                    cells: std::mem::take(&mut current_cells),
                    label: Some(label),
                });
            }
            _ => {
                let atom = parse_atom(ts);
                let scripted = parse_scripts(ts, atom);
                current_cell.push(scripted);
            }
        }
    }

    // Final row
    current_cells.push(flatten_row(current_cell));
    if current_cells
        .iter()
        .any(|e| !matches!(e, MathExpr::Row { children: r } if r.is_empty()))
    {
        result.push(AlignRow {
            cells: current_cells,
            label: None,
        });
    }

    result
}

// ─── Array parsing ────────────────────────────────────────────────────────────

fn parse_array_env(ts: &mut TokenStream) -> MathExpr {
    // First argument: column specification
    ts.skip_whitespace();
    let columns = if matches!(ts.peek(), Token::LBrace) {
        let spec_group = parse_group(ts);
        parse_column_spec(&extract_text_content(&spec_group))
    } else {
        Vec::new()
    };

    let rows = parse_matrix_body(ts, "array");
    MathExpr::Array { columns, rows }
}

fn parse_column_spec(spec: &str) -> Vec<ColumnAlign> {
    let mut result = Vec::new();
    for c in spec.chars() {
        match c {
            'l' => result.push(ColumnAlign::Left),
            'c' => result.push(ColumnAlign::Center),
            'r' => result.push(ColumnAlign::Right),
            _ => {} // skip | separators, etc.
        }
    }
    result
}

// ─── Style body helper ────────────────────────────────────────────────────────

/// For `\displaystyle`, etc., the "body" is everything that follows in the current row.
/// Normally \displaystyle applies to the rest of the current group. We parse the next
/// atom (or group) as the body.
fn parse_style_body(ts: &mut TokenStream) -> MathExpr {
    ts.skip_whitespace();
    // Consume the rest of the current group (or just next atom)
    if matches!(ts.peek(), Token::LBrace) {
        parse_group(ts)
    } else {
        // Take the rest of the row as body
        let items = parse_row(ts);
        flatten_row(items)
    }
}

// ─── Utility helpers ─────────────────────────────────────────────────────────

/// Consume tokens until `\end{env}`, returning collected raw text.
fn collect_until_end(ts: &mut TokenStream, env: &str) -> String {
    let mut raw = String::new();
    loop {
        match ts.peek().clone() {
            Token::Eof => break,
            Token::End(e) if e == env => break,
            tok => {
                raw.push_str(&token_to_raw(&tok));
                ts.next();
            }
        }
    }
    raw
}

fn consume_end(ts: &mut TokenStream, env: &str) {
    if matches!(ts.peek(), Token::End(e) if e == env) {
        ts.next();
    }
}

/// Parse tokens until `]` (for optional arguments like `\sqrt[...]`).
fn parse_until_rbracket(ts: &mut TokenStream) -> Vec<MathExpr> {
    let mut items = Vec::new();
    loop {
        ts.skip_whitespace();
        match ts.peek() {
            Token::RBracket | Token::Eof => break,
            _ => {
                let atom = parse_atom(ts);
                let scripted = parse_scripts(ts, atom);
                items.push(scripted);
            }
        }
    }
    items
}

/// Collect tokens until `]` as a raw string (for \smash[t]{...}).
fn collect_until_rbracket_str(ts: &mut TokenStream) -> String {
    let mut s = String::new();
    loop {
        match ts.peek().clone() {
            Token::RBracket | Token::Eof => break,
            tok => {
                s.push_str(&token_to_raw(&tok));
                ts.next();
            }
        }
    }
    s
}

/// If `items` has a single element, return it; otherwise wrap in `Row`.
fn flatten_row(items: Vec<MathExpr>) -> MathExpr {
    match items.len() {
        0 => MathExpr::Row {
            children: Vec::new(),
        },
        1 => items.into_iter().next().unwrap(),
        _ => MathExpr::Row { children: items },
    }
}

/// Convert a token to a raw LaTeX string fragment (for error messages).
fn token_to_raw(tok: &Token) -> String {
    match tok {
        Token::LBrace => "{".to_string(),
        Token::RBrace => "}".to_string(),
        Token::Caret => "^".to_string(),
        Token::Underscore => "_".to_string(),
        Token::Ampersand => "&".to_string(),
        Token::Tilde => "~".to_string(),
        Token::LParen => "(".to_string(),
        Token::RParen => ")".to_string(),
        Token::LBracket => "[".to_string(),
        Token::RBracket => "]".to_string(),
        Token::Pipe => "|".to_string(),
        Token::Plus => "+".to_string(),
        Token::Minus => "-".to_string(),
        Token::Equals => "=".to_string(),
        Token::LessThan => "<".to_string(),
        Token::GreaterThan => ">".to_string(),
        Token::Comma => ",".to_string(),
        Token::Semicolon => ";".to_string(),
        Token::Colon => ":".to_string(),
        Token::Bang => "!".to_string(),
        Token::Prime => "'".to_string(),
        Token::Dot => ".".to_string(),
        Token::Command(c) => format!("\\{c}"),
        Token::DoubleBackslash => "\\\\".to_string(),
        Token::ThinSpace => "\\,".to_string(),
        Token::MedSpace => "\\;".to_string(),
        Token::NegThinSpace => "\\!".to_string(),
        Token::Letter(c) => c.to_string(),
        Token::Digit(c) => c.to_string(),
        Token::Begin(e) => format!("\\begin{{{e}}}"),
        Token::End(e) => format!("\\end{{{e}}}"),
        Token::Whitespace => " ".to_string(),
        Token::Eof => String::new(),
    }
}

/// Extract a plain text string from a MathExpr tree (best-effort).
fn extract_text_content(expr: &MathExpr) -> String {
    match expr {
        MathExpr::Ident { value: s }
        | MathExpr::Number { value: s }
        | MathExpr::Text { value: s } => s.clone(),
        MathExpr::Row { children: items } => {
            items.iter().map(extract_text_content).collect::<String>()
        }
        MathExpr::Operator(op) => op.symbol.clone(),
        MathExpr::Space(_) => " ".to_string(),
        _ => String::new(),
    }
}

/// Convert a list of MathExpr back to a rough raw string (for error messages only).
fn exprs_to_raw(items: &[MathExpr]) -> String {
    items.iter().map(|_| "...").collect::<Vec<_>>().join("")
}

/// Map accent command names to AccentKind.
fn accent_kind(cmd: &str) -> Option<AccentKind> {
    match cmd {
        "hat" => Some(AccentKind::Hat),
        "widehat" => Some(AccentKind::WideHat),
        "tilde" => Some(AccentKind::Tilde),
        "widetilde" => Some(AccentKind::WideTilde),
        "vec" => Some(AccentKind::Vec),
        "dot" => Some(AccentKind::Dot),
        "ddot" => Some(AccentKind::Ddot),
        "bar" => Some(AccentKind::Bar),
        "acute" => Some(AccentKind::Acute),
        "grave" => Some(AccentKind::Grave),
        "breve" => Some(AccentKind::Breve),
        "check" => Some(AccentKind::Check),
        _ => None,
    }
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;
    use crate::tokenizer::{TokenStream, tokenize};

    fn parse(input: &str) -> MathExpr {
        let tokens = tokenize(input);
        let mut ts = TokenStream::new(tokens);
        parse_expr(&mut ts)
    }

    #[test]
    fn parse_single_letter() {
        let expr = parse("x");
        assert_eq!(
            expr,
            MathExpr::Ident {
                value: "x".to_string()
            }
        );
    }

    #[test]
    fn parse_number() {
        let expr = parse("42");
        assert_eq!(
            expr,
            MathExpr::Number {
                value: "42".to_string()
            }
        );
    }

    #[test]
    fn parse_decimal_number() {
        let expr = parse("3.14");
        assert_eq!(
            expr,
            MathExpr::Number {
                value: "3.14".to_string()
            }
        );
    }

    #[test]
    fn parse_superscript_braced() {
        let expr = parse("x^{2}");
        assert_eq!(
            expr,
            MathExpr::Superscript {
                base: Box::new(MathExpr::Ident {
                    value: "x".to_string()
                }),
                script: Box::new(MathExpr::Number {
                    value: "2".to_string()
                }),
            }
        );
    }

    #[test]
    fn parse_superscript_unbraced() {
        let expr = parse("x^2");
        assert_eq!(
            expr,
            MathExpr::Superscript {
                base: Box::new(MathExpr::Ident {
                    value: "x".to_string()
                }),
                script: Box::new(MathExpr::Number {
                    value: "2".to_string()
                }),
            }
        );
    }

    #[test]
    fn parse_subscript_unbraced() {
        let expr = parse("x_i");
        assert_eq!(
            expr,
            MathExpr::Subscript {
                base: Box::new(MathExpr::Ident {
                    value: "x".to_string()
                }),
                script: Box::new(MathExpr::Ident {
                    value: "i".to_string()
                }),
            }
        );
    }

    #[test]
    fn parse_sub_superscript() {
        let expr = parse("x_i^2");
        assert_eq!(
            expr,
            MathExpr::Subsuperscript {
                base: Box::new(MathExpr::Ident {
                    value: "x".to_string()
                }),
                sub: Box::new(MathExpr::Ident {
                    value: "i".to_string()
                }),
                sup: Box::new(MathExpr::Number {
                    value: "2".to_string()
                }),
            }
        );
    }

    #[test]
    fn parse_frac() {
        let expr = parse(r"\frac{a}{b}");
        assert_eq!(
            expr,
            MathExpr::Frac {
                numerator: Box::new(MathExpr::Ident {
                    value: "a".to_string()
                }),
                denominator: Box::new(MathExpr::Ident {
                    value: "b".to_string()
                }),
                style: FracStyle::Auto,
            }
        );
    }

    #[test]
    fn parse_dfrac() {
        let expr = parse(r"\dfrac{a}{b}");
        assert!(matches!(
            expr,
            MathExpr::Frac {
                style: FracStyle::Display,
                ..
            }
        ));
    }

    #[test]
    fn parse_sqrt() {
        let expr = parse(r"\sqrt{x}");
        assert_eq!(
            expr,
            MathExpr::Sqrt {
                index: None,
                body: Box::new(MathExpr::Ident {
                    value: "x".to_string()
                }),
            }
        );
    }

    #[test]
    fn parse_sqrt_with_index() {
        let expr = parse(r"\sqrt[3]{x}");
        assert!(matches!(expr, MathExpr::Sqrt { index: Some(_), .. }));
    }

    #[test]
    fn parse_greek_letter() {
        let expr = parse(r"\alpha");
        assert_eq!(
            expr,
            MathExpr::Ident {
                value: "α".to_string()
            }
        );
    }

    #[test]
    fn parse_infty() {
        let expr = parse(r"\infty");
        assert_eq!(
            expr,
            MathExpr::Ident {
                value: "∞".to_string()
            }
        );
    }

    #[test]
    fn parse_sum_with_limits() {
        let expr = parse(r"\sum_{i=0}^{n}");
        assert!(matches!(
            expr,
            MathExpr::BigOperator {
                lower: Some(_),
                upper: Some(_),
                ..
            }
        ));
    }

    #[test]
    fn parse_left_right_parens() {
        let expr = parse(r"\left( x \right)");
        assert!(matches!(
            expr,
            MathExpr::Fenced {
                open: Delimiter::Paren,
                close: Delimiter::Paren,
                ..
            }
        ));
    }

    #[test]
    fn parse_text_command() {
        let expr = parse(r"\text{hello}");
        assert_eq!(
            expr,
            MathExpr::Text {
                value: "hello".to_string()
            }
        );
    }

    #[test]
    fn parse_unknown_command_error() {
        let expr = parse(r"\unknowncmd");
        assert!(matches!(expr, MathExpr::Error { .. }));
    }

    #[test]
    fn parse_empty_input() {
        let expr = parse("");
        assert_eq!(
            expr,
            MathExpr::Row {
                children: Vec::new()
            }
        );
    }

    #[test]
    fn parse_spacing_commands() {
        let expr = parse(r"\quad");
        assert_eq!(expr, MathExpr::Space(MathSpace::Quad));
    }

    #[test]
    fn parse_thin_space() {
        let expr = parse(r"\,");
        assert_eq!(expr, MathExpr::Space(MathSpace::Thin));
    }

    #[test]
    fn parse_row_multiple_atoms() {
        let expr = parse("a+b");
        assert!(matches!(expr, MathExpr::Row { .. }));
    }

    #[test]
    fn parse_nested_frac() {
        let expr = parse(r"\frac{\frac{a}{b}}{c}");
        assert!(matches!(expr, MathExpr::Frac { .. }));
    }

    #[test]
    fn parse_mathbb() {
        let expr = parse(r"\mathbb{R}");
        assert!(matches!(
            expr,
            MathExpr::FontOverride {
                font: MathFont::Blackboard,
                ..
            }
        ));
    }

    #[test]
    fn parse_overline() {
        let expr = parse(r"\overline{x}");
        assert!(matches!(expr, MathExpr::Overline { .. }));
    }

    #[test]
    fn parse_hat_accent() {
        let expr = parse(r"\hat{x}");
        assert!(matches!(
            expr,
            MathExpr::Accent {
                kind: AccentKind::Hat,
                ..
            }
        ));
    }

    #[test]
    fn parse_leq_operator() {
        let expr = parse(r"\leq");
        assert!(matches!(
            expr,
            MathExpr::Operator(MathOperator {
                kind: OperatorKind::Relation,
                ..
            })
        ));
    }

    #[test]
    fn parse_frac_error_recovery() {
        // Unknown command inside a frac should produce Error node for inner, not crash the frac
        let expr = parse(r"\frac{a}{\unknowncmd}");
        assert!(matches!(expr, MathExpr::Frac { .. }));
        if let MathExpr::Frac { denominator, .. } = expr {
            assert!(matches!(*denominator, MathExpr::Error { .. }));
        }
    }

    #[test]
    fn parse_unmatched_brace_error() {
        // Missing closing brace — should produce Error, not panic
        let expr = parse(r"\frac{a}{b");
        // The parser should still produce a Frac (or Error), not panic
        // Just ensure no panic and we get something back
        let _ = expr;
    }
}
