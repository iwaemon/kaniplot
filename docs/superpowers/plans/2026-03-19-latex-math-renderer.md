# LaTeX Math Renderer Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable `$...$` LaTeX math in titles, axis labels, and legend labels, rendered via SVG `<text>`/`<tspan>` with embedded Latin Modern Math font.

**Architecture:** Three-layer pipeline: LaTeX parser (string → MathNode AST), layout engine (AST → positioned glyphs), SVG integration (glyphs → `<tspan>` elements). Font is Base64-embedded in SVG via `@font-face`. The `base64` crate is added as a dependency.

**Tech Stack:** Rust, `base64` crate, Latin Modern Math WOFF2 font (SIL OFL)

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/math/mod.rs` | Module root: re-exports parser, layout, symbols | **Create** |
| `src/math/symbols.rs` | `\alpha` → `'α'` command-to-Unicode mapping table | **Create** |
| `src/math/parser.rs` | LaTeX subset parser: string → `Vec<MathNode>` | **Create** |
| `src/math/layout.rs` | Layout engine: `Vec<MathNode>` → `LayoutResult` | **Create** |
| `src/fonts/mod.rs` | `include_bytes!` for font + Base64 helper | **Create** |
| `fonts/latinmodern-math.woff2` | Latin Modern Math font file | **Create** (download) |
| `fonts/OFL.txt` | SIL Open Font License text | **Create** |
| `src/lib.rs` | Add `pub mod math; pub mod fonts;` | **Modify** |
| `src/renderer/svg.rs` | `$...$` detection + `<tspan>` math rendering + `@font-face` | **Modify** |
| `Cargo.toml` | Add `base64` dependency | **Modify** |
| `tests/integration.rs` | End-to-end math rendering tests | **Modify** |

---

### Task 1: Symbol mapping table

**Files:**
- Create: `src/math/symbols.rs`
- Create: `src/math/mod.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Write failing tests**

Create `src/math/symbols.rs`:

```rust
/// Look up a LaTeX command name (without backslash) and return its Unicode character.
pub fn lookup_symbol(name: &str) -> Option<char> {
    todo!()
}

/// Look up an accent command name and return the AccentKind.
pub fn lookup_accent(name: &str) -> Option<AccentKind> {
    todo!()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AccentKind {
    Hat,    // \hat  → U+0302
    Bar,    // \bar  → U+0304
    Vec,    // \vec  → U+20D7
    Dot,    // \dot  → U+0307
    Tilde,  // \tilde → U+0303
}

impl AccentKind {
    pub fn combining_char(self) -> char {
        match self {
            AccentKind::Hat   => '\u{0302}',
            AccentKind::Bar   => '\u{0304}',
            AccentKind::Vec   => '\u{20D7}',
            AccentKind::Dot   => '\u{0307}',
            AccentKind::Tilde => '\u{0303}',
        }
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
        assert_eq!(lookup_symbol("omega"), Some('ω'));
    }

    #[test]
    fn test_greek_uppercase() {
        assert_eq!(lookup_symbol("Gamma"), Some('Γ'));
        assert_eq!(lookup_symbol("Delta"), Some('Δ'));
        assert_eq!(lookup_symbol("Sigma"), Some('Σ'));
        assert_eq!(lookup_symbol("Omega"), Some('Ω'));
    }

    #[test]
    fn test_operators() {
        assert_eq!(lookup_symbol("sum"), Some('Σ'));
        assert_eq!(lookup_symbol("int"), Some('∫'));
        assert_eq!(lookup_symbol("prod"), Some('Π'));
        assert_eq!(lookup_symbol("infty"), Some('∞'));
    }

    #[test]
    fn test_relations() {
        assert_eq!(lookup_symbol("leq"), Some('≤'));
        assert_eq!(lookup_symbol("geq"), Some('≥'));
        assert_eq!(lookup_symbol("neq"), Some('≠'));
        assert_eq!(lookup_symbol("approx"), Some('≈'));
    }

    #[test]
    fn test_misc() {
        assert_eq!(lookup_symbol("partial"), Some('∂'));
        assert_eq!(lookup_symbol("nabla"), Some('∇'));
        assert_eq!(lookup_symbol("pm"), Some('±'));
        assert_eq!(lookup_symbol("times"), Some('×'));
        assert_eq!(lookup_symbol("cdot"), Some('·'));
        assert_eq!(lookup_symbol("ldots"), Some('…'));
    }

    #[test]
    fn test_unknown() {
        assert_eq!(lookup_symbol("notacommand"), None);
    }

    #[test]
    fn test_accents() {
        assert_eq!(lookup_accent("hat"), Some(AccentKind::Hat));
        assert_eq!(lookup_accent("bar"), Some(AccentKind::Bar));
        assert_eq!(lookup_accent("vec"), Some(AccentKind::Vec));
        assert_eq!(lookup_accent("dot"), Some(AccentKind::Dot));
        assert_eq!(lookup_accent("tilde"), Some(AccentKind::Tilde));
        assert_eq!(lookup_accent("notaccent"), None);
    }
}
```

- [ ] **Step 2: Create module files**

Create `src/math/mod.rs`:

```rust
pub mod symbols;
pub mod parser;
pub mod layout;
```

Add to `src/lib.rs` after `pub mod renderer;`:

```rust
pub mod math;
pub mod fonts;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test math::symbols --lib`
Expected: FAIL (todo!())

- [ ] **Step 4: Implement symbol lookup**

Replace the `todo!()` in `lookup_symbol`:

```rust
pub fn lookup_symbol(name: &str) -> Option<char> {
    match name {
        // Greek lowercase
        "alpha"   => Some('α'), "beta"    => Some('β'), "gamma"   => Some('γ'),
        "delta"   => Some('δ'), "epsilon" => Some('ε'), "zeta"    => Some('ζ'),
        "eta"     => Some('η'), "theta"   => Some('θ'), "iota"    => Some('ι'),
        "kappa"   => Some('κ'), "lambda"  => Some('λ'), "mu"      => Some('μ'),
        "nu"      => Some('ν'), "xi"      => Some('ξ'), "pi"      => Some('π'),
        "rho"     => Some('ρ'), "sigma"   => Some('σ'), "tau"     => Some('τ'),
        "upsilon" => Some('υ'), "phi"     => Some('φ'), "chi"     => Some('χ'),
        "psi"     => Some('ψ'), "omega"   => Some('ω'),
        // Greek uppercase
        "Gamma" => Some('Γ'), "Delta"   => Some('Δ'), "Theta"   => Some('Θ'),
        "Lambda" => Some('Λ'), "Xi"     => Some('Ξ'), "Pi"      => Some('Π'),
        "Sigma" => Some('Σ'), "Upsilon" => Some('Υ'), "Phi"     => Some('Φ'),
        "Psi"   => Some('Ψ'), "Omega"   => Some('Ω'),
        // Large operators
        "sum"  => Some('Σ'), "prod" => Some('Π'), "int" => Some('∫'),
        // Relations
        "leq"    => Some('≤'), "geq"    => Some('≥'), "neq"     => Some('≠'),
        "approx" => Some('≈'), "equiv"  => Some('≡'), "sim"     => Some('∼'),
        // Misc
        "infty"   => Some('∞'), "partial" => Some('∂'), "nabla" => Some('∇'),
        "pm"      => Some('±'), "mp"      => Some('∓'), "times" => Some('×'),
        "cdot"    => Some('·'), "ldots"   => Some('…'),
        _ => None,
    }
}

pub fn lookup_accent(name: &str) -> Option<AccentKind> {
    match name {
        "hat"   => Some(AccentKind::Hat),
        "bar"   => Some(AccentKind::Bar),
        "vec"   => Some(AccentKind::Vec),
        "dot"   => Some(AccentKind::Dot),
        "tilde" => Some(AccentKind::Tilde),
        _ => None,
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test math::symbols --lib`
Expected: PASS (8 tests)

- [ ] **Step 6: Commit**

```bash
git add src/math/ src/lib.rs
git commit -m "feat: add LaTeX symbol-to-Unicode mapping table"
```

---

### Task 2: LaTeX parser

**Files:**
- Create: `src/math/parser.rs`

- [ ] **Step 1: Write AST types and failing tests**

Create `src/math/parser.rs`:

```rust
use crate::math::symbols::AccentKind;

#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    Char(char),
    Symbol(char),
    Number(String),
    Operator(char),
    Group(Vec<MathNode>),
    Superscript(Box<MathNode>, Box<MathNode>),
    Subscript(Box<MathNode>, Box<MathNode>),
    SubSuperscript(Box<MathNode>, Box<MathNode>, Box<MathNode>),
    Frac(Vec<MathNode>, Vec<MathNode>),
    Accent(AccentKind, Box<MathNode>),
    TextRoman(String),
}

/// Parse a LaTeX math string (contents between $...$) into a list of MathNodes.
pub fn parse_math(input: &str) -> Result<Vec<MathNode>, String> {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_chars() {
        let nodes = parse_math("abc").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Char('a'),
            MathNode::Char('b'),
            MathNode::Char('c'),
        ]);
    }

    #[test]
    fn test_number() {
        let nodes = parse_math("42").unwrap();
        assert_eq!(nodes, vec![MathNode::Number("42".into())]);
    }

    #[test]
    fn test_operators() {
        let nodes = parse_math("a + b").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Char('a'),
            MathNode::Operator('+'),
            MathNode::Char('b'),
        ]);
    }

    #[test]
    fn test_superscript_single() {
        let nodes = parse_math("x^2").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Superscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Number("2".into())),
            ),
        ]);
    }

    #[test]
    fn test_superscript_group() {
        let nodes = parse_math("x^{10}").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Superscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Group(vec![
                    MathNode::Number("10".into()),
                ])),
            ),
        ]);
    }

    #[test]
    fn test_subscript() {
        let nodes = parse_math("x_i").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Subscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Char('i')),
            ),
        ]);
    }

    #[test]
    fn test_sub_superscript() {
        let nodes = parse_math("x_i^2").unwrap();
        assert_eq!(nodes, vec![
            MathNode::SubSuperscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Char('i')),
                Box::new(MathNode::Number("2".into())),
            ),
        ]);
    }

    #[test]
    fn test_greek_symbol() {
        let nodes = parse_math("\\alpha").unwrap();
        assert_eq!(nodes, vec![MathNode::Symbol('α')]);
    }

    #[test]
    fn test_frac() {
        let nodes = parse_math("\\frac{a}{b}").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Frac(
                vec![MathNode::Char('a')],
                vec![MathNode::Char('b')],
            ),
        ]);
    }

    #[test]
    fn test_accent_hat() {
        let nodes = parse_math("\\hat{x}").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Accent(AccentKind::Hat, Box::new(MathNode::Char('x'))),
        ]);
    }

    #[test]
    fn test_mathrm() {
        let nodes = parse_math("\\mathrm{Re}").unwrap();
        assert_eq!(nodes, vec![MathNode::TextRoman("Re".into())]);
    }

    #[test]
    fn test_complex_expression() {
        // E = mc^2
        let nodes = parse_math("E = mc^2").unwrap();
        assert_eq!(nodes.len(), 4); // E, =, m, c^2
        assert_eq!(nodes[0], MathNode::Char('E'));
        assert_eq!(nodes[1], MathNode::Operator('='));
        assert_eq!(nodes[2], MathNode::Char('m'));
        match &nodes[3] {
            MathNode::Superscript(base, exp) => {
                assert_eq!(**base, MathNode::Char('c'));
                assert_eq!(**exp, MathNode::Number("2".into()));
            }
            other => panic!("Expected Superscript, got {:?}", other),
        }
    }

    #[test]
    fn test_spaces_ignored() {
        let nodes = parse_math("a  b").unwrap();
        assert_eq!(nodes, vec![MathNode::Char('a'), MathNode::Char('b')]);
    }

    #[test]
    fn test_mixed_greek_and_text() {
        let nodes = parse_math("\\omega_0").unwrap();
        assert_eq!(nodes, vec![
            MathNode::Subscript(
                Box::new(MathNode::Symbol('ω')),
                Box::new(MathNode::Number("0".into())),
            ),
        ]);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test math::parser --lib`
Expected: FAIL (todo!())

- [ ] **Step 3: Implement the parser**

Replace `todo!()` in `parse_math`:

```rust
pub fn parse_math(input: &str) -> Result<Vec<MathNode>, String> {
    let chars: Vec<char> = input.chars().collect();
    let mut pos = 0;
    let mut nodes = Vec::new();

    while pos < chars.len() {
        let node = parse_atom(&chars, &mut pos)?;
        // Check for sub/superscript
        let node = parse_scripts(&chars, &mut pos, node)?;
        nodes.push(node);
    }

    Ok(nodes)
}

fn parse_atom(chars: &[char], pos: &mut usize) -> Result<MathNode, String> {
    skip_spaces(chars, pos);
    if *pos >= chars.len() {
        return Err("Unexpected end of math input".into());
    }

    let ch = chars[*pos];
    match ch {
        '\\' => parse_command(chars, pos),
        '{' => {
            *pos += 1;
            let inner = parse_until_brace(chars, pos)?;
            Ok(MathNode::Group(inner))
        }
        '0'..='9' | '.' => parse_number(chars, pos),
        '+' | '-' | '=' | '<' | '>' | '(' | ')' | '[' | ']' | ',' | '/' | '!' => {
            *pos += 1;
            Ok(MathNode::Operator(ch))
        }
        'a'..='z' | 'A'..='Z' => {
            *pos += 1;
            Ok(MathNode::Char(ch))
        }
        _ => {
            *pos += 1;
            Ok(MathNode::Char(ch))
        }
    }
}

fn parse_command(chars: &[char], pos: &mut usize) -> Result<MathNode, String> {
    *pos += 1; // skip backslash
    let start = *pos;
    while *pos < chars.len() && chars[*pos].is_ascii_alphabetic() {
        *pos += 1;
    }
    let name: String = chars[start..*pos].iter().collect();

    if name == "frac" {
        skip_spaces(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '{' {
            return Err("\\frac expects {numerator}{denominator}".into());
        }
        *pos += 1;
        let num = parse_until_brace(chars, pos)?;
        skip_spaces(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '{' {
            return Err("\\frac expects {numerator}{denominator}".into());
        }
        *pos += 1;
        let den = parse_until_brace(chars, pos)?;
        return Ok(MathNode::Frac(num, den));
    }

    if name == "mathrm" {
        skip_spaces(chars, pos);
        if *pos >= chars.len() || chars[*pos] != '{' {
            return Err("\\mathrm expects {text}".into());
        }
        *pos += 1;
        let start = *pos;
        while *pos < chars.len() && chars[*pos] != '}' {
            *pos += 1;
        }
        let text: String = chars[start..*pos].iter().collect();
        if *pos < chars.len() { *pos += 1; } // skip }
        return Ok(MathNode::TextRoman(text));
    }

    if let Some(accent_kind) = crate::math::symbols::lookup_accent(&name) {
        skip_spaces(chars, pos);
        if *pos < chars.len() && chars[*pos] == '{' {
            *pos += 1;
            let inner = parse_until_brace(chars, pos)?;
            let body = if inner.len() == 1 {
                inner.into_iter().next().unwrap()
            } else {
                MathNode::Group(inner)
            };
            return Ok(MathNode::Accent(accent_kind, Box::new(body)));
        } else if *pos < chars.len() {
            let atom = parse_atom(chars, pos)?;
            return Ok(MathNode::Accent(accent_kind, Box::new(atom)));
        }
        return Err(format!("\\{name} expects an argument"));
    }

    if let Some(sym) = crate::math::symbols::lookup_symbol(&name) {
        return Ok(MathNode::Symbol(sym));
    }

    Err(format!("Unknown LaTeX command: \\{name}"))
}

fn parse_number(chars: &[char], pos: &mut usize) -> Result<MathNode, String> {
    let start = *pos;
    while *pos < chars.len() && (chars[*pos].is_ascii_digit() || chars[*pos] == '.') {
        *pos += 1;
    }
    let s: String = chars[start..*pos].iter().collect();
    Ok(MathNode::Number(s))
}

fn parse_until_brace(chars: &[char], pos: &mut usize) -> Result<Vec<MathNode>, String> {
    let mut nodes = Vec::new();
    while *pos < chars.len() && chars[*pos] != '}' {
        let node = parse_atom(chars, pos)?;
        let node = parse_scripts(chars, pos, node)?;
        nodes.push(node);
    }
    if *pos < chars.len() { *pos += 1; } // skip }
    Ok(nodes)
}

fn parse_scripts(chars: &[char], pos: &mut usize, base: MathNode) -> Result<MathNode, String> {
    skip_spaces(chars, pos);

    let mut sub = None;
    let mut sup = None;

    // Check for _ or ^
    while *pos < chars.len() && (chars[*pos] == '_' || chars[*pos] == '^') {
        let is_super = chars[*pos] == '^';
        *pos += 1;
        skip_spaces(chars, pos);
        let script = if *pos < chars.len() && chars[*pos] == '{' {
            *pos += 1;
            let inner = parse_until_brace(chars, pos)?;
            if inner.len() == 1 {
                inner.into_iter().next().unwrap()
            } else {
                MathNode::Group(inner)
            }
        } else if *pos < chars.len() {
            parse_atom(chars, pos)?
        } else {
            return Err("Expected script after ^ or _".into());
        };

        if is_super {
            sup = Some(script);
        } else {
            sub = Some(script);
        }

        skip_spaces(chars, pos);
    }

    match (sub, sup) {
        (None, None) => Ok(base),
        (None, Some(s)) => Ok(MathNode::Superscript(Box::new(base), Box::new(s))),
        (Some(s), None) => Ok(MathNode::Subscript(Box::new(base), Box::new(s))),
        (Some(sb), Some(sp)) => Ok(MathNode::SubSuperscript(Box::new(base), Box::new(sb), Box::new(sp))),
    }
}

fn skip_spaces(chars: &[char], pos: &mut usize) {
    while *pos < chars.len() && chars[*pos] == ' ' {
        *pos += 1;
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test math::parser --lib`
Expected: PASS (14 tests)

- [ ] **Step 5: Commit**

```bash
git add src/math/parser.rs
git commit -m "feat: implement LaTeX subset parser with sub/superscript, frac, accents"
```

---

### Task 3: Layout engine

**Files:**
- Create: `src/math/layout.rs`

- [ ] **Step 1: Write types and failing tests**

Create `src/math/layout.rs`:

```rust
use crate::math::parser::MathNode;
use crate::math::symbols::AccentKind;

/// A positioned glyph in the layout output.
#[derive(Debug, Clone)]
pub struct LayoutGlyph {
    pub text: String,
    pub x: f64,             // x offset from start (em units)
    pub y: f64,             // y offset from baseline (em units, negative = up)
    pub font_size_ratio: f64,
    pub italic: bool,
    pub is_math_font: bool,
}

/// Result of laying out a math expression.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub glyphs: Vec<LayoutGlyph>,
    pub width: f64,
    pub height: f64,
    pub baseline: f64,
}

/// Lay out a list of MathNodes into positioned glyphs.
pub fn layout_math(nodes: &[MathNode]) -> LayoutResult {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::parser::parse_math;

    #[test]
    fn test_simple_char() {
        let nodes = parse_math("x").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 1);
        assert_eq!(result.glyphs[0].text, "x");
        assert!(result.glyphs[0].italic);
        assert!(result.width > 0.0);
    }

    #[test]
    fn test_number_not_italic() {
        let nodes = parse_math("42").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 1);
        assert_eq!(result.glyphs[0].text, "42");
        assert!(!result.glyphs[0].italic);
    }

    #[test]
    fn test_operator_spacing() {
        let nodes = parse_math("a + b").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 3);
        // Operator '+' should have spacing around it
        let plus_x = result.glyphs[1].x;
        let a_end = result.glyphs[0].x + 0.55; // char width
        assert!(plus_x > a_end, "Operator should have space before it");
    }

    #[test]
    fn test_superscript_position() {
        let nodes = parse_math("x^2").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 2); // x and 2
        let base = &result.glyphs[0];
        let sup = &result.glyphs[1];
        assert!(sup.y < base.y, "Superscript should be above baseline");
        assert!(sup.font_size_ratio < 1.0, "Superscript should be smaller");
    }

    #[test]
    fn test_subscript_position() {
        let nodes = parse_math("x_i").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 2);
        let sub = &result.glyphs[1];
        assert!(sub.y > 0.0, "Subscript should be below baseline");
        assert!(sub.font_size_ratio < 1.0, "Subscript should be smaller");
    }

    #[test]
    fn test_frac_layout() {
        let nodes = parse_math("\\frac{a}{b}").unwrap();
        let result = layout_math(&nodes);
        // Should have: numerator 'a', fraction line '—', denominator 'b'
        assert!(result.glyphs.len() >= 3);
        let num = result.glyphs.iter().find(|g| g.text == "a").unwrap();
        let den = result.glyphs.iter().find(|g| g.text == "b").unwrap();
        assert!(num.y < den.y, "Numerator should be above denominator");
    }

    #[test]
    fn test_symbol_uses_math_font() {
        let nodes = parse_math("\\alpha").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 1);
        assert_eq!(result.glyphs[0].text, "α");
        assert!(result.glyphs[0].is_math_font);
    }

    #[test]
    fn test_mathrm_not_italic() {
        let nodes = parse_math("\\mathrm{Re}").unwrap();
        let result = layout_math(&nodes);
        assert_eq!(result.glyphs.len(), 1);
        assert_eq!(result.glyphs[0].text, "Re");
        assert!(!result.glyphs[0].italic);
    }

    #[test]
    fn test_accent() {
        let nodes = parse_math("\\hat{x}").unwrap();
        let result = layout_math(&nodes);
        // Should produce a single glyph "x̂" (x + combining hat)
        assert_eq!(result.glyphs.len(), 1);
        assert!(result.glyphs[0].text.contains('x'));
    }

    #[test]
    fn test_emc2() {
        let nodes = parse_math("E = mc^2").unwrap();
        let result = layout_math(&nodes);
        // E, =, m, c, 2
        assert!(result.glyphs.len() >= 4);
        assert!(result.width > 0.0);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test math::layout --lib`
Expected: FAIL (todo!())

- [ ] **Step 3: Implement the layout engine**

Replace `todo!()` in `layout_math`:

```rust
// Character width constants (em units)
const CHAR_WIDTH: f64 = 0.55;
const DIGIT_WIDTH: f64 = 0.5;
const OPERATOR_WIDTH: f64 = 0.6;
const OPERATOR_SPACE: f64 = 0.15;
const SYMBOL_WIDTH: f64 = 0.6;
const LARGE_OP_WIDTH: f64 = 0.8;

const SUPERSCRIPT_RATIO: f64 = 0.7;
const SUPERSCRIPT_SHIFT: f64 = -0.4;
const SUBSCRIPT_RATIO: f64 = 0.7;
const SUBSCRIPT_SHIFT: f64 = 0.2;
const FRAC_RATIO: f64 = 0.8;
const FRAC_NUM_SHIFT: f64 = -0.7;
const FRAC_DEN_SHIFT: f64 = 0.3;

pub fn layout_math(nodes: &[MathNode]) -> LayoutResult {
    let mut glyphs = Vec::new();
    let mut x = 0.0;
    layout_nodes(nodes, &mut glyphs, &mut x, 0.0, 1.0);
    let min_y = glyphs.iter().map(|g| g.y).fold(0.0_f64, f64::min);
    let max_y = glyphs.iter().map(|g| g.y + g.font_size_ratio).fold(1.0_f64, f64::max);
    LayoutResult { glyphs, width: x, height: max_y - min_y, baseline: -min_y }
}

fn layout_nodes(
    nodes: &[MathNode],
    glyphs: &mut Vec<LayoutGlyph>,
    x: &mut f64,
    y: f64,
    size_ratio: f64,
) {
    for node in nodes {
        layout_node(node, glyphs, x, y, size_ratio);
    }
}

fn layout_node(
    node: &MathNode,
    glyphs: &mut Vec<LayoutGlyph>,
    x: &mut f64,
    y: f64,
    size_ratio: f64,
) {
    match node {
        MathNode::Char(c) => {
            glyphs.push(LayoutGlyph {
                text: c.to_string(),
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: true,
                is_math_font: true,
            });
            *x += CHAR_WIDTH * size_ratio;
        }
        MathNode::Number(s) => {
            glyphs.push(LayoutGlyph {
                text: s.clone(),
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: false,
                is_math_font: true,
            });
            *x += DIGIT_WIDTH * s.len() as f64 * size_ratio;
        }
        MathNode::Operator(c) => {
            *x += OPERATOR_SPACE * size_ratio;
            glyphs.push(LayoutGlyph {
                text: c.to_string(),
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: false,
                is_math_font: true,
            });
            *x += OPERATOR_WIDTH * size_ratio;
            *x += OPERATOR_SPACE * size_ratio;
        }
        MathNode::Symbol(c) => {
            let w = if matches!(c, 'Σ' | 'Π' | '∫') { LARGE_OP_WIDTH } else { SYMBOL_WIDTH };
            glyphs.push(LayoutGlyph {
                text: c.to_string(),
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: false,
                is_math_font: true,
            });
            *x += w * size_ratio;
        }
        MathNode::Group(children) => {
            layout_nodes(children, glyphs, x, y, size_ratio);
        }
        MathNode::Superscript(base, sup) => {
            layout_node(base, glyphs, x, y, size_ratio);
            let sup_y = y + SUPERSCRIPT_SHIFT * size_ratio;
            let sup_size = size_ratio * SUPERSCRIPT_RATIO;
            layout_node(sup, glyphs, x, sup_y, sup_size);
        }
        MathNode::Subscript(base, sub) => {
            layout_node(base, glyphs, x, y, size_ratio);
            let sub_y = y + SUBSCRIPT_SHIFT * size_ratio;
            let sub_size = size_ratio * SUBSCRIPT_RATIO;
            layout_node(sub, glyphs, x, sub_y, sub_size);
        }
        MathNode::SubSuperscript(base, sub, sup) => {
            layout_node(base, glyphs, x, y, size_ratio);
            let save_x = *x;
            let sup_y = y + SUPERSCRIPT_SHIFT * size_ratio;
            let sup_size = size_ratio * SUPERSCRIPT_RATIO;
            layout_node(sup, glyphs, x, sup_y, sup_size);
            let after_sup = *x;
            *x = save_x;
            let sub_y = y + SUBSCRIPT_SHIFT * size_ratio;
            let sub_size = size_ratio * SUBSCRIPT_RATIO;
            layout_node(sub, glyphs, x, sub_y, sub_size);
            let after_sub = *x;
            *x = after_sup.max(after_sub);
        }
        MathNode::Frac(num, den) => {
            // Layout numerator and denominator separately to find widths
            let mut num_glyphs = Vec::new();
            let mut num_w = 0.0;
            let frac_size = size_ratio * FRAC_RATIO;
            layout_nodes(num, &mut num_glyphs, &mut num_w, 0.0, frac_size);
            let mut den_glyphs = Vec::new();
            let mut den_w = 0.0;
            layout_nodes(den, &mut den_glyphs, &mut den_w, 0.0, frac_size);

            let total_w = num_w.max(den_w);
            let num_offset = (total_w - num_w) / 2.0;
            let den_offset = (total_w - den_w) / 2.0;

            // Place numerator
            for mut g in num_glyphs {
                g.x += *x + num_offset;
                g.y += y + FRAC_NUM_SHIFT * size_ratio;
                glyphs.push(g);
            }
            // Fraction line
            glyphs.push(LayoutGlyph {
                text: "—".into(),
                x: *x, y: y - 0.05 * size_ratio,
                font_size_ratio: size_ratio,
                italic: false,
                is_math_font: true,
            });
            // Place denominator
            for mut g in den_glyphs {
                g.x += *x + den_offset;
                g.y += y + FRAC_DEN_SHIFT * size_ratio;
                glyphs.push(g);
            }

            *x += total_w;
        }
        MathNode::Accent(kind, body) => {
            // Render body with combining accent character
            let body_text = match body.as_ref() {
                MathNode::Char(c) => c.to_string(),
                MathNode::Symbol(c) => c.to_string(),
                _ => {
                    // For complex bodies, layout body and append accent to last glyph
                    layout_node(body, glyphs, x, y, size_ratio);
                    if let Some(last) = glyphs.last_mut() {
                        last.text.push(kind.combining_char());
                    }
                    return;
                }
            };
            let mut text = body_text;
            text.push(kind.combining_char());
            glyphs.push(LayoutGlyph {
                text,
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: true,
                is_math_font: true,
            });
            *x += CHAR_WIDTH * size_ratio;
        }
        MathNode::TextRoman(s) => {
            glyphs.push(LayoutGlyph {
                text: s.clone(),
                x: *x, y,
                font_size_ratio: size_ratio,
                italic: false,
                is_math_font: false,
            });
            *x += CHAR_WIDTH * s.len() as f64 * size_ratio;
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test math::layout --lib`
Expected: PASS (10 tests)

- [ ] **Step 5: Commit**

```bash
git add src/math/layout.rs
git commit -m "feat: implement math layout engine with positioning and sizing"
```

---

### Task 4: Font embedding

**Files:**
- Create: `fonts/latinmodern-math.woff2` (download)
- Create: `fonts/OFL.txt`
- Create: `src/fonts/mod.rs`
- Modify: `Cargo.toml`

- [ ] **Step 1: Download Latin Modern Math WOFF2**

Download Latin Modern Math font. Try these sources in order:

```bash
mkdir -p fonts

# Option 1: GUST official (OTF — convert to WOFF2 or use OTF directly)
curl -L -o /tmp/lm-math.zip "https://www.gust.org.pl/projects/e-foundry/lm-math/download/latinmodern-math-1959.zip"
unzip -j /tmp/lm-math.zip "*/latinmodern-math.otf" -d fonts/ 2>/dev/null || \
unzip -j /tmp/lm-math.zip "*/LMMath-Regular.otf" -d fonts/ 2>/dev/null

# Option 2: If WOFF2 needed, use google-webfonts-helper or similar
# The OTF format also works with @font-face (change format to "opentype")
```

**If only OTF is available:** Change `src/fonts/mod.rs` to use `include_bytes!("../../fonts/latinmodern-math.otf")` and the `@font-face` format to `format("opentype")` instead of `format("woff2")`. Both work in SVG; OTF files are larger (~500KB vs ~300KB) but functionally equivalent.

**Fallback:** If the font cannot be downloaded, create a minimal placeholder and note it as a TODO. The math rendering pipeline works regardless — the font just affects visual appearance.

- [ ] **Step 2: Create OFL license file**

Create `fonts/OFL.txt` with the SIL Open Font License 1.1 text. The key notice:

```
Copyright 2012-2014 GUST e-Foundry.
This Font Software is licensed under the SIL Open Font License, Version 1.1.
```

- [ ] **Step 3: Add base64 dependency**

Add to `Cargo.toml`:

```toml
[dependencies]
pest = "2.8"
pest_derive = "2.8"
base64 = "0.22"
```

- [ ] **Step 4: Write failing tests and implement font module**

Create `src/fonts/mod.rs`:

```rust
use base64::Engine;

const FONT_DATA: &[u8] = include_bytes!("../../fonts/latinmodern-math.woff2");

/// Return the font data as a Base64-encoded string.
pub fn font_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(FONT_DATA)
}

/// Return a CSS @font-face rule with the embedded font.
pub fn svg_font_face_style() -> String {
    format!(
        r#"@font-face {{ font-family: "Latin Modern Math"; src: url(data:font/woff2;base64,{}) format("woff2"); }}"#,
        font_base64()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_data_not_empty() {
        assert!(!FONT_DATA.is_empty());
    }

    #[test]
    fn test_base64_encoding() {
        let b64 = font_base64();
        assert!(!b64.is_empty());
        // Verify it's valid base64 by decoding
        let decoded = base64::engine::general_purpose::STANDARD.decode(&b64).unwrap();
        assert_eq!(decoded.len(), FONT_DATA.len());
    }

    #[test]
    fn test_font_face_style() {
        let style = svg_font_face_style();
        assert!(style.contains("@font-face"));
        assert!(style.contains("Latin Modern Math"));
        assert!(style.contains("data:font/woff2;base64,"));
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test fonts --lib`
Expected: PASS (3 tests)

- [ ] **Step 6: Commit**

```bash
git add fonts/ src/fonts/ Cargo.toml
git commit -m "feat: embed Latin Modern Math font with Base64 encoding"
```

---

### Task 5: SVG integration — `$...$` detection and math rendering

**Files:**
- Modify: `src/renderer/svg.rs`

- [ ] **Step 1: Write failing tests**

Add to `src/renderer/svg.rs` tests module:

```rust
#[test]
fn test_svg_math_title() {
    let mut model = make_simple_model();
    model.title = Some("$E = mc^2$".into());
    let svg = render_svg(&model);
    assert!(svg.contains("Latin Modern Math"), "Should use math font");
    assert!(svg.contains("@font-face"), "Should embed font");
}

#[test]
fn test_svg_mixed_title() {
    let mut model = make_simple_model();
    model.title = Some("Energy: $E = mc^2$".into());
    let svg = render_svg(&model);
    assert!(svg.contains("Energy:"), "Should contain plain text");
    assert!(svg.contains("Latin Modern Math"), "Should contain math font");
}

#[test]
fn test_svg_no_font_when_no_math() {
    let model = make_simple_model();
    let svg = render_svg(&model);
    assert!(!svg.contains("@font-face"), "Should NOT embed font when no math");
}

#[test]
fn test_svg_math_in_xlabel() {
    let mut model = make_simple_model();
    model.x_axis.label = Some("$\\omega$ (rad/s)".into());
    let svg = render_svg(&model);
    assert!(svg.contains("ω"), "Should contain omega symbol");
    assert!(svg.contains("@font-face"), "Should embed font");
}

#[test]
fn test_svg_math_in_legend() {
    let mut model = make_simple_model();
    model.series[0].label = Some("$\\alpha$ curve".into());
    let svg = render_svg(&model);
    assert!(svg.contains("α"), "Should contain alpha symbol");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test renderer::svg --lib`
Expected: FAIL (new tests fail because math rendering not yet integrated)

- [ ] **Step 3: Implement `$...$` detection and SVG math rendering**

Add these helper functions to `src/renderer/svg.rs`:

```rust
use crate::math;

/// Check if any text in the model contains math ($...$).
fn model_has_math(model: &PlotModel) -> bool {
    let texts = [
        model.title.as_deref(),
        model.x_axis.label.as_deref(),
        model.y_axis.label.as_deref(),
    ];
    for t in texts.into_iter().flatten() {
        if t.contains('$') { return true; }
    }
    for s in &model.series {
        if let Some(label) = &s.label {
            if label.contains('$') { return true; }
        }
    }
    false
}

/// Render a text string that may contain $...$ math segments into SVG tspan elements.
fn render_math_text(text: &str, font_size: f64) -> String {
    if !text.contains('$') {
        return escape_xml(text);
    }

    let parts: Vec<&str> = text.split('$').collect();
    let mut out = String::new();

    for (i, part) in parts.iter().enumerate() {
        if part.is_empty() { continue; }
        if i % 2 == 0 {
            // Plain text
            out.push_str(&escape_xml(part));
        } else {
            // Math text
            match math::parser::parse_math(part) {
                Ok(nodes) => {
                    let result = math::layout::layout_math(&nodes);
                    let mut prev_dy: f64 = 0.0;
                    for glyph in &result.glyphs {
                        let glyph_size = font_size * glyph.font_size_ratio;
                        let dy_px = glyph.y * font_size - prev_dy;
                        let font_family = if glyph.is_math_font {
                            r#" font-family="Latin Modern Math""#
                        } else {
                            ""
                        };
                        let font_style = if glyph.italic {
                            r#" font-style="italic""#
                        } else {
                            ""
                        };
                        use std::fmt::Write;
                        write!(out,
                            r#"<tspan{font_family}{font_style} font-size="{glyph_size:.1}"{}>{}</tspan>"#,
                            if dy_px.abs() > 0.01 { format!(r#" dy="{dy_px:.1}""#) } else { String::new() },
                            escape_xml(&glyph.text),
                        ).unwrap();
                        prev_dy = glyph.y * font_size;
                    }
                    // Reset dy
                    if prev_dy.abs() > 0.01 {
                        use std::fmt::Write;
                        write!(out, r#"<tspan dy="{:.1}"> </tspan>"#, -prev_dy).unwrap();
                    }
                }
                Err(_) => {
                    // Fallback: render as plain text
                    out.push_str(&escape_xml(part));
                }
            }
        }
    }

    out
}
```

Then modify `render_svg` in these places:

**a) Replace the `<defs>` line (line 38) to conditionally include font:**

Replace:
```rust
writeln!(svg, r#"<defs><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#).unwrap();
```

With:
```rust
if model_has_math(model) {
    writeln!(svg, r#"<defs><style>{}</style><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#,
        crate::fonts::svg_font_face_style()
    ).unwrap();
} else {
    writeln!(svg, r#"<defs><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#).unwrap();
}
```

**b) Replace title rendering (lines 44-47):**

Replace:
```rust
        writeln!(svg,
            r#"<text x="{tx}" y="{ty}" text-anchor="middle" font-size="{TITLE_FONT_SIZE}" font-weight="bold">{}</text>"#,
            escape_xml(title)
        ).unwrap();
```

With:
```rust
        writeln!(svg,
            r#"<text x="{tx}" y="{ty}" text-anchor="middle" font-size="{TITLE_FONT_SIZE}" font-weight="bold">{}</text>"#,
            render_math_text(title, TITLE_FONT_SIZE)
        ).unwrap();
```

**c) Replace xlabel rendering (lines 114-117):**

Replace `escape_xml(label)` with `render_math_text(label, FONT_SIZE)`.

**d) Replace ylabel rendering (lines 124-127):**

Replace `escape_xml(label)` with `render_math_text(label, FONT_SIZE)`.

**e) Replace legend text rendering (lines 259-262):**

Replace `escape_xml(series.label.as_deref().unwrap_or(""))` with `render_math_text(series.label.as_deref().unwrap_or(""), LEGEND_FONT_SIZE)`.

- [ ] **Step 4: Run all tests**

Run: `cargo test --lib`
Expected: ALL PASS

- [ ] **Step 5: Commit**

```bash
git add src/renderer/svg.rs
git commit -m "feat: integrate math rendering into SVG output with font embedding"
```

---

### Task 6: Integration tests

**Files:**
- Modify: `tests/integration.rs`

- [ ] **Step 1: Write integration tests**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_math_in_title() {
    let script = "set title \"$E = mc^2$\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(stdout.contains("Latin Modern Math"), "Should use math font");
    assert!(stdout.contains("@font-face"), "Should embed font");
}

#[test]
fn test_math_in_xlabel() {
    let script = "set xlabel \"$\\\\omega$ (rad/s)\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(stdout.contains("ω"), "Should render omega");
}

#[test]
fn test_no_math_no_font_embedding() {
    let script = "set title \"Plain Title\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(!stdout.contains("@font-face"), "Should not embed font");
}
```

- [ ] **Step 2: Run integration tests**

Run: `cargo test --test integration`
Expected: ALL PASS

- [ ] **Step 3: Commit**

```bash
git add tests/integration.rs
git commit -m "test: add integration tests for LaTeX math rendering"
```

---

### Task 7: Clippy and cleanup

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -- -D warnings`

- [ ] **Step 2: Fix any warnings**

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 4: Update README**

Add a "LaTeX 数式" section to README.md:

```markdown
## LaTeX 数式

タイトル・軸ラベル・凡例で `$...$` を使って数式を記述できます。

```gnuplot
set title "$E = mc^2$"
set xlabel "$\\omega$ (rad/s)"
set ylabel "$|\\psi|^2$"
plot sin(x) title "$\\sin(x)$"
```

### サポートされている記法

| 記法 | 例 | 結果 |
|------|-----|------|
| 上付き | `$x^2$`, `$x^{10}$` | x² |
| 下付き | `$x_i$` | xᵢ |
| 分数 | `$\\frac{a}{b}$` | a/b |
| ギリシャ文字 | `$\\alpha$`, `$\\Omega$` | α, Ω |
| アクセント | `$\\hat{x}$`, `$\\bar{x}$` | x̂, x̄ |
| 演算子 | `$\\sum$`, `$\\int$` | Σ, ∫ |
| ローマン体 | `$\\mathrm{Re}$` | Re |
```

- [ ] **Step 5: Commit**

```bash
git add -A
git commit -m "docs: add LaTeX math documentation and fix clippy warnings"
```
