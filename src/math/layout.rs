// src/math/layout.rs - Math layout engine: positions and sizes glyphs

use super::parser::MathNode;

// Layout constants (in em units, relative to base font size)
const CHAR_WIDTH: f64 = 0.55;
const DIGIT_WIDTH: f64 = 0.5;
const OPERATOR_WIDTH: f64 = 0.6;
const OPERATOR_SPACE: f64 = 0.15;
const SYMBOL_WIDTH: f64 = 0.6;
const LARGE_OP_WIDTH: f64 = 0.8;

// Script sizing and positioning
const SCRIPT_SIZE: f64 = 0.7;
const SUPERSCRIPT_Y_SHIFT: f64 = -0.4; // upward (negative y = higher on page)
const SUBSCRIPT_Y_SHIFT: f64 = 0.2;   // downward

// Fraction sizing and positioning
const FRAC_SIZE: f64 = 0.8;
const FRAC_NUM_Y: f64 = -0.7;  // numerator y shift (up)
const FRAC_DEN_Y: f64 = 0.3;   // denominator y shift (down)

/// A single positioned glyph in the layout.
#[derive(Debug, Clone, PartialEq)]
pub struct LayoutGlyph {
    /// The text/character(s) to render.
    pub text: String,
    /// Horizontal position in em units from the left edge.
    pub x: f64,
    /// Vertical position in em units from the baseline (positive = below baseline).
    pub y: f64,
    /// Font size relative to base (1.0 = 100%).
    pub font_size_ratio: f64,
    /// Whether to render in italic style.
    pub italic: bool,
    /// Whether to use a dedicated math font.
    pub is_math_font: bool,
}

/// Result of laying out a math expression.
#[derive(Debug, Clone)]
pub struct LayoutResult {
    pub glyphs: Vec<LayoutGlyph>,
    /// Total width of the laid-out expression in em units.
    pub width: f64,
    /// Total height in em units.
    pub height: f64,
    /// Baseline position from the top in em units.
    pub baseline: f64,
}

/// Internal state threaded through recursive layout calls.
struct LayoutCtx {
    glyphs: Vec<LayoutGlyph>,
    x: f64,
    /// Current y of the baseline for this level.
    y: f64,
    /// Current font size ratio.
    size: f64,
}

impl LayoutCtx {
    fn new(y: f64, size: f64) -> Self {
        LayoutCtx {
            glyphs: Vec::new(),
            x: 0.0,
            y,
            size,
        }
    }

    fn push(&mut self, text: String, width: f64, italic: bool, is_math_font: bool) {
        self.glyphs.push(LayoutGlyph {
            text,
            x: self.x,
            y: self.y,
            font_size_ratio: self.size,
            italic,
            is_math_font,
        });
        self.x += width * self.size;
    }

    /// Layout a slice of nodes, appending glyphs and advancing x.
    fn layout_nodes(&mut self, nodes: &[MathNode]) {
        for node in nodes {
            self.layout_node(node);
        }
    }

    fn layout_node(&mut self, node: &MathNode) {
        match node {
            MathNode::Char(c) => {
                self.push(c.to_string(), CHAR_WIDTH, true, true);
            }
            MathNode::Symbol(c) => {
                // Determine if this is a large operator by checking the char
                let width = if is_large_op(*c) {
                    LARGE_OP_WIDTH
                } else {
                    SYMBOL_WIDTH
                };
                self.push(c.to_string(), width, false, true);
            }
            MathNode::Number(s) => {
                let width = DIGIT_WIDTH * s.len() as f64;
                self.glyphs.push(LayoutGlyph {
                    text: s.clone(),
                    x: self.x,
                    y: self.y,
                    font_size_ratio: self.size,
                    italic: false,
                    is_math_font: true,
                });
                self.x += width * self.size;
            }
            MathNode::Operator(c) => {
                // Add leading space, the operator, then trailing space
                self.x += OPERATOR_SPACE * self.size;
                self.push(c.to_string(), OPERATOR_WIDTH, false, false);
                self.x += OPERATOR_SPACE * self.size;
            }
            MathNode::Group(nodes) => {
                self.layout_nodes(nodes);
            }
            MathNode::Superscript(base, exp) => {
                self.layout_node(base);
                let base_x = self.x;
                let mut sub_ctx = LayoutCtx::new(self.y + SUPERSCRIPT_Y_SHIFT * self.size, self.size * SCRIPT_SIZE);
                sub_ctx.x = base_x;
                sub_ctx.layout_node(exp);
                let new_x = sub_ctx.x;
                self.glyphs.extend(sub_ctx.glyphs);
                self.x = new_x;
            }
            MathNode::Subscript(base, sub) => {
                self.layout_node(base);
                let base_x = self.x;
                let mut sub_ctx = LayoutCtx::new(self.y + SUBSCRIPT_Y_SHIFT * self.size, self.size * SCRIPT_SIZE);
                sub_ctx.x = base_x;
                sub_ctx.layout_node(sub);
                let new_x = sub_ctx.x;
                self.glyphs.extend(sub_ctx.glyphs);
                self.x = new_x;
            }
            MathNode::SubSuperscript(base, sub, sup) => {
                self.layout_node(base);
                let base_x = self.x;

                // Layout subscript
                let mut sub_ctx = LayoutCtx::new(self.y + SUBSCRIPT_Y_SHIFT * self.size, self.size * SCRIPT_SIZE);
                sub_ctx.x = base_x;
                sub_ctx.layout_node(sub);
                let sub_width = sub_ctx.x - base_x;

                // Layout superscript
                let mut sup_ctx = LayoutCtx::new(self.y + SUPERSCRIPT_Y_SHIFT * self.size, self.size * SCRIPT_SIZE);
                sup_ctx.x = base_x;
                sup_ctx.layout_node(sup);
                let sup_width = sup_ctx.x - base_x;

                self.glyphs.extend(sub_ctx.glyphs);
                self.glyphs.extend(sup_ctx.glyphs);
                // Advance x by the wider of the two scripts
                self.x = base_x + sub_width.max(sup_width);
            }
            MathNode::Frac(num, den) => {
                let frac_size = self.size * FRAC_SIZE;

                // Layout numerator
                let mut num_ctx = LayoutCtx::new(self.y + FRAC_NUM_Y * self.size, frac_size);
                num_ctx.x = self.x;
                num_ctx.layout_nodes(num);
                let num_width = num_ctx.x - self.x;

                // Layout denominator
                let mut den_ctx = LayoutCtx::new(self.y + FRAC_DEN_Y * self.size, frac_size);
                den_ctx.x = self.x;
                den_ctx.layout_nodes(den);
                let den_width = den_ctx.x - self.x;

                let frac_width = num_width.max(den_width);

                // Center numerator and denominator within frac_width
                let num_offset = (frac_width - num_width) / 2.0;
                let den_offset = (frac_width - den_width) / 2.0;
                for g in &mut num_ctx.glyphs {
                    g.x += num_offset;
                }
                for g in &mut den_ctx.glyphs {
                    g.x += den_offset;
                }

                // Fraction bar
                self.glyphs.push(LayoutGlyph {
                    text: "—".to_string(),
                    x: self.x,
                    y: self.y,
                    font_size_ratio: self.size,
                    italic: false,
                    is_math_font: false,
                });

                self.glyphs.extend(num_ctx.glyphs);
                self.glyphs.extend(den_ctx.glyphs);
                self.x += frac_width;
            }
            MathNode::Accent(kind, inner) => {
                // Render the base, then append the combining character
                let start_x = self.x;
                let start_glyphs_len = self.glyphs.len();
                self.layout_node(inner);
                // Combine the accent into the first glyph produced by the inner node
                if self.glyphs.len() > start_glyphs_len {
                    self.glyphs[start_glyphs_len].text.push(kind.combining_char());
                } else {
                    // Fallback: push a standalone combining char at start_x
                    self.glyphs.push(LayoutGlyph {
                        text: kind.combining_char().to_string(),
                        x: start_x,
                        y: self.y,
                        font_size_ratio: self.size,
                        italic: false,
                        is_math_font: true,
                    });
                }
            }
            MathNode::TextRoman(s) => {
                let width = CHAR_WIDTH * s.len() as f64;
                self.glyphs.push(LayoutGlyph {
                    text: s.clone(),
                    x: self.x,
                    y: self.y,
                    font_size_ratio: self.size,
                    italic: false,
                    is_math_font: false,
                });
                self.x += width * self.size;
            }
        }
    }
}

/// Returns true if the character is a "large operator" (∑ ∏ ∫).
fn is_large_op(c: char) -> bool {
    matches!(c, '∑' | '∏' | '∫')
}

/// Lay out a slice of `MathNode`s and return positioning information.
pub fn layout_math(nodes: &[MathNode]) -> LayoutResult {
    let mut ctx = LayoutCtx::new(0.0, 1.0);
    ctx.layout_nodes(nodes);

    let width = ctx.x;
    let glyphs = ctx.glyphs;

    // Compute height and baseline from glyph positions.
    // y == 0 is the baseline; negative y = above baseline.
    // We assume each glyph occupies approximately 1em * font_size_ratio in height.
    if glyphs.is_empty() {
        return LayoutResult {
            glyphs,
            width,
            height: 1.0,
            baseline: 0.0,
        };
    }

    let min_y = glyphs
        .iter()
        .map(|g| g.y - g.font_size_ratio) // top of glyph
        .fold(f64::INFINITY, f64::min);
    let max_y = glyphs
        .iter()
        .map(|g| g.y + g.font_size_ratio * 0.2) // bottom descender
        .fold(f64::NEG_INFINITY, f64::max);

    let height = (max_y - min_y).max(1.0);
    // Baseline relative to the top of the bounding box
    let baseline = -min_y;

    LayoutResult {
        glyphs,
        width,
        height,
        baseline,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::math::parser::parse_math;

    fn layout(input: &str) -> LayoutResult {
        let nodes = parse_math(input).expect("parse failed");
        layout_math(&nodes)
    }

    #[test]
    fn test_simple_char() {
        let r = layout("x");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        assert_eq!(g.text, "x");
        assert!(g.italic);
        assert!(g.is_math_font);
        assert!((g.font_size_ratio - 1.0).abs() < 1e-10);
        assert!((g.x - 0.0).abs() < 1e-10);
        assert!((g.y - 0.0).abs() < 1e-10);
    }

    #[test]
    fn test_number_not_italic() {
        let r = layout("42");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        assert_eq!(g.text, "42");
        assert!(!g.italic);
    }

    #[test]
    fn test_operator_spacing() {
        let r = layout("+");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        assert_eq!(g.text, "+");
        // Operator should be placed after leading space
        assert!((g.x - OPERATOR_SPACE).abs() < 1e-10);
        // Total width = space + operator + space
        let expected_width = OPERATOR_SPACE + OPERATOR_WIDTH + OPERATOR_SPACE;
        assert!((r.width - expected_width).abs() < 1e-10);
    }

    #[test]
    fn test_superscript_position() {
        let r = layout("x^2");
        // glyphs: x, then 2 (superscript)
        assert_eq!(r.glyphs.len(), 2);
        let base = &r.glyphs[0];
        let exp = &r.glyphs[1];
        assert_eq!(base.text, "x");
        assert_eq!(exp.text, "2");
        // superscript y should be shifted up
        assert!(exp.y < base.y);
        assert!((exp.y - SUPERSCRIPT_Y_SHIFT).abs() < 1e-10);
        assert!((exp.font_size_ratio - SCRIPT_SIZE).abs() < 1e-10);
    }

    #[test]
    fn test_subscript_position() {
        let r = layout("x_i");
        assert_eq!(r.glyphs.len(), 2);
        let base = &r.glyphs[0];
        let sub = &r.glyphs[1];
        assert_eq!(base.text, "x");
        assert_eq!(sub.text, "i");
        // subscript y should be shifted down
        assert!(sub.y > base.y);
        assert!((sub.y - SUBSCRIPT_Y_SHIFT).abs() < 1e-10);
        assert!((sub.font_size_ratio - SCRIPT_SIZE).abs() < 1e-10);
    }

    #[test]
    fn test_frac_layout() {
        let r = layout(r"\frac{a}{b}");
        // Expect fraction bar glyph + numerator glyph + denominator glyph
        assert!(r.glyphs.len() >= 3);
        // Find the fraction bar
        let bar = r.glyphs.iter().find(|g| g.text == "—");
        assert!(bar.is_some(), "fraction bar glyph not found");
        // Numerator should be above baseline, denominator below
        let num_glyph = r.glyphs.iter().find(|g| g.text == "a").unwrap();
        let den_glyph = r.glyphs.iter().find(|g| g.text == "b").unwrap();
        assert!(num_glyph.y < 0.0, "numerator should be above baseline");
        assert!(den_glyph.y > 0.0, "denominator should be below baseline");
    }

    #[test]
    fn test_symbol_uses_math_font() {
        let r = layout(r"\alpha");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        assert_eq!(g.text, "α");
        assert!(g.is_math_font);
        assert!(!g.italic);
    }

    #[test]
    fn test_mathrm_not_italic() {
        let r = layout(r"\mathrm{Re}");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        assert_eq!(g.text, "Re");
        assert!(!g.italic);
        assert!(!g.is_math_font);
    }

    #[test]
    fn test_accent() {
        let r = layout(r"\hat{x}");
        assert_eq!(r.glyphs.len(), 1);
        let g = &r.glyphs[0];
        // Should be "x" + combining circumflex
        assert!(g.text.starts_with('x'));
        assert!(g.text.contains('\u{0302}'));
    }

    #[test]
    fn test_e_equals_mc_squared() {
        let r = layout("E=mc^2");
        // E, =, m, c (with superscript 2 merged into sub-layout), 2
        // Glyphs: E, =, m, c, 2
        assert!(r.glyphs.len() >= 5);
        // Verify E is at x=0
        let e_glyph = &r.glyphs[0];
        assert_eq!(e_glyph.text, "E");
        assert!((e_glyph.x - 0.0).abs() < 1e-10);
        // Width should be positive
        assert!(r.width > 0.0);
        // Height should be positive
        assert!(r.height > 0.0);
    }
}
