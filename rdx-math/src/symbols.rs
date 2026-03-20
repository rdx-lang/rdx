/// Lookup tables for Greek letters, operators, large operators, and named functions.
use crate::{MathFont, OperatorKind};

/// Returns the Unicode character for a Greek letter command name, or `None` if not recognised.
pub(crate) fn greek_letter(name: &str) -> Option<&'static str> {
    match name {
        // Lowercase
        "alpha" => Some("α"),
        "beta" => Some("β"),
        "gamma" => Some("γ"),
        "delta" => Some("δ"),
        "epsilon" => Some("ε"),
        "varepsilon" => Some("ε"),
        "zeta" => Some("ζ"),
        "eta" => Some("η"),
        "theta" => Some("θ"),
        "vartheta" => Some("ϑ"),
        "iota" => Some("ι"),
        "kappa" => Some("κ"),
        "lambda" => Some("λ"),
        "mu" => Some("μ"),
        "nu" => Some("ν"),
        "xi" => Some("ξ"),
        "pi" => Some("π"),
        "varpi" => Some("ϖ"),
        "rho" => Some("ρ"),
        "varrho" => Some("ϱ"),
        "sigma" => Some("σ"),
        "varsigma" => Some("ς"),
        "tau" => Some("τ"),
        "upsilon" => Some("υ"),
        "phi" => Some("φ"),
        "varphi" => Some("φ"),
        "chi" => Some("χ"),
        "psi" => Some("ψ"),
        "omega" => Some("ω"),
        // Uppercase
        "Gamma" => Some("Γ"),
        "Delta" => Some("Δ"),
        "Theta" => Some("Θ"),
        "Lambda" => Some("Λ"),
        "Xi" => Some("Ξ"),
        "Pi" => Some("Π"),
        "Sigma" => Some("Σ"),
        "Upsilon" => Some("Υ"),
        "Phi" => Some("Φ"),
        "Psi" => Some("Ψ"),
        "Omega" => Some("Ω"),
        _ => None,
    }
}

/// Returns `(symbol, OperatorKind)` for a binary/relation/set/logic operator command, or `None`.
pub(crate) fn operator(name: &str) -> Option<(&'static str, OperatorKind)> {
    match name {
        // Binary
        "times" => Some(("×", OperatorKind::Binary)),
        "cdot" => Some(("·", OperatorKind::Binary)),
        "pm" => Some(("±", OperatorKind::Binary)),
        "mp" => Some(("∓", OperatorKind::Binary)),
        "div" => Some(("÷", OperatorKind::Binary)),
        // Relation
        "neq" | "ne" => Some(("≠", OperatorKind::Relation)),
        "leq" | "le" => Some(("≤", OperatorKind::Relation)),
        "geq" | "ge" => Some(("≥", OperatorKind::Relation)),
        "approx" => Some(("≈", OperatorKind::Relation)),
        "equiv" => Some(("≡", OperatorKind::Relation)),
        "sim" => Some(("∼", OperatorKind::Relation)),
        "cong" => Some(("≅", OperatorKind::Relation)),
        "propto" => Some(("∝", OperatorKind::Relation)),
        // Set
        "in" => Some(("∈", OperatorKind::Relation)),
        "notin" => Some(("∉", OperatorKind::Relation)),
        "subset" => Some(("⊂", OperatorKind::Relation)),
        "supset" => Some(("⊃", OperatorKind::Relation)),
        "subseteq" => Some(("⊆", OperatorKind::Relation)),
        "supseteq" => Some(("⊇", OperatorKind::Relation)),
        "cup" => Some(("∪", OperatorKind::Binary)),
        "cap" => Some(("∩", OperatorKind::Binary)),
        // Logic
        "land" => Some(("∧", OperatorKind::Binary)),
        "lor" => Some(("∨", OperatorKind::Binary)),
        "neg" | "lnot" => Some(("¬", OperatorKind::Prefix)),
        "implies" => Some(("⟹", OperatorKind::Relation)),
        "iff" => Some(("⟺", OperatorKind::Relation)),
        // Arrows (Tier 2 but include here for operator classification)
        "to" | "rightarrow" => Some(("→", OperatorKind::Relation)),
        "leftarrow" => Some(("←", OperatorKind::Relation)),
        "Rightarrow" => Some(("⇒", OperatorKind::Relation)),
        "Leftarrow" => Some(("⇐", OperatorKind::Relation)),
        "leftrightarrow" => Some(("↔", OperatorKind::Relation)),
        "Leftrightarrow" => Some(("⟺", OperatorKind::Relation)),
        "mapsto" => Some(("↦", OperatorKind::Relation)),
        "hookrightarrow" => Some(("↪", OperatorKind::Relation)),
        "hookleftarrow" => Some(("↩", OperatorKind::Relation)),
        "uparrow" => Some(("↑", OperatorKind::Relation)),
        "downarrow" => Some(("↓", OperatorKind::Relation)),
        // Dots (Tier 2)
        "dots" | "ldots" => Some(("…", OperatorKind::Punctuation)),
        "cdots" => Some(("⋯", OperatorKind::Punctuation)),
        "vdots" => Some(("⋮", OperatorKind::Punctuation)),
        "ddots" => Some(("⋱", OperatorKind::Punctuation)),
        // Punctuation
        "colon" => Some((":", OperatorKind::Punctuation)),
        _ => None,
    }
}

/// Returns `(symbol, true)` if this command is a large operator (BigOperator).
/// The bool is a placeholder; we always return `OperatorKind::Large` for these.
pub(crate) fn large_operator(name: &str) -> Option<&'static str> {
    match name {
        "sum" => Some("∑"),
        "prod" => Some("∏"),
        "int" => Some("∫"),
        "iint" => Some("∬"),
        "iiint" => Some("∭"),
        "oint" => Some("∮"),
        "bigcup" => Some("⋃"),
        "bigcap" => Some("⋂"),
        "bigoplus" => Some("⊕"),
        "bigotimes" => Some("⊗"),
        "bigsqcup" => Some("⊔"),
        "biguplus" => Some("⊎"),
        "bigvee" => Some("⋁"),
        "bigwedge" => Some("⋀"),
        _ => None,
    }
}

/// Named function operators (upright font, not identifiers).
/// Returns the display string for \lim, \sin, etc.
pub(crate) fn named_operator(name: &str) -> Option<&'static str> {
    match name {
        "lim" => Some("lim"),
        "max" => Some("max"),
        "min" => Some("min"),
        "sup" => Some("sup"),
        "inf" => Some("inf"),
        "log" => Some("log"),
        "ln" => Some("ln"),
        "sin" => Some("sin"),
        "cos" => Some("cos"),
        "tan" => Some("tan"),
        "sec" => Some("sec"),
        "csc" => Some("csc"),
        "cot" => Some("cot"),
        "arcsin" => Some("arcsin"),
        "arccos" => Some("arccos"),
        "arctan" => Some("arctan"),
        "exp" => Some("exp"),
        "det" => Some("det"),
        "gcd" => Some("gcd"),
        "arg" => Some("arg"),
        "dim" => Some("dim"),
        "ker" => Some("ker"),
        "deg" => Some("deg"),
        "hom" => Some("hom"),
        "Pr" => Some("Pr"),
        "liminf" => Some("lim inf"),
        "limsup" => Some("lim sup"),
        _ => None,
    }
}

/// Returns the MathFont for a font-override command.
pub(crate) fn font_override_command(name: &str) -> Option<MathFont> {
    match name {
        "mathbb" => Some(MathFont::Blackboard),
        "mathcal" => Some(MathFont::Calligraphic),
        "mathfrak" => Some(MathFont::Fraktur),
        "mathscr" => Some(MathFont::Script),
        "mathbf" | "boldsymbol" => Some(MathFont::Bold),
        "mathsf" => Some(MathFont::SansSerif),
        "mathtt" => Some(MathFont::Monospace),
        "mathit" | "textit" => Some(MathFont::Italic),
        "mathrm" | "textrm" | "text" | "mbox" => Some(MathFont::Roman),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greek_lowercase_known() {
        assert_eq!(greek_letter("alpha"), Some("α"));
        assert_eq!(greek_letter("omega"), Some("ω"));
    }

    #[test]
    fn greek_uppercase_known() {
        assert_eq!(greek_letter("Gamma"), Some("Γ"));
        assert_eq!(greek_letter("Omega"), Some("Ω"));
    }

    #[test]
    fn greek_unknown() {
        assert_eq!(greek_letter("notgreek"), None);
    }

    #[test]
    fn operator_binary() {
        let (sym, kind) = operator("times").unwrap();
        assert_eq!(sym, "×");
        assert!(matches!(kind, OperatorKind::Binary));
    }

    #[test]
    fn operator_relation() {
        let (sym, kind) = operator("leq").unwrap();
        assert_eq!(sym, "≤");
        assert!(matches!(kind, OperatorKind::Relation));
    }

    #[test]
    fn large_operator_sum() {
        assert_eq!(large_operator("sum"), Some("∑"));
    }

    #[test]
    fn named_operator_sin() {
        assert_eq!(named_operator("sin"), Some("sin"));
    }

    #[test]
    fn font_mathbb() {
        assert!(matches!(
            font_override_command("mathbb"),
            Some(MathFont::Blackboard)
        ));
    }
}
