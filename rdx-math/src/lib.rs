pub mod parser;
pub mod symbols;
/// `rdx-math` — LaTeX math parser for the RDX specification.
///
/// Parses LaTeX math strings (without `$` delimiters) into structured [`MathExpr`] trees.
///
/// # Example
///
/// ```rust
/// use rdx_math::parse;
///
/// let expr = parse(r"\frac{a}{b}");
/// ```
pub mod tokenizer;

use std::collections::HashMap;

// ─── Re-export rdx-ast types ─────────────────────────────────────────────────

pub use rdx_ast::{
    AccentKind, AlignRow, CaseRow, ColumnAlign, Delimiter, FracStyle, LimitStyle, MathExpr,
    MathFont, MathOperator, MathSpace, MathStyle, MatrixDelimiters, OperatorKind, SmashMode,
};

// ─── Macro definition ─────────────────────────────────────────────────────────

/// A user-defined LaTeX macro.
pub struct MacroDef {
    /// Number of arguments (0 for nullary macros).
    pub arity: u8,
    /// Template string using `#1`, `#2`, … as argument placeholders.
    pub template: String,
}

// ─── Public API ───────────────────────────────────────────────────────────────

/// Parse a LaTeX math string into a [`MathExpr`] tree.
///
/// The input must not include the surrounding `$` delimiters.
///
/// Any unrecognised constructs are wrapped in [`MathExpr::Error`] nodes rather than
/// causing a panic.
pub fn parse(input: &str) -> MathExpr {
    let tokens = tokenizer::tokenize(input);
    let mut ts = tokenizer::TokenStream::new(tokens);
    parser::parse_expr(&mut ts)
}

/// Parse a LaTeX math string with user-defined macro expansion.
///
/// Macros are expanded before parsing. The `macros` map keys must include the backslash
/// (e.g., `"\\R"`). Each expansion step decrements an internal depth counter; if the
/// counter reaches zero the expansion is aborted and an [`MathExpr::Error`] is returned
/// for the affected expression.
pub fn parse_with_macros(input: &str, macros: &HashMap<String, MacroDef>) -> MathExpr {
    match expand_macros(input, macros, 64) {
        Ok(expanded) => parse(&expanded),
        Err(msg) => MathExpr::Error {
            raw: input.to_string(),
            message: msg,
        },
    }
}

// ─── Macro expansion ──────────────────────────────────────────────────────────

/// Expand all macros in `input`, up to `max_depth` recursion levels.
fn expand_macros(
    input: &str,
    macros: &HashMap<String, MacroDef>,
    max_depth: usize,
) -> Result<String, String> {
    if max_depth == 0 {
        return Err(
            "macro expansion depth limit (64) exceeded — possible infinite loop".to_string(),
        );
    }

    let mut result = String::with_capacity(input.len());
    let chars: Vec<char> = input.chars().collect();
    let n = chars.len();
    let mut i = 0;

    while i < n {
        if chars[i] != '\\' {
            result.push(chars[i]);
            i += 1;
            continue;
        }

        // We have a backslash.  Try to match a macro name.
        let macro_start = i;
        i += 1; // skip '\'

        if i >= n {
            result.push('\\');
            continue;
        }

        // Collect the command name
        let name_start = i;
        if chars[i].is_ascii_alphabetic() {
            while i < n && chars[i].is_ascii_alphabetic() {
                i += 1;
            }
        } else {
            // Single non-alpha symbol command
            i += 1;
        }
        let cmd_name: String = chars[name_start..i].iter().collect();
        let full_name = format!("\\{cmd_name}");

        if let Some(def) = macros.get(&full_name) {
            // Collect arguments
            let mut args: Vec<String> = Vec::new();
            let mut j = i;

            for _ in 0..def.arity {
                // Skip whitespace
                while j < n && chars[j].is_ascii_whitespace() {
                    j += 1;
                }
                if j >= n {
                    break;
                }
                if chars[j] == '{' {
                    // Brace-delimited argument
                    j += 1; // skip {
                    let arg_start = j;
                    let mut depth = 1usize;
                    while j < n && depth > 0 {
                        match chars[j] {
                            '{' => depth += 1,
                            '}' => depth -= 1,
                            _ => {}
                        }
                        if depth > 0 {
                            j += 1;
                        } else {
                            // closing brace
                            break;
                        }
                    }
                    let arg: String = chars[arg_start..j].iter().collect();
                    if j < n && chars[j] == '}' {
                        j += 1; // skip closing }
                    }
                    args.push(arg);
                } else {
                    // Single character argument
                    args.push(chars[j].to_string());
                    j += 1;
                }
            }
            i = j;

            // Substitute arguments into template
            let mut expansion = def.template.clone();
            for (k, arg) in args.iter().enumerate() {
                let placeholder = format!("#{}", k + 1);
                expansion = expansion.replace(&placeholder, arg);
            }

            // Recursively expand the expansion
            let sub = expand_macros(&expansion, macros, max_depth - 1)?;
            result.push_str(&sub);
        } else {
            // Not a macro — emit verbatim
            let raw: String = chars[macro_start..i].iter().collect();
            result.push_str(&raw);
        }
    }

    Ok(result)
}

// ─── Tests ────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn simple_fraction() {
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
    fn superscript() {
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
    fn subscript_superscript() {
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
    fn sum_with_limits() {
        let expr = parse(r"\sum_{i=0}^{n} a_i");
        assert!(matches!(
            expr,
            MathExpr::Row { .. } // Row containing BigOperator and subscript
        ));
    }

    #[test]
    fn nested_fractions() {
        let expr = parse(r"\frac{\frac{a}{b}}{c}");
        assert!(matches!(expr, MathExpr::Frac { .. }));
        if let MathExpr::Frac { numerator, .. } = &expr {
            assert!(matches!(**numerator, MathExpr::Frac { .. }));
        }
    }

    #[test]
    fn left_right_delimiters() {
        let expr = parse(r"\left( \frac{a}{b} \right)");
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
    fn sqrt_with_index() {
        let expr = parse(r"\sqrt[3]{x}");
        assert!(matches!(expr, MathExpr::Sqrt { index: Some(_), .. }));
    }

    #[test]
    fn unknown_command_error_recovery() {
        let expr = parse(r"\frac{a}{\unknowncmd}");
        // Should have Frac structure
        assert!(matches!(expr, MathExpr::Frac { .. }));
        // Denominator should be an Error node
        if let MathExpr::Frac { denominator, .. } = expr {
            assert!(
                matches!(*denominator, MathExpr::Error { .. }),
                "expected Error node for unknown command, got {:?}",
                *denominator
            );
        }
    }

    #[test]
    fn greek_letters() {
        let alpha = parse(r"\alpha");
        assert_eq!(
            alpha,
            MathExpr::Ident {
                value: "α".to_string()
            }
        );

        let beta = parse(r"\beta");
        assert_eq!(
            beta,
            MathExpr::Ident {
                value: "β".to_string()
            }
        );

        // Expression: \alpha + \beta  (a Row)
        let expr = parse(r"\alpha + \beta");
        assert!(matches!(expr, MathExpr::Row { .. }));
    }

    #[test]
    fn text_in_math() {
        let expr = parse(r"\text{hello world}");
        assert_eq!(
            expr,
            MathExpr::Text {
                value: "hello world".to_string()
            }
        );
    }

    #[test]
    fn macro_expansion_nullary() {
        let mut macros = HashMap::new();
        macros.insert(
            "\\R".to_string(),
            MacroDef {
                arity: 0,
                template: "\\mathbb{R}".to_string(),
            },
        );
        let expr = parse_with_macros(r"x \in \R", &macros);
        // Should parse as Row containing Ident("x"), Operator("∈"), FontOverride(Blackboard, Ident("R"))
        assert!(matches!(expr, MathExpr::Row { .. }));
        if let MathExpr::Row { children } = &expr {
            let last = children.last().unwrap();
            assert!(
                matches!(
                    last,
                    MathExpr::FontOverride {
                        font: MathFont::Blackboard,
                        ..
                    }
                ),
                "expected FontOverride(Blackboard, ...), got {:?}",
                last
            );
        }
    }

    #[test]
    fn macro_expansion_with_arg() {
        let mut macros = HashMap::new();
        macros.insert(
            "\\norm".to_string(),
            MacroDef {
                arity: 1,
                template: "\\left\\lVert #1 \\right\\rVert".to_string(),
            },
        );
        let expr = parse_with_macros(r"\norm{x+y}", &macros);
        // Should expand to \left\lVert x+y \right\rVert → Fenced(DoublePipe, ...)
        assert!(
            matches!(expr, MathExpr::Fenced { .. }),
            "expected Fenced, got {:?}",
            expr
        );
    }

    #[test]
    fn empty_input() {
        let expr = parse("");
        assert_eq!(
            expr,
            MathExpr::Row {
                children: Vec::new()
            }
        );
    }

    #[test]
    fn spacing_commands() {
        let thin = parse(r"\,");
        assert_eq!(thin, MathExpr::Space(MathSpace::Thin));

        let quad = parse(r"\quad");
        assert_eq!(quad, MathExpr::Space(MathSpace::Quad));

        // Combined: a \, b \quad c  → Row
        let expr = parse(r"a \, b \quad c");
        assert!(matches!(expr, MathExpr::Row { .. }));
    }

    #[test]
    fn relational_operators() {
        let leq = parse(r"\leq");
        assert!(matches!(
            leq,
            MathExpr::Operator(MathOperator {
                kind: OperatorKind::Relation,
                ..
            })
        ));

        let neq = parse(r"\neq");
        assert!(matches!(
            neq,
            MathExpr::Operator(MathOperator {
                kind: OperatorKind::Relation,
                ..
            })
        ));
    }

    #[test]
    fn sum_with_sub_and_sup() {
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
    fn macro_expansion_depth_limit() {
        // A self-recursive macro should not infinite loop
        let mut macros = HashMap::new();
        macros.insert(
            "\\bad".to_string(),
            MacroDef {
                arity: 0,
                template: "\\bad".to_string(),
            },
        );
        let expr = parse_with_macros(r"\bad", &macros);
        // Should produce an Error (depth exceeded), not panic
        assert!(
            matches!(expr, MathExpr::Error { .. }),
            "expected Error for infinite macro, got {:?}",
            expr
        );
    }

    #[test]
    fn all_greek_lowercase() {
        let letters = [
            "alpha",
            "beta",
            "gamma",
            "delta",
            "epsilon",
            "varepsilon",
            "zeta",
            "eta",
            "theta",
            "vartheta",
            "iota",
            "kappa",
            "lambda",
            "mu",
            "nu",
            "xi",
            "pi",
            "varpi",
            "rho",
            "varrho",
            "sigma",
            "varsigma",
            "tau",
            "upsilon",
            "phi",
            "varphi",
            "chi",
            "psi",
            "omega",
        ];
        for name in &letters {
            let expr = parse(&format!("\\{name}"));
            assert!(
                matches!(expr, MathExpr::Ident { .. }),
                "\\{name} should be Ident, got {:?}",
                expr
            );
        }
    }

    #[test]
    fn all_greek_uppercase() {
        let letters = [
            "Gamma", "Delta", "Theta", "Lambda", "Xi", "Pi", "Sigma", "Upsilon", "Phi", "Psi",
            "Omega",
        ];
        for name in &letters {
            let expr = parse(&format!("\\{name}"));
            assert!(
                matches!(expr, MathExpr::Ident { .. }),
                "\\{name} should be Ident, got {:?}",
                expr
            );
        }
    }

    #[test]
    fn all_tier1_operators() {
        let ops = [
            r"\times",
            r"\cdot",
            r"\pm",
            r"\mp",
            r"\div",
            r"\neq",
            r"\leq",
            r"\geq",
            r"\approx",
            r"\equiv",
            r"\sim",
            r"\cong",
            r"\propto",
            r"\in",
            r"\notin",
            r"\subset",
            r"\supset",
            r"\cup",
            r"\cap",
            r"\land",
            r"\lor",
            r"\neg",
            r"\implies",
            r"\iff",
        ];
        for op in &ops {
            let expr = parse(op);
            assert!(
                matches!(expr, MathExpr::Operator(_)),
                "{op} should be Operator, got {:?}",
                expr
            );
        }
    }

    #[test]
    fn all_large_operators() {
        let ops = [
            r"\sum", r"\prod", r"\int", r"\iint", r"\iiint", r"\oint", r"\bigcup", r"\bigcap",
        ];
        for op in &ops {
            let expr = parse(op);
            assert!(
                matches!(expr, MathExpr::BigOperator { .. }),
                "{op} should be BigOperator, got {:?}",
                expr
            );
        }
    }

    #[test]
    fn frac_styles() {
        let auto = parse(r"\frac{1}{2}");
        assert!(matches!(
            auto,
            MathExpr::Frac {
                style: FracStyle::Auto,
                ..
            }
        ));

        let display = parse(r"\dfrac{1}{2}");
        assert!(matches!(
            display,
            MathExpr::Frac {
                style: FracStyle::Display,
                ..
            }
        ));

        let text = parse(r"\tfrac{1}{2}");
        assert!(matches!(
            text,
            MathExpr::Frac {
                style: FracStyle::Text,
                ..
            }
        ));
    }

    #[test]
    fn delimiter_variants() {
        let paren = parse(r"\left( x \right)");
        assert!(matches!(
            paren,
            MathExpr::Fenced {
                open: Delimiter::Paren,
                close: Delimiter::Paren,
                ..
            }
        ));

        let bracket = parse(r"\left[ x \right]");
        assert!(matches!(
            bracket,
            MathExpr::Fenced {
                open: Delimiter::Bracket,
                close: Delimiter::Bracket,
                ..
            }
        ));

        let brace = parse(r"\left\{ x \right\}");
        assert!(matches!(
            brace,
            MathExpr::Fenced {
                open: Delimiter::Brace,
                close: Delimiter::Brace,
                ..
            }
        ));

        let angle = parse(r"\left\langle x \right\rangle");
        assert!(matches!(
            angle,
            MathExpr::Fenced {
                open: Delimiter::Angle,
                close: Delimiter::Angle,
                ..
            }
        ));
    }

    #[test]
    fn invisible_delimiter() {
        let expr = parse(r"\left. x \right|");
        assert!(matches!(
            expr,
            MathExpr::Fenced {
                open: Delimiter::None,
                close: Delimiter::Pipe,
                ..
            }
        ));
    }

    #[test]
    fn partial_and_nabla() {
        let partial = parse(r"\partial");
        assert_eq!(
            partial,
            MathExpr::Ident {
                value: "∂".to_string()
            }
        );

        let nabla = parse(r"\nabla");
        assert_eq!(
            nabla,
            MathExpr::Ident {
                value: "∇".to_string()
            }
        );
    }

    #[test]
    fn mathrm_produces_font_override() {
        let expr = parse(r"\mathrm{d}");
        // \mathrm is font Roman, but for a single letter we want FontOverride
        assert!(
            matches!(
                expr,
                MathExpr::FontOverride {
                    font: MathFont::Roman,
                    ..
                }
            ),
            "expected FontOverride(Roman), got {:?}",
            expr
        );
    }

    #[test]
    fn tier2_accent_commands() {
        let accents = [
            (r"\hat{x}", AccentKind::Hat),
            (r"\tilde{x}", AccentKind::Tilde),
            (r"\vec{x}", AccentKind::Vec),
            (r"\dot{x}", AccentKind::Dot),
            (r"\ddot{x}", AccentKind::Ddot),
            (r"\bar{x}", AccentKind::Bar),
        ];
        for (input, expected_kind) in accents {
            let expr = parse(input);
            assert!(
                matches!(&expr, MathExpr::Accent { kind, .. } if *kind == expected_kind),
                "{input} should be Accent({:?}), got {:?}",
                expected_kind,
                expr
            );
        }
    }

    #[test]
    fn tier2_over_under() {
        let ol = parse(r"\overline{x}");
        assert!(matches!(ol, MathExpr::Overline { .. }));

        let ul = parse(r"\underline{x}");
        assert!(matches!(ul, MathExpr::Underline { .. }));

        let ob = parse(r"\overbrace{x}");
        assert!(matches!(ob, MathExpr::Overbrace { .. }));

        let ub = parse(r"\underbrace{x}");
        assert!(matches!(ub, MathExpr::Underbrace { .. }));
    }

    #[test]
    fn pmatrix_environment() {
        let expr = parse(r"\begin{pmatrix} a & b \\ c & d \end{pmatrix}");
        assert!(
            matches!(
                expr,
                MathExpr::Matrix {
                    delimiters: MatrixDelimiters::Paren,
                    ..
                }
            ),
            "expected pmatrix, got {:?}",
            expr
        );
    }

    #[test]
    fn cases_environment() {
        let expr = parse(r"\begin{cases} x & x > 0 \\ -x & x \leq 0 \end{cases}");
        assert!(
            matches!(expr, MathExpr::Cases { .. }),
            "expected Cases, got {:?}",
            expr
        );
    }

    #[test]
    fn align_environment() {
        let expr = parse(r"\begin{align} x &= 1 \\ y &= 2 \end{align}");
        assert!(
            matches!(expr, MathExpr::Align { .. }),
            "expected Align, got {:?}",
            expr
        );
    }

    #[test]
    fn unknown_environment_error() {
        let expr = parse(r"\begin{unknownenv} x \end{unknownenv}");
        assert!(
            matches!(expr, MathExpr::Error { .. }),
            "expected Error, got {:?}",
            expr
        );
    }

    #[test]
    fn never_panics_on_malformed() {
        // These should all produce results without panicking
        let inputs = [
            r"\frac{}{",        // missing }
            r"\frac{}",         // missing second arg
            r"\sqrt[",          // unclosed optional
            r"\left(",          // missing \right
            r"^{x}",            // dangling script
            r"\begin{pmatrix}", // missing \end
            r"\color{red}",     // missing body
        ];
        for input in inputs {
            let _ = parse(input);
        }
    }
}
