// src/math/parser.rs - Hand-written recursive-descent parser for LaTeX math subset

use super::symbols::{lookup_accent, lookup_symbol, AccentKind};

/// A node in the parsed math AST.
#[derive(Debug, Clone, PartialEq)]
pub enum MathNode {
    /// A single letter rendered in italic math font.
    Char(char),
    /// A Unicode symbol produced by a \command lookup.
    Symbol(char),
    /// A sequence of digits rendered upright.
    Number(String),
    /// An operator character (+, -, =, etc.) rendered upright.
    Operator(char),
    /// A brace-delimited group {abc}.
    Group(Vec<MathNode>),
    /// Superscript: base^exp.
    Superscript(Box<MathNode>, Box<MathNode>),
    /// Subscript: base_sub.
    Subscript(Box<MathNode>, Box<MathNode>),
    /// Combined sub- and superscript: base_sub^sup.
    SubSuperscript(Box<MathNode>, Box<MathNode>, Box<MathNode>),
    /// Fraction \frac{num}{den}.
    Frac(Vec<MathNode>, Vec<MathNode>),
    /// Accent \hat{x} etc.
    Accent(AccentKind, Box<MathNode>),
    /// Roman (non-italic) text \mathrm{Re}.
    TextRoman(String),
}

struct Parser<'a> {
    input: &'a [u8],
    pos: usize,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        Parser {
            input: input.as_bytes(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<u8> {
        self.input.get(self.pos).copied()
    }

    fn advance(&mut self) -> Option<u8> {
        let ch = self.input.get(self.pos).copied();
        if ch.is_some() {
            self.pos += 1;
        }
        ch
    }

    fn skip_spaces(&mut self) {
        while matches!(self.peek(), Some(b' ' | b'\t' | b'\n' | b'\r')) {
            self.pos += 1;
        }
    }

    /// Parse a brace group `{...}`, returning the inner node list.
    fn parse_group(&mut self) -> Result<Vec<MathNode>, String> {
        // Expect opening brace
        match self.advance() {
            Some(b'{') => {}
            other => {
                return Err(format!(
                    "expected '{{' at position {}, got {:?}",
                    self.pos,
                    other.map(|b| b as char)
                ))
            }
        }
        let nodes = self.parse_node_list()?;
        match self.advance() {
            Some(b'}') => {}
            other => {
                return Err(format!(
                    "expected '}}' at position {}, got {:?}",
                    self.pos,
                    other.map(|b| b as char)
                ))
            }
        }
        Ok(nodes)
    }

    /// Parse all nodes until EOF or closing `}`.
    fn parse_node_list(&mut self) -> Result<Vec<MathNode>, String> {
        let mut nodes = Vec::new();
        loop {
            self.skip_spaces();
            match self.peek() {
                None | Some(b'}') => break,
                _ => {
                    let atom = self.parse_atom()?;
                    // Check for ^ and/or _ after the atom
                    let node = self.parse_scripts(atom)?;
                    nodes.push(node);
                }
            }
        }
        Ok(nodes)
    }

    /// Parse a single atom (no scripts yet).
    fn parse_atom(&mut self) -> Result<MathNode, String> {
        self.skip_spaces();
        match self.peek() {
            Some(b'\\') => {
                self.pos += 1; // consume backslash
                let cmd = self.read_command_name();
                self.parse_command(&cmd)
            }
            Some(b'{') => {
                let nodes = self.parse_group()?;
                Ok(MathNode::Group(nodes))
            }
            Some(c) if c.is_ascii_digit() => {
                let mut s = String::new();
                while matches!(self.peek(), Some(d) if d.is_ascii_digit()) {
                    s.push(self.advance().unwrap() as char);
                }
                Ok(MathNode::Number(s))
            }
            Some(c) if c.is_ascii_alphabetic() => {
                let ch = self.advance().unwrap() as char;
                Ok(MathNode::Char(ch))
            }
            Some(b'+' | b'-' | b'=' | b'<' | b'>' | b'(' | b')' | b',' | b'/' | b'!') => {
                let ch = self.advance().unwrap() as char;
                Ok(MathNode::Operator(ch))
            }
            Some(c) => Err(format!(
                "unexpected character '{}' at position {}",
                c as char, self.pos
            )),
            None => Err(format!("unexpected end of input at position {}", self.pos)),
        }
    }

    /// Read an ASCII command name after the backslash.
    fn read_command_name(&mut self) -> String {
        let mut name = String::new();
        // A command name is either one non-alpha char or a run of alpha chars.
        match self.peek() {
            Some(c) if (c as char).is_ascii_alphabetic() => {
                while matches!(self.peek(), Some(c) if (c as char).is_ascii_alphabetic()) {
                    name.push(self.advance().unwrap() as char);
                }
                // Consume a single trailing space after the command name if present
                if self.peek() == Some(b' ') {
                    self.pos += 1;
                }
            }
            Some(_) => {
                name.push(self.advance().unwrap() as char);
            }
            None => {}
        }
        name
    }

    /// Dispatch on a parsed command name.
    fn parse_command(&mut self, cmd: &str) -> Result<MathNode, String> {
        // Special structural commands
        match cmd {
            "frac" => {
                let num = self.parse_group()?;
                let den = self.parse_group()?;
                return Ok(MathNode::Frac(num, den));
            }
            "mathrm" => {
                let nodes = self.parse_group()?;
                let text: String = nodes
                    .iter()
                    .map(|n| match n {
                        MathNode::Char(c) => c.to_string(),
                        MathNode::Number(s) => s.clone(),
                        MathNode::Operator(c) => c.to_string(),
                        _ => String::new(),
                    })
                    .collect();
                return Ok(MathNode::TextRoman(text));
            }
            "text" => {
                let text = self.parse_text_group()?;
                return Ok(MathNode::TextRoman(text));
            }
            _ => {}
        }

        // Accent commands
        if let Some(accent) = lookup_accent(cmd) {
            let inner_nodes = self.parse_group()?;
            // Wrap in Group if more than one node, otherwise unwrap single
            let inner = if inner_nodes.len() == 1 {
                Box::new(inner_nodes.into_iter().next().unwrap())
            } else {
                Box::new(MathNode::Group(inner_nodes))
            };
            return Ok(MathNode::Accent(accent, inner));
        }

        // Symbol lookup
        if let Some(ch) = lookup_symbol(cmd) {
            return Ok(MathNode::Symbol(ch));
        }

        Err(format!("unknown LaTeX command: \\{}", cmd))
    }

    /// After parsing an atom, check for `^` and `_` suffixes.
    fn parse_scripts(&mut self, base: MathNode) -> Result<MathNode, String> {
        self.skip_spaces();
        let has_sub = self.peek() == Some(b'_');
        let has_sup = self.peek() == Some(b'^');

        if !has_sub && !has_sup {
            return Ok(base);
        }

        let mut sub: Option<MathNode> = None;
        let mut sup: Option<MathNode> = None;

        // First script
        match self.peek() {
            Some(b'_') => {
                self.pos += 1;
                sub = Some(self.parse_script_arg()?);
            }
            Some(b'^') => {
                self.pos += 1;
                sup = Some(self.parse_script_arg()?);
            }
            _ => {}
        }

        self.skip_spaces();
        // Second script (if any)
        match self.peek() {
            Some(b'_') if sub.is_none() => {
                self.pos += 1;
                sub = Some(self.parse_script_arg()?);
            }
            Some(b'^') if sup.is_none() => {
                self.pos += 1;
                sup = Some(self.parse_script_arg()?);
            }
            _ => {}
        }

        let base = Box::new(base);
        match (sub, sup) {
            (Some(s), Some(p)) => Ok(MathNode::SubSuperscript(base, Box::new(s), Box::new(p))),
            (Some(s), None) => Ok(MathNode::Subscript(base, Box::new(s))),
            (None, Some(p)) => Ok(MathNode::Superscript(base, Box::new(p))),
            (None, None) => unreachable!(),
        }
    }

    /// Parse a `{...}` group as raw text, preserving spaces.
    /// Used for `\text{...}` where content is literal text.
    fn parse_text_group(&mut self) -> Result<String, String> {
        match self.advance() {
            Some(b'{') => {}
            other => {
                return Err(format!(
                    "expected '{{' at position {}, got {:?}",
                    self.pos,
                    other.map(|b| b as char)
                ))
            }
        }
        let mut text = String::new();
        let mut depth = 1;
        while let Some(b) = self.advance() {
            match b {
                b'{' => { depth += 1; text.push('{'); }
                b'}' => {
                    depth -= 1;
                    if depth == 0 { return Ok(text); }
                    text.push('}');
                }
                _ => text.push(b as char),
            }
        }
        Err(format!("unterminated text group at position {}", self.pos))
    }

    /// Parse the argument to ^ or _: either a brace group or a single atom.
    fn parse_script_arg(&mut self) -> Result<MathNode, String> {
        self.skip_spaces();
        if self.peek() == Some(b'{') {
            let nodes = self.parse_group()?;
            if nodes.len() == 1 {
                Ok(nodes.into_iter().next().unwrap())
            } else {
                Ok(MathNode::Group(nodes))
            }
        } else {
            self.parse_atom()
        }
    }
}

/// Parse a LaTeX math string into a list of `MathNode`s.
///
/// Returns an error string if the input contains unknown commands or
/// malformed syntax.
pub fn parse_math(input: &str) -> Result<Vec<MathNode>, String> {
    let mut parser = Parser::new(input);
    let nodes = parser.parse_node_list()?;
    if parser.pos < parser.input.len() {
        return Err(format!(
            "unexpected character '{}' at position {}",
            parser.input[parser.pos] as char,
            parser.pos
        ));
    }
    Ok(nodes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_chars() {
        let nodes = parse_math("abc").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Char('a'), MathNode::Char('b'), MathNode::Char('c')]
        );
    }

    #[test]
    fn test_numbers() {
        let nodes = parse_math("42").unwrap();
        assert_eq!(nodes, vec![MathNode::Number("42".to_string())]);
    }

    #[test]
    fn test_operators() {
        let nodes = parse_math("+").unwrap();
        assert_eq!(nodes, vec![MathNode::Operator('+')]);

        let nodes = parse_math("-").unwrap();
        assert_eq!(nodes, vec![MathNode::Operator('-')]);

        let nodes = parse_math("=").unwrap();
        assert_eq!(nodes, vec![MathNode::Operator('=')]);
    }

    #[test]
    fn test_superscript_single() {
        let nodes = parse_math("x^2").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Superscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Number("2".to_string()))
            )]
        );
    }

    #[test]
    fn test_superscript_group() {
        let nodes = parse_math("x^{ab}").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Superscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Group(vec![
                    MathNode::Char('a'),
                    MathNode::Char('b')
                ]))
            )]
        );
    }

    #[test]
    fn test_subscript() {
        let nodes = parse_math("x_i").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Subscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Char('i'))
            )]
        );
    }

    #[test]
    fn test_sub_superscript() {
        let nodes = parse_math("x_i^2").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::SubSuperscript(
                Box::new(MathNode::Char('x')),
                Box::new(MathNode::Char('i')),
                Box::new(MathNode::Number("2".to_string()))
            )]
        );
    }

    #[test]
    fn test_greek_symbols() {
        let nodes = parse_math(r"\alpha").unwrap();
        assert_eq!(nodes, vec![MathNode::Symbol('α')]);

        let nodes = parse_math(r"\beta").unwrap();
        assert_eq!(nodes, vec![MathNode::Symbol('β')]);

        let nodes = parse_math(r"\Omega").unwrap();
        assert_eq!(nodes, vec![MathNode::Symbol('Ω')]);
    }

    #[test]
    fn test_frac() {
        let nodes = parse_math(r"\frac{a}{b}").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Frac(
                vec![MathNode::Char('a')],
                vec![MathNode::Char('b')]
            )]
        );
    }

    #[test]
    fn test_accent() {
        let nodes = parse_math(r"\hat{x}").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Accent(
                AccentKind::Hat,
                Box::new(MathNode::Char('x'))
            )]
        );
    }

    #[test]
    fn test_mathrm() {
        let nodes = parse_math(r"\mathrm{Re}").unwrap();
        assert_eq!(nodes, vec![MathNode::TextRoman("Re".to_string())]);
    }

    #[test]
    fn test_complex_expression() {
        // E = m c^2
        let nodes = parse_math("E=mc^2").unwrap();
        assert_eq!(
            nodes,
            vec![
                MathNode::Char('E'),
                MathNode::Operator('='),
                MathNode::Char('m'),
                MathNode::Superscript(
                    Box::new(MathNode::Char('c')),
                    Box::new(MathNode::Number("2".to_string()))
                )
            ]
        );
    }

    #[test]
    fn test_spaces_ignored() {
        let nodes = parse_math("a b").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Char('a'), MathNode::Char('b')]
        );
    }

    #[test]
    fn test_text_command() {
        let nodes = parse_math(r"\text{Hello}").unwrap();
        assert_eq!(nodes, vec![MathNode::TextRoman("Hello".to_string())]);
    }

    #[test]
    fn test_text_with_spaces() {
        let nodes = parse_math(r"\text{a b}").unwrap();
        assert_eq!(nodes, vec![MathNode::TextRoman("a b".to_string())]);
    }

    #[test]
    fn test_mixed_greek_and_text() {
        let nodes = parse_math(r"\alpha x").unwrap();
        assert_eq!(
            nodes,
            vec![MathNode::Symbol('α'), MathNode::Char('x')]
        );
    }
}
