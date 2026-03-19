// src/math/symbols.rs - LaTeX symbol-to-Unicode mapping table

/// Accent kinds supported in math mode.
#[derive(Debug, Clone, PartialEq)]
pub enum AccentKind {
    Hat,
    Bar,
    Vec,
    Dot,
    Tilde,
}

impl AccentKind {
    /// Returns the Unicode combining character for this accent.
    pub fn combining_char(&self) -> char {
        match self {
            AccentKind::Hat => '\u{0302}',   // COMBINING CIRCUMFLEX ACCENT
            AccentKind::Bar => '\u{0304}',   // COMBINING MACRON
            AccentKind::Vec => '\u{20D7}',   // COMBINING RIGHT ARROW ABOVE
            AccentKind::Dot => '\u{0307}',   // COMBINING DOT ABOVE
            AccentKind::Tilde => '\u{0303}', // COMBINING TILDE
        }
    }
}

/// Maps a LaTeX command name to a Unicode character.
/// Returns `None` if the command is not in the table.
pub fn lookup_symbol(name: &str) -> Option<char> {
    match name {
        // Greek lowercase
        "alpha" => Some('α'),
        "beta" => Some('β'),
        "gamma" => Some('γ'),
        "delta" => Some('δ'),
        "epsilon" => Some('ε'),
        "zeta" => Some('ζ'),
        "eta" => Some('η'),
        "theta" => Some('θ'),
        "iota" => Some('ι'),
        "kappa" => Some('κ'),
        "lambda" => Some('λ'),
        "mu" => Some('μ'),
        "nu" => Some('ν'),
        "xi" => Some('ξ'),
        "pi" => Some('π'),
        "rho" => Some('ρ'),
        "sigma" => Some('σ'),
        "tau" => Some('τ'),
        "upsilon" => Some('υ'),
        "phi" => Some('φ'),
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
        // Large operators
        "sum" => Some('∑'),
        "prod" => Some('∏'),
        "int" => Some('∫'),
        // Relations
        "leq" => Some('≤'),
        "geq" => Some('≥'),
        "neq" => Some('≠'),
        "approx" => Some('≈'),
        "equiv" => Some('≡'),
        "sim" => Some('∼'),
        // Miscellaneous
        "infty" => Some('∞'),
        "partial" => Some('∂'),
        "nabla" => Some('∇'),
        "pm" => Some('±'),
        "mp" => Some('∓'),
        "times" => Some('×'),
        "cdot" => Some('·'),
        "ldots" => Some('…'),
        _ => None,
    }
}

/// Maps a LaTeX accent command name to an `AccentKind`.
/// Returns `None` if the command is not a known accent.
pub fn lookup_accent(name: &str) -> Option<AccentKind> {
    match name {
        "hat" => Some(AccentKind::Hat),
        "bar" => Some(AccentKind::Bar),
        "vec" => Some(AccentKind::Vec),
        "dot" => Some(AccentKind::Dot),
        "tilde" => Some(AccentKind::Tilde),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greek_lowercase() {
        assert_eq!(lookup_symbol("alpha"), Some('α'));
        assert_eq!(lookup_symbol("beta"), Some('β'));
        assert_eq!(lookup_symbol("gamma"), Some('γ'));
        assert_eq!(lookup_symbol("delta"), Some('δ'));
        assert_eq!(lookup_symbol("epsilon"), Some('ε'));
        assert_eq!(lookup_symbol("zeta"), Some('ζ'));
        assert_eq!(lookup_symbol("eta"), Some('η'));
        assert_eq!(lookup_symbol("theta"), Some('θ'));
        assert_eq!(lookup_symbol("iota"), Some('ι'));
        assert_eq!(lookup_symbol("kappa"), Some('κ'));
        assert_eq!(lookup_symbol("lambda"), Some('λ'));
        assert_eq!(lookup_symbol("mu"), Some('μ'));
        assert_eq!(lookup_symbol("nu"), Some('ν'));
        assert_eq!(lookup_symbol("xi"), Some('ξ'));
        assert_eq!(lookup_symbol("pi"), Some('π'));
        assert_eq!(lookup_symbol("rho"), Some('ρ'));
        assert_eq!(lookup_symbol("sigma"), Some('σ'));
        assert_eq!(lookup_symbol("tau"), Some('τ'));
        assert_eq!(lookup_symbol("upsilon"), Some('υ'));
        assert_eq!(lookup_symbol("phi"), Some('φ'));
        assert_eq!(lookup_symbol("chi"), Some('χ'));
        assert_eq!(lookup_symbol("psi"), Some('ψ'));
        assert_eq!(lookup_symbol("omega"), Some('ω'));
    }

    #[test]
    fn test_greek_uppercase() {
        assert_eq!(lookup_symbol("Gamma"), Some('Γ'));
        assert_eq!(lookup_symbol("Delta"), Some('Δ'));
        assert_eq!(lookup_symbol("Theta"), Some('Θ'));
        assert_eq!(lookup_symbol("Lambda"), Some('Λ'));
        assert_eq!(lookup_symbol("Xi"), Some('Ξ'));
        assert_eq!(lookup_symbol("Pi"), Some('Π'));
        assert_eq!(lookup_symbol("Sigma"), Some('Σ'));
        assert_eq!(lookup_symbol("Upsilon"), Some('Υ'));
        assert_eq!(lookup_symbol("Phi"), Some('Φ'));
        assert_eq!(lookup_symbol("Psi"), Some('Ψ'));
        assert_eq!(lookup_symbol("Omega"), Some('Ω'));
    }

    #[test]
    fn test_large_operators() {
        assert_eq!(lookup_symbol("sum"), Some('∑'));
        assert_eq!(lookup_symbol("prod"), Some('∏'));
        assert_eq!(lookup_symbol("int"), Some('∫'));
    }

    #[test]
    fn test_relations() {
        assert_eq!(lookup_symbol("leq"), Some('≤'));
        assert_eq!(lookup_symbol("geq"), Some('≥'));
        assert_eq!(lookup_symbol("neq"), Some('≠'));
        assert_eq!(lookup_symbol("approx"), Some('≈'));
        assert_eq!(lookup_symbol("equiv"), Some('≡'));
        assert_eq!(lookup_symbol("sim"), Some('∼'));
    }

    #[test]
    fn test_misc_symbols() {
        assert_eq!(lookup_symbol("infty"), Some('∞'));
        assert_eq!(lookup_symbol("partial"), Some('∂'));
        assert_eq!(lookup_symbol("nabla"), Some('∇'));
        assert_eq!(lookup_symbol("pm"), Some('±'));
        assert_eq!(lookup_symbol("mp"), Some('∓'));
        assert_eq!(lookup_symbol("times"), Some('×'));
        assert_eq!(lookup_symbol("cdot"), Some('·'));
        assert_eq!(lookup_symbol("ldots"), Some('…'));
    }

    #[test]
    fn test_unknown_symbol() {
        assert_eq!(lookup_symbol("unknown"), None);
        assert_eq!(lookup_symbol(""), None);
    }

    #[test]
    fn test_accents() {
        assert_eq!(lookup_accent("hat"), Some(AccentKind::Hat));
        assert_eq!(lookup_accent("bar"), Some(AccentKind::Bar));
        assert_eq!(lookup_accent("vec"), Some(AccentKind::Vec));
        assert_eq!(lookup_accent("dot"), Some(AccentKind::Dot));
        assert_eq!(lookup_accent("tilde"), Some(AccentKind::Tilde));
        assert_eq!(lookup_accent("unknown"), None);
    }

    #[test]
    fn test_combining_chars() {
        assert_eq!(AccentKind::Hat.combining_char(), '\u{0302}');
        assert_eq!(AccentKind::Bar.combining_char(), '\u{0304}');
        assert_eq!(AccentKind::Vec.combining_char(), '\u{20D7}');
        assert_eq!(AccentKind::Dot.combining_char(), '\u{0307}');
        assert_eq!(AccentKind::Tilde.combining_char(), '\u{0303}');
    }
}
