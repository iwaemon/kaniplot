# Core Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a working end-to-end pipeline: parse gnuplot commands → evaluate expressions → render SVG output, so that `plot sin(x) with lines` produces a valid SVG chart.

**Architecture:** One-directional data flow: CLI reads input → Parser produces AST → Engine builds PlotModel (evaluating expressions, computing autoscale/tics) → SVG Renderer outputs SVG. Each layer has a clean interface boundary.

**Tech Stack:** Rust, pest (expression PEG parser), ttf-parser (font metrics for text sizing)

**Scope:** This is Plan 1 of ~5. Covers parser, engine, SVG renderer, and basic CLI. Excludes: data file loading, LaTeX math rendering, PDF/PNG/EPS/Window backends, REPL.

**Spec:** `docs/superpowers/specs/2026-03-19-kaniplot-design.md`

---

## File Structure

```
kaniplot/
├── Cargo.toml
├── src/
│   ├── main.rs                 # CLI entry point (script/pipe mode)
│   ├── lib.rs                  # Library root, re-exports modules
│   ├── parser/
│   │   ├── mod.rs              # Command parser (recursive descent)
│   │   ├── ast.rs              # AST type definitions
│   │   ├── expr_parser.rs      # Expression AST builder from pest pairs
│   │   ├── expr.pest           # PEG grammar for expressions
│   │   └── abbreviation.rs    # Abbreviation resolver
│   ├── engine/
│   │   ├── mod.rs              # PlotModel builder (AST → renderable model)
│   │   ├── model.rs            # PlotModel, Axis, Series structs
│   │   ├── evaluator.rs        # Expression evaluator (f64 arithmetic)
│   │   ├── autoscale.rs        # Auto-range and tick computation
│   │   └── session.rs          # SessionState (cumulative set state)
│   └── renderer/
│       ├── mod.rs              # Renderer trait definition
│       └── svg.rs              # SVG backend
└── tests/
    └── integration/
        ├── mod.rs
        └── plot_basic.rs       # End-to-end: script → SVG output
```

---

### Task 1: Project Scaffold

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/lib.rs`

- [ ] **Step 1: Initialize Cargo project**

```bash
cargo init --name kaniplot
```

- [ ] **Step 2: Set up Cargo.toml with dependencies**

```toml
[package]
name = "kaniplot"
version = "0.1.0"
edition = "2021"

[dependencies]
pest = "2.8"
pest_derive = "2.8"
```

- [ ] **Step 3: Set up lib.rs with module stubs**

```rust
// src/lib.rs
pub mod parser;
pub mod engine;
pub mod renderer;
```

Create module directories and `mod.rs` stubs:

```rust
// src/parser/mod.rs
pub mod ast;
pub mod expr_parser;
mod abbreviation;

// src/engine/mod.rs
pub mod model;
pub mod evaluator;
pub mod autoscale;
pub mod session;

// src/renderer/mod.rs
pub mod svg;
```

- [ ] **Step 4: Set up main.rs minimal entry point**

```rust
// src/main.rs
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Script file mode
        input = std::fs::read_to_string(&args[1]).expect("Cannot read file");
    } else {
        // Pipe mode: read stdin
        io::stdin().read_to_string(&mut input).expect("Cannot read stdin");
    }

    eprintln!("Input: {}", input.trim());
    eprintln!("(kaniplot not yet implemented)");
}
```

- [ ] **Step 5: Verify it compiles**

Run: `cargo build`
Expected: Compiles with no errors (may have unused warnings, that's fine)

- [ ] **Step 6: Add .gitignore and commit**

```gitignore
/target
```

```bash
git add Cargo.toml Cargo.lock src/ .gitignore
git commit -m "chore: scaffold project structure with module stubs"
```

---

### Task 2: AST Type Definitions

**Files:**
- Create: `src/parser/ast.rs`

- [ ] **Step 1: Write test for AST construction**

```rust
// tests at bottom of src/parser/ast.rs
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_command_with_expression() {
        let cmd = Command::Plot(PlotCommand {
            series: vec![PlotSeries::Expression {
                expr: Expr::FuncCall("sin".into(), vec![Expr::Variable("x".into())]),
                style: PlotStyle::default(),
            }],
        });
        match cmd {
            Command::Plot(p) => assert_eq!(p.series.len(), 1),
            _ => panic!("Expected Plot command"),
        }
    }

    #[test]
    fn test_set_command_xrange() {
        let cmd = Command::Set(SetCommand::XRange(Range {
            min: Bound::Value(0.0),
            max: Bound::Value(10.0),
        }));
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_default_plot_style() {
        let style = PlotStyle::default();
        assert_eq!(style.kind, StyleKind::Lines);
        assert!(style.line_color.is_none());
        assert!(style.title.is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parser::ast`
Expected: FAIL — types not defined

- [ ] **Step 3: Implement AST types**

```rust
// src/parser/ast.rs

/// Top-level command
#[derive(Debug, Clone)]
pub enum Command {
    Plot(PlotCommand),
    Set(SetCommand),
    Unset(SetProperty),
    Replot,
    Quit,
}

/// plot command
#[derive(Debug, Clone)]
pub struct PlotCommand {
    pub series: Vec<PlotSeries>,
}

#[derive(Debug, Clone)]
pub enum PlotSeries {
    Expression {
        expr: Expr,
        style: PlotStyle,
    },
    DataFile {
        path: String,
        using: Option<UsingSpec>,
        index: Option<usize>,
        every: Option<usize>,
        style: PlotStyle,
    },
}

#[derive(Debug, Clone)]
pub struct UsingSpec {
    pub columns: Vec<UsingColumn>,
}

#[derive(Debug, Clone)]
pub enum UsingColumn {
    Index(usize),        // plain column number: 1, 2, ...
    Expr(Expr),          // expression: ($1*1000)
}

#[derive(Debug, Clone)]
pub struct PlotStyle {
    pub kind: StyleKind,
    pub line_color: Option<Color>,
    pub line_width: Option<f64>,
    pub dash_type: Option<DashType>,
    pub point_type: Option<u32>,
    pub point_size: Option<f64>,
    pub title: Option<String>,
}

impl Default for PlotStyle {
    fn default() -> Self {
        Self {
            kind: StyleKind::Lines,
            line_color: None,
            line_width: None,
            dash_type: None,
            point_type: None,
            point_size: None,
            title: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Lines,
    Points,
    LinesPoints,
    Dots,
    Impulses,
    Boxes,
    ErrorBars,
    FilledCurves,
}

#[derive(Debug, Clone)]
pub struct Color {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashType {
    Solid,
    Dash,
    Dot,
    DashDot,
    DashDotDot,
}

/// set/unset property identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetProperty {
    XRange,
    YRange,
    Title,
    XLabel,
    YLabel,
    Terminal,
    Output,
    Key,
    XTics,
    YTics,
    Border,
    Font,
    Samples,
}

/// set command
#[derive(Debug, Clone)]
pub enum SetCommand {
    XRange(Range),
    YRange(Range),
    Title(String),
    XLabel(String),
    YLabel(String),
    Terminal(TerminalType),
    Output(String),
    Key(KeyOptions),
    XTics(TicsSpec),
    YTics(TicsSpec),
    Border(u32),
    Font(String),
    Samples(usize),
}

#[derive(Debug, Clone)]
pub struct Range {
    pub min: Bound,
    pub max: Bound,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Bound {
    Auto,
    Value(f64),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalType {
    Svg,
    Pdf,
    Png,
    Eps,
    Window,
}

#[derive(Debug, Clone)]
pub struct KeyOptions {
    pub visible: bool,
    pub position: KeyPosition,
}

impl Default for KeyOptions {
    fn default() -> Self {
        Self {
            visible: true,
            position: KeyPosition::TopRight,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPosition {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}

#[derive(Debug, Clone)]
pub enum TicsSpec {
    Auto,
    Increment { start: f64, step: f64, end: Option<f64> },
    List(Vec<(f64, Option<String>)>),
}

/// Expression AST
#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Variable(String),
    ColumnRef(usize),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    FuncCall(String, Vec<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,
    Eq,
    Ne,
    Lt,
    Gt,
    Le,
    Ge,
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,
    Not,
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib parser::ast`
Expected: All 3 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/ast.rs
git commit -m "feat: define AST types for commands, expressions, and styles"
```

---

### Task 3: Expression PEG Grammar & Parser

**Files:**
- Create: `src/parser/expr.pest`
- Create: `src/parser/expr_parser.rs`

- [ ] **Step 1: Write tests for expression parsing**

```rust
// src/parser/expr_parser.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_parse_number() {
        let expr = parse_expr("42").unwrap();
        match expr {
            Expr::Number(n) => assert_eq!(n, 42.0),
            _ => panic!("Expected Number, got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_variable() {
        let expr = parse_expr("x").unwrap();
        match expr {
            Expr::Variable(name) => assert_eq!(name, "x"),
            _ => panic!("Expected Variable"),
        }
    }

    #[test]
    fn test_parse_function_call() {
        let expr = parse_expr("sin(x)").unwrap();
        match expr {
            Expr::FuncCall(name, args) => {
                assert_eq!(name, "sin");
                assert_eq!(args.len(), 1);
            }
            _ => panic!("Expected FuncCall"),
        }
    }

    #[test]
    fn test_parse_binary_op() {
        let expr = parse_expr("x + 1").unwrap();
        match expr {
            Expr::BinaryOp(_, BinOp::Add, _) => {}
            _ => panic!("Expected BinaryOp Add, got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_power() {
        // x**2 should parse as BinaryOp(Pow)
        let expr = parse_expr("x**2").unwrap();
        match expr {
            Expr::BinaryOp(_, BinOp::Pow, _) => {}
            _ => panic!("Expected BinaryOp Pow, got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_unary_neg() {
        let expr = parse_expr("-x").unwrap();
        match expr {
            Expr::UnaryOp(UnaryOp::Neg, _) => {}
            _ => panic!("Expected UnaryOp Neg, got {:?}", expr),
        }
    }

    #[test]
    fn test_parse_complex_expression() {
        // sin(x) + x**2 should parse without error
        let expr = parse_expr("sin(x) + x**2");
        assert!(expr.is_ok(), "Failed to parse: {:?}", expr.err());
    }

    #[test]
    fn test_parse_nested_functions() {
        let expr = parse_expr("sin(cos(x))");
        assert!(expr.is_ok());
    }

    #[test]
    fn test_parse_ternary() {
        let expr = parse_expr("x > 0 ? x : -x");
        assert!(expr.is_ok());
        match expr.unwrap() {
            Expr::Ternary(_, _, _) => {}
            other => panic!("Expected Ternary, got {:?}", other),
        }
    }

    #[test]
    fn test_parse_column_ref() {
        let expr = parse_expr("$1 + $2").unwrap();
        match expr {
            Expr::BinaryOp(lhs, BinOp::Add, rhs) => {
                match *lhs {
                    Expr::ColumnRef(1) => {}
                    _ => panic!("Expected ColumnRef(1)"),
                }
                match *rhs {
                    Expr::ColumnRef(2) => {}
                    _ => panic!("Expected ColumnRef(2)"),
                }
            }
            _ => panic!("Expected BinaryOp"),
        }
    }

    #[test]
    fn test_precedence_mul_over_add() {
        // 1 + 2 * 3 should be 1 + (2 * 3), not (1 + 2) * 3
        let expr = parse_expr("1 + 2 * 3").unwrap();
        match expr {
            Expr::BinaryOp(_, BinOp::Add, rhs) => {
                match *rhs {
                    Expr::BinaryOp(_, BinOp::Mul, _) => {}
                    _ => panic!("Expected Mul on rhs"),
                }
            }
            _ => panic!("Expected Add at top"),
        }
    }

    #[test]
    fn test_precedence_pow_right_assoc() {
        // 2**3**4 should be 2**(3**4)
        let expr = parse_expr("2**3**4").unwrap();
        match expr {
            Expr::BinaryOp(_, BinOp::Pow, rhs) => {
                match *rhs {
                    Expr::BinaryOp(_, BinOp::Pow, _) => {}
                    _ => panic!("Expected Pow on rhs (right-assoc)"),
                }
            }
            _ => panic!("Expected Pow at top"),
        }
    }

    #[test]
    fn test_mixed_mul_and_pow() {
        // x * y**2 should be x * (y**2), not (x*y)**2
        let expr = parse_expr("x * y**2").unwrap();
        match expr {
            Expr::BinaryOp(_, BinOp::Mul, rhs) => {
                match *rhs {
                    Expr::BinaryOp(_, BinOp::Pow, _) => {}
                    _ => panic!("Expected Pow on rhs"),
                }
            }
            _ => panic!("Expected Mul at top"),
        }
    }

    #[test]
    fn test_atan2_two_args() {
        let expr = parse_expr("atan2(y, x)").unwrap();
        match expr {
            Expr::FuncCall(name, args) => {
                assert_eq!(name, "atan2");
                assert_eq!(args.len(), 2);
            }
            _ => panic!("Expected FuncCall"),
        }
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parser::expr_parser`
Expected: FAIL — no `parse_expr` function

- [ ] **Step 3: Write the PEG grammar**

```pest
// src/parser/expr.pest

// Whitespace — silently consumed between tokens
WHITESPACE = _{ " " | "\t" }

// Entry rule
expression = { SOI ~ ternary ~ EOI }

// Ternary: expr ? expr : expr (right-associative)
ternary = { or_expr ~ ("?" ~ ternary ~ ":" ~ ternary)? }

// Logical OR
or_expr = { and_expr ~ ("||" ~ and_expr)* }

// Logical AND
and_expr = { comparison ~ ("&&" ~ comparison)* }

// Comparison
comparison = { additive ~ (comp_op ~ additive)? }
comp_op = { "==" | "!=" | "<=" | ">=" | "<" | ">" }

// Addition / Subtraction
additive = { multiplicative ~ ((add_op) ~ multiplicative)* }
add_op = { "+" | "-" }

// Multiplication / Division / Modulo
multiplicative = { power ~ ((mul_op) ~ power)* }
mul_op = { !("**") ~ "*" | "/" | "%" }

// Power (right-associative)
power = { unary ~ ("**" ~ power)? }

// Unary
unary = { unary_op? ~ atom }
unary_op = { "-" | "!" }

// Atom
atom = {
    func_call
    | column_ref
    | number
    | variable
    | "(" ~ ternary ~ ")"
}

// Function call: sin(x), atan2(y, x)
func_call = { identifier ~ "(" ~ arg_list ~ ")" }
arg_list = { ternary ~ ("," ~ ternary)* }

// Column reference: $1, $2
column_ref = { "$" ~ ASCII_DIGIT+ }

// Number: 42, 3.14, 1e-3, .5
number = @{
    ASCII_DIGIT+ ~ ("." ~ ASCII_DIGIT*)? ~ (("e" | "E") ~ ("+" | "-")? ~ ASCII_DIGIT+)?
    | "." ~ ASCII_DIGIT+ ~ (("e" | "E") ~ ("+" | "-")? ~ ASCII_DIGIT+)?
}

// Variable / constant name
variable = @{ identifier }
identifier = @{ ASCII_ALPHA ~ (ASCII_ALPHANUMERIC | "_")* }
```

- [ ] **Step 4: Implement `parse_expr` function**

```rust
// src/parser/expr_parser.rs
use pest::Parser;
use pest_derive::Parser;

use crate::parser::ast::*;

#[derive(Parser)]
#[grammar = "parser/expr.pest"]
struct ExprParser;

/// Parse an expression string into an Expr AST.
pub fn parse_expr(input: &str) -> Result<Expr, String> {
    let pairs = ExprParser::parse(Rule::expression, input)
        .map_err(|e| format!("Parse error: {e}"))?;

    let expr_pair = pairs
        .into_iter()
        .next()
        .unwrap() // expression rule
        .into_inner()
        .find(|p| p.as_rule() != Rule::EOI)
        .unwrap();

    build_ternary(expr_pair)
}

fn build_ternary(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let condition = build_or_expr(inner.next().unwrap())?;

    if let Some(true_branch) = inner.next() {
        let false_branch = inner.next().unwrap();
        Ok(Expr::Ternary(
            Box::new(condition),
            Box::new(build_ternary(true_branch)?),
            Box::new(build_ternary(false_branch)?),
        ))
    } else {
        Ok(condition)
    }
}

fn build_or_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let mut lhs = build_and_expr(inner.next().unwrap())?;
    while let Some(rhs_pair) = inner.next() {
        let rhs = build_and_expr(rhs_pair)?;
        lhs = Expr::BinaryOp(Box::new(lhs), BinOp::Or, Box::new(rhs));
    }
    Ok(lhs)
}

fn build_and_expr(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let mut lhs = build_comparison(inner.next().unwrap())?;
    while let Some(rhs_pair) = inner.next() {
        let rhs = build_comparison(rhs_pair)?;
        lhs = Expr::BinaryOp(Box::new(lhs), BinOp::And, Box::new(rhs));
    }
    Ok(lhs)
}

fn build_comparison(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let lhs = build_additive(inner.next().unwrap())?;

    if let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "==" => BinOp::Eq,
            "!=" => BinOp::Ne,
            "<"  => BinOp::Lt,
            ">"  => BinOp::Gt,
            "<=" => BinOp::Le,
            ">=" => BinOp::Ge,
            s    => return Err(format!("Unknown comparison op: {s}")),
        };
        let rhs = build_additive(inner.next().unwrap())?;
        Ok(Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs)))
    } else {
        Ok(lhs)
    }
}

fn build_additive(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let mut lhs = build_multiplicative(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "+" => BinOp::Add,
            "-" => BinOp::Sub,
            s   => return Err(format!("Unknown additive op: {s}")),
        };
        let rhs = build_multiplicative(inner.next().unwrap())?;
        lhs = Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs));
    }
    Ok(lhs)
}

fn build_multiplicative(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let mut lhs = build_power(inner.next().unwrap())?;
    while let Some(op_pair) = inner.next() {
        let op = match op_pair.as_str() {
            "*" => BinOp::Mul,
            "/" => BinOp::Div,
            "%" => BinOp::Mod,
            s   => return Err(format!("Unknown multiplicative op: {s}")),
        };
        let rhs = build_power(inner.next().unwrap())?;
        lhs = Expr::BinaryOp(Box::new(lhs), op, Box::new(rhs));
    }
    Ok(lhs)
}

fn build_power(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let base = build_unary(inner.next().unwrap())?;

    if let Some(exp_pair) = inner.next() {
        let exp = build_power(exp_pair)?; // right-associative recursion
        Ok(Expr::BinaryOp(Box::new(base), BinOp::Pow, Box::new(exp)))
    } else {
        Ok(base)
    }
}

fn build_unary(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let mut inner = pair.into_inner();
    let first = inner.next().unwrap();

    if first.as_rule() == Rule::unary_op {
        let op = match first.as_str() {
            "-" => UnaryOp::Neg,
            "!" => UnaryOp::Not,
            s   => return Err(format!("Unknown unary op: {s}")),
        };
        let operand = build_atom(inner.next().unwrap())?;
        Ok(Expr::UnaryOp(op, Box::new(operand)))
    } else {
        build_atom(first)
    }
}

fn build_atom(pair: pest::iterators::Pair<Rule>) -> Result<Expr, String> {
    let inner = pair.into_inner().next().unwrap();
    match inner.as_rule() {
        Rule::number => {
            let n: f64 = inner.as_str().parse().map_err(|e| format!("{e}"))?;
            Ok(Expr::Number(n))
        }
        Rule::variable => Ok(Expr::Variable(inner.as_str().to_string())),
        Rule::column_ref => {
            let digits = &inner.as_str()[1..]; // skip '$'
            let idx: usize = digits.parse().map_err(|e| format!("{e}"))?;
            Ok(Expr::ColumnRef(idx))
        }
        Rule::func_call => {
            let mut fc_inner = inner.into_inner();
            let name = fc_inner.next().unwrap().as_str().to_string();
            let args_pair = fc_inner.next().unwrap();
            let args: Result<Vec<Expr>, String> = args_pair
                .into_inner()
                .map(build_ternary)
                .collect();
            Ok(Expr::FuncCall(name, args?))
        }
        Rule::ternary => build_ternary(inner),
        r => Err(format!("Unexpected rule in atom: {r:?}")),
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib parser::expr_parser`
Expected: All 14 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/parser/expr.pest src/parser/expr_parser.rs
git commit -m "feat: implement expression PEG grammar and parser"
```

---

### Task 4: Expression Evaluator

**Files:**
- Create: `src/engine/evaluator.rs`

- [ ] **Step 1: Write tests**

```rust
// src/engine/evaluator.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expr_parser::parse_expr;
    use std::f64::consts::PI;

    fn eval(input: &str, x: f64) -> f64 {
        let expr = parse_expr(input).unwrap();
        evaluate(&expr, x).unwrap()
    }

    #[test]
    fn test_constant() {
        assert_eq!(eval("42", 0.0), 42.0);
    }

    #[test]
    fn test_variable_x() {
        assert_eq!(eval("x", 3.0), 3.0);
    }

    #[test]
    fn test_pi_constant() {
        assert!((eval("pi", 0.0) - PI).abs() < 1e-10);
    }

    #[test]
    fn test_arithmetic() {
        assert_eq!(eval("2 + 3 * 4", 0.0), 14.0);
    }

    #[test]
    fn test_power() {
        assert_eq!(eval("2**10", 0.0), 1024.0);
    }

    #[test]
    fn test_sin() {
        assert!((eval("sin(pi)", 0.0)).abs() < 1e-10);
    }

    #[test]
    fn test_cos() {
        assert!((eval("cos(0)", 0.0) - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_nested() {
        // sin(x)**2 + cos(x)**2 == 1
        let val = eval("sin(x)**2 + cos(x)**2", 1.5);
        assert!((val - 1.0).abs() < 1e-10);
    }

    #[test]
    fn test_unary_neg() {
        assert_eq!(eval("-5", 0.0), -5.0);
    }

    #[test]
    fn test_ternary() {
        assert_eq!(eval("x > 0 ? 1 : -1", 5.0), 1.0);
        assert_eq!(eval("x > 0 ? 1 : -1", -5.0), -1.0);
    }

    #[test]
    fn test_atan2() {
        let val = eval("atan2(1, 1)", 0.0);
        assert!((val - PI / 4.0).abs() < 1e-10);
    }

    #[test]
    fn test_abs() {
        assert_eq!(eval("abs(-7)", 0.0), 7.0);
    }

    #[test]
    fn test_sqrt() {
        assert_eq!(eval("sqrt(9)", 0.0), 3.0);
    }

    #[test]
    fn test_log_exp() {
        assert!((eval("log(exp(1))", 0.0) - 1.0).abs() < 1e-10);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::evaluator`
Expected: FAIL

- [ ] **Step 3: Implement evaluator**

```rust
// src/engine/evaluator.rs
use crate::parser::ast::*;
use std::f64::consts::{E, PI};

/// Evaluate an expression with the given x value.
pub fn evaluate(expr: &Expr, x: f64) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name) => match name.as_str() {
            "x" => Ok(x),
            "pi" => Ok(PI),
            "e" => Ok(E),
            _ => Err(format!("Unknown variable: {name}")),
        },
        Expr::ColumnRef(idx) => Err(format!("Column ref ${idx} not supported in function plot")),
        Expr::UnaryOp(op, operand) => {
            let val = evaluate(operand, x)?;
            match op {
                UnaryOp::Neg => Ok(-val),
                UnaryOp::Not => Ok(if val == 0.0 { 1.0 } else { 0.0 }),
            }
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let l = evaluate(lhs, x)?;
            let r = evaluate(rhs, x)?;
            Ok(match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => l / r,
                BinOp::Mod => l % r,
                BinOp::Pow => l.powf(r),
                BinOp::Eq => if (l - r).abs() < f64::EPSILON { 1.0 } else { 0.0 },
                BinOp::Ne => if (l - r).abs() >= f64::EPSILON { 1.0 } else { 0.0 },
                BinOp::Lt => if l < r { 1.0 } else { 0.0 },
                BinOp::Gt => if l > r { 1.0 } else { 0.0 },
                BinOp::Le => if l <= r { 1.0 } else { 0.0 },
                BinOp::Ge => if l >= r { 1.0 } else { 0.0 },
                BinOp::And => if l != 0.0 && r != 0.0 { 1.0 } else { 0.0 },
                BinOp::Or => if l != 0.0 || r != 0.0 { 1.0 } else { 0.0 },
            })
        }
        Expr::FuncCall(name, args) => {
            let vals: Result<Vec<f64>, _> = args.iter().map(|a| evaluate(a, x)).collect();
            let vals = vals?;
            call_builtin(name, &vals)
        }
        Expr::Ternary(cond, true_branch, false_branch) => {
            let c = evaluate(cond, x)?;
            if c != 0.0 {
                evaluate(true_branch, x)
            } else {
                evaluate(false_branch, x)
            }
        }
    }
}

fn call_builtin(name: &str, args: &[f64]) -> Result<f64, String> {
    match (name, args) {
        ("sin", [a])   => Ok(a.sin()),
        ("cos", [a])   => Ok(a.cos()),
        ("tan", [a])   => Ok(a.tan()),
        ("asin", [a])  => Ok(a.asin()),
        ("acos", [a])  => Ok(a.acos()),
        ("atan", [a])  => Ok(a.atan()),
        ("atan2", [a, b]) => Ok(a.atan2(*b)),
        ("exp", [a])   => Ok(a.exp()),
        ("log", [a])   => Ok(a.ln()),
        ("log10", [a]) => Ok(a.log10()),
        ("sqrt", [a])  => Ok(a.sqrt()),
        ("abs", [a])   => Ok(a.abs()),
        ("ceil", [a])  => Ok(a.ceil()),
        ("floor", [a]) => Ok(a.floor()),
        ("int", [a])   => Ok(a.trunc()),
        _ => Err(format!("Unknown function or wrong arg count: {name}({})", args.len())),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib engine::evaluator`
Expected: All 15 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/evaluator.rs
git commit -m "feat: implement expression evaluator with builtins (sin, cos, etc.)"
```

---

### Task 5: Abbreviation Resolver

**Files:**
- Create: `src/parser/abbreviation.rs`

- [ ] **Step 1: Write tests**

```rust
// src/parser/abbreviation.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let cmds = &["plot", "print", "pause"];
        assert_eq!(resolve("plot", cmds).unwrap(), "plot");
    }

    #[test]
    fn test_unique_prefix() {
        let cmds = &["plot", "print", "pause"];
        assert_eq!(resolve("pl", cmds).unwrap(), "plot");
    }

    #[test]
    fn test_single_char_unique() {
        let cmds = &["plot", "set", "replot"];
        assert_eq!(resolve("p", cmds).unwrap(), "plot");
        assert_eq!(resolve("s", cmds).unwrap(), "set");
        assert_eq!(resolve("r", cmds).unwrap(), "replot");
    }

    #[test]
    fn test_ambiguous() {
        let cmds = &["plot", "print", "pause"];
        let err = resolve("p", cmds).unwrap_err();
        assert!(err.contains("Ambiguous"));
    }

    #[test]
    fn test_no_match() {
        let cmds = &["plot", "set"];
        let err = resolve("xyz", cmds).unwrap_err();
        assert!(err.contains("Unknown"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parser::abbreviation`
Expected: FAIL

- [ ] **Step 3: Implement abbreviation resolver**

```rust
// src/parser/abbreviation.rs

/// Resolve an abbreviated command or option name against a list of candidates.
/// Returns the full name if exactly one candidate matches, otherwise an error.
pub fn resolve<'a>(input: &str, candidates: &[&'a str]) -> Result<&'a str, String> {
    // Exact match first
    if let Some(&exact) = candidates.iter().find(|&&c| c == input) {
        return Ok(exact);
    }

    let matches: Vec<&str> = candidates
        .iter()
        .filter(|&&c| c.starts_with(input))
        .copied()
        .collect();

    match matches.len() {
        0 => Err(format!("Unknown command: '{input}'")),
        1 => Ok(matches[0]),
        _ => Err(format!(
            "Ambiguous abbreviation '{input}': could be {}",
            matches.join(", ")
        )),
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib parser::abbreviation`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/abbreviation.rs
git commit -m "feat: implement gnuplot-style abbreviation resolver"
```

---

### Task 6: Command Parser (Recursive Descent)

**Files:**
- Create: `src/parser/mod.rs` (replace stub)

- [ ] **Step 1: Write tests**

```rust
// src/parser/mod.rs — tests at bottom
// Note: parse_command returns Result<Option<Command>, String>.
// Use .unwrap().unwrap() for tests expecting Some(Command).
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_parse_plot_sin_x() {
        let cmd = parse_command("plot sin(x)").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => {
                assert_eq!(p.series.len(), 1);
                match &p.series[0] {
                    PlotSeries::Expression { style, .. } => {
                        assert_eq!(style.kind, StyleKind::Lines);
                    }
                    _ => panic!("Expected Expression"),
                }
            }
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_with_style() {
        let cmd = parse_command("plot sin(x) with points").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.kind, StyleKind::Points);
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_multiple_series() {
        let cmd = parse_command("plot sin(x), cos(x) with points").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => assert_eq!(p.series.len(), 2),
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_with_title() {
        let cmd = parse_command(r#"plot sin(x) title "sine""#).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.title.as_deref(), Some("sine"));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_abbreviation() {
        let cmd = parse_command("p sin(x) w l").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => {
                assert_eq!(p.series.len(), 1);
                match &p.series[0] {
                    PlotSeries::Expression { style, .. } => {
                        assert_eq!(style.kind, StyleKind::Lines);
                    }
                    _ => panic!("Expected Expression"),
                }
            }
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_set_xrange() {
        let cmd = parse_command("set xrange [0:10]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_set_xrange_auto() {
        let cmd = parse_command("set xrange [*:10]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Auto);
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_set_title() {
        let cmd = parse_command(r#"set title "My Plot""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Title(t)) => assert_eq!(t, "My Plot"),
            _ => panic!("Expected Set Title"),
        }
    }

    #[test]
    fn test_parse_set_xlabel() {
        let cmd = parse_command(r#"set xlabel "$x$""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XLabel(l)) => assert_eq!(l, "$x$"),
            _ => panic!("Expected Set XLabel"),
        }
    }

    #[test]
    fn test_parse_set_terminal() {
        let cmd = parse_command("set terminal svg").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Terminal(t)) => assert_eq!(t, TerminalType::Svg),
            _ => panic!("Expected Set Terminal"),
        }
    }

    #[test]
    fn test_parse_set_output() {
        let cmd = parse_command(r#"set output "graph.svg""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Output(o)) => assert_eq!(o, "graph.svg"),
            _ => panic!("Expected Set Output"),
        }
    }

    #[test]
    fn test_parse_set_samples() {
        let cmd = parse_command("set samples 500").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Samples(n)) => assert_eq!(n, 500),
            _ => panic!("Expected Set Samples"),
        }
    }

    #[test]
    fn test_parse_set_abbreviation() {
        let cmd = parse_command("se xra [0:1]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(1.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_replot() {
        let cmd = parse_command("replot").unwrap().unwrap();
        assert!(matches!(cmd, Command::Replot));
    }

    #[test]
    fn test_parse_rep_abbreviation() {
        let cmd = parse_command("rep").unwrap().unwrap();
        assert!(matches!(cmd, Command::Replot));
    }

    #[test]
    fn test_parse_quit() {
        assert!(matches!(parse_command("quit").unwrap().unwrap(), Command::Quit));
        assert!(matches!(parse_command("q").unwrap().unwrap(), Command::Quit));
        assert!(matches!(parse_command("exit").unwrap().unwrap(), Command::Quit));
    }

    #[test]
    fn test_parse_unset_title() {
        let cmd = parse_command("unset title").unwrap().unwrap();
        match cmd {
            Command::Unset(SetProperty::Title) => {}
            _ => panic!("Expected Unset Title"),
        }
    }

    #[test]
    fn test_parse_set_key_position() {
        let cmd = parse_command("set key top left").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Key(k)) => {
                assert!(k.visible);
                assert_eq!(k.position, KeyPosition::TopLeft);
            }
            _ => panic!("Expected Set Key"),
        }
    }

    #[test]
    fn test_parse_linewidth() {
        let cmd = parse_command("plot sin(x) linewidth 2.5").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.line_width, Some(2.5));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_linecolor() {
        let cmd = parse_command(r#"plot sin(x) linecolor rgb "#FF0000""#).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    let c = style.line_color.as_ref().unwrap();
                    assert_eq!((c.r, c.g, c.b), (255, 0, 0));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_data_file() {
        let cmd = parse_command(r#"plot "data.txt" using 1:2 with lines"#).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::DataFile { path, using, style, .. } => {
                    assert_eq!(path, "data.txt");
                    assert!(using.is_some());
                    assert_eq!(style.kind, StyleKind::Lines);
                }
                _ => panic!("Expected DataFile"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_set_border() {
        let cmd = parse_command("set border 3").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Border(b)) => assert_eq!(b, 3),
            _ => panic!("Expected Set Border"),
        }
    }

    #[test]
    fn test_unknown_command_error() {
        let result = parse_command("foobar");
        assert!(result.is_err());
    }

    #[test]
    fn test_comment_ignored() {
        assert!(parse_command("# this is a comment").unwrap().is_none());
    }

    #[test]
    fn test_empty_line() {
        assert!(parse_command("").unwrap().is_none());
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib parser::tests`
Expected: FAIL

- [ ] **Step 3: Implement command parser**

The command parser is a hand-written recursive descent parser. It:
1. Tokenizes input into words (respecting quoted strings)
2. Resolves abbreviations for commands and options
3. Dispatches to sub-parsers for `plot`, `set`, etc.

Implementation structure in `src/parser/mod.rs`:

```rust
// src/parser/mod.rs
pub mod ast;
pub mod expr_parser;
pub mod abbreviation;

use ast::*;
use abbreviation::resolve;
use expr_parser::parse_expr;

/// Parse a single command line into a Command.
/// Returns Ok(None) for empty lines and comments.
pub fn parse_command(input: &str) -> Result<Option<Command>, String> {
    let input = input.trim();
    if input.is_empty() || input.starts_with('#') {
        return Ok(None);
    }

    let mut tokens = Tokenizer::new(input);
    let cmd_word = tokens.next_word().ok_or("Empty command")?;

    // "exit" is not part of the normal abbreviation table
    if cmd_word == "exit" {
        return Ok(Some(Command::Quit));
    }

    let cmds = &["plot", "set", "unset", "replot", "quit"];
    let cmd_name = resolve(&cmd_word, cmds)?;

    match cmd_name {
        "plot"   => parse_plot(&mut tokens).map(|c| Some(Command::Plot(c))),
        "set"    => parse_set(&mut tokens).map(|c| Some(Command::Set(c))),
        "unset"  => parse_unset(&mut tokens).map(|p| Some(Command::Unset(p))),
        "replot" => Ok(Some(Command::Replot)),
        "quit"   => Ok(Some(Command::Quit)),
        _        => Err(format!("Unknown command: {cmd_name}")),
    }
}

/// Parse multiple lines (script).
pub fn parse_script(input: &str) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    for (line_num, line) in input.lines().enumerate() {
        match parse_command(line) {
            Ok(Some(cmd)) => commands.push(cmd),
            Ok(None) => {} // empty line or comment
            Err(e) => return Err(format!("Line {}: {e}", line_num + 1)),
        }
    }
    Ok(commands)
}

// --- Tokenizer ---

struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len()
            && self.input.as_bytes()[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn next_word(&mut self) -> Option<String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_whitespace() || ch == b',' || ch == b'[' {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            None
        } else {
            Some(self.input[start..self.pos].to_lowercase())
        }
    }

    fn next_quoted_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() || self.input.as_bytes()[self.pos] != b'"' {
            return Err("Expected quoted string".into());
        }
        self.pos += 1; // skip opening quote
        let start = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'"' {
            self.pos += 1;
        }
        if self.pos >= self.input.len() {
            return Err("Unterminated string".into());
        }
        let s = self.input[start..self.pos].to_string();
        self.pos += 1; // skip closing quote
        Ok(s)
    }

    fn next_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_digit() || ch == b'.' || ch == b'-' || ch == b'+' || ch == b'e' || ch == b'E' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {e}"))
    }

    fn peek_char(&self) -> Option<char> {
        let rem = self.remaining().trim_start();
        rem.chars().next()
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len_utf8();
            Ok(())
        } else {
            Err(format!("Expected '{expected}' at position {}", self.pos))
        }
    }
}

// --- plot parser ---

fn parse_plot(tokens: &mut Tokenizer) -> Result<PlotCommand, String> {
    let mut series = Vec::new();

    loop {
        tokens.skip_whitespace();
        if tokens.remaining().is_empty() {
            break;
        }

        let s = parse_plot_series(tokens)?;
        series.push(s);

        tokens.skip_whitespace();
        if tokens.remaining().starts_with(',') {
            tokens.pos += 1; // consume comma
        } else {
            break;
        }
    }

    if series.is_empty() {
        return Err("plot: expected at least one series".into());
    }

    Ok(PlotCommand { series })
}

fn parse_plot_series(tokens: &mut Tokenizer) -> Result<PlotSeries, String> {
    tokens.skip_whitespace();

    // Check if it's a data file (starts with ")
    if tokens.peek_char() == Some('"') {
        let path = tokens.next_quoted_string()?;
        let mut using = None;
        let mut index = None;
        let mut every = None;
        let mut style = PlotStyle::default();

        // Parse optional modifiers
        loop {
            tokens.skip_whitespace();
            let remaining = tokens.remaining();
            if remaining.is_empty() || remaining.starts_with(',') {
                break;
            }

            let save_pos = tokens.pos;
            if let Some(word) = tokens.next_word() {
                match try_resolve_plot_option(&word) {
                    Some("using") => {
                        using = Some(parse_using_spec(tokens)?);
                    }
                    Some("index") => {
                        index = Some(tokens.next_number()? as usize);
                    }
                    Some("every") => {
                        every = Some(tokens.next_number()? as usize);
                    }
                    Some("with") => {
                        style.kind = parse_style_kind(tokens)?;
                    }
                    Some("title") => {
                        style.title = Some(tokens.next_quoted_string()?);
                    }
                    Some("linewidth") | Some("lw") => {
                        style.line_width = Some(tokens.next_number()?);
                    }
                    Some("linecolor") | Some("lc") => {
                        style.line_color = Some(parse_color(tokens)?);
                    }
                    Some("pointtype") | Some("pt") => {
                        style.point_type = Some(tokens.next_number()? as u32);
                    }
                    Some("pointsize") | Some("ps") => {
                        style.point_size = Some(tokens.next_number()?);
                    }
                    _ => {
                        tokens.pos = save_pos;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(PlotSeries::DataFile { path, using, index, every, style })
    } else {
        // Expression plot — collect text until modifier keyword or comma
        let expr_str = collect_expression_text(tokens);
        let expr = parse_expr(&expr_str)?;
        let mut style = PlotStyle::default();

        // Parse optional modifiers
        loop {
            tokens.skip_whitespace();
            let remaining = tokens.remaining();
            if remaining.is_empty() || remaining.starts_with(',') {
                break;
            }

            let save_pos = tokens.pos;
            if let Some(word) = tokens.next_word() {
                match try_resolve_plot_option(&word) {
                    Some("with") => {
                        style.kind = parse_style_kind(tokens)?;
                    }
                    Some("title") => {
                        style.title = Some(tokens.next_quoted_string()?);
                    }
                    Some("linewidth") | Some("lw") => {
                        style.line_width = Some(tokens.next_number()?);
                    }
                    Some("linecolor") | Some("lc") => {
                        style.line_color = Some(parse_color(tokens)?);
                    }
                    Some("pointtype") | Some("pt") => {
                        style.point_type = Some(tokens.next_number()? as u32);
                    }
                    Some("pointsize") | Some("ps") => {
                        style.point_size = Some(tokens.next_number()?);
                    }
                    _ => {
                        tokens.pos = save_pos;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(PlotSeries::Expression { expr, style })
    }
}

/// Collect expression text until we hit a plot modifier keyword or comma.
fn collect_expression_text(tokens: &mut Tokenizer) -> String {
    tokens.skip_whitespace();
    let start = tokens.pos;

    // Plot modifier keywords that terminate expression collection
    let modifiers = [
        "with", "w", "title", "t", "linewidth", "lw",
        "linecolor", "lc", "pointtype", "pt", "pointsize", "ps",
        "using", "u", "index", "every",
    ];

    loop {
        tokens.skip_whitespace();
        let remaining = tokens.remaining();
        if remaining.is_empty() || remaining.starts_with(',') {
            break;
        }

        // Check if current word is a modifier keyword
        let save_pos = tokens.pos;
        if let Some(word) = tokens.next_word() {
            if modifiers.contains(&word.as_str()) || try_resolve_plot_option(&word).is_some() {
                tokens.pos = save_pos;
                break;
            }
        } else {
            break;
        }
    }

    tokens.input[start..tokens.pos].trim().to_string()
}

fn try_resolve_plot_option(word: &str) -> Option<&'static str> {
    // Direct abbreviation matches for plot options
    let options = &[
        "with", "title", "linewidth", "linecolor",
        "pointtype", "pointsize", "using", "index", "every",
    ];
    // Also handle common two-letter abbreviations
    match word {
        "w" => Some("with"),
        "t" => Some("title"),
        "lw" => Some("linewidth"),
        "lc" => Some("linecolor"),
        "pt" => Some("pointtype"),
        "ps" => Some("pointsize"),
        "u" => Some("using"),
        _ => resolve(word, options).ok(),
    }
}

fn parse_style_kind(tokens: &mut Tokenizer) -> Result<StyleKind, String> {
    let word = tokens.next_word().ok_or("Expected style name after 'with'")?;
    let styles = &[
        "lines", "points", "linespoints", "dots",
        "impulses", "boxes", "errorbars", "filledcurves",
    ];
    // Handle common abbreviations
    let style_name = match word.as_str() {
        "l" => "lines",
        "p" => "points",
        "lp" => "linespoints",
        "d" => "dots",
        _ => resolve(&word, styles)?,
    };

    match style_name {
        "lines"       => Ok(StyleKind::Lines),
        "points"      => Ok(StyleKind::Points),
        "linespoints" => Ok(StyleKind::LinesPoints),
        "dots"        => Ok(StyleKind::Dots),
        "impulses"    => Ok(StyleKind::Impulses),
        "boxes"       => Ok(StyleKind::Boxes),
        "errorbars"   => Ok(StyleKind::ErrorBars),
        "filledcurves" => Ok(StyleKind::FilledCurves),
        _             => Err(format!("Unknown style: {word}")),
    }
}

fn parse_color(tokens: &mut Tokenizer) -> Result<Color, String> {
    let word = tokens.next_word().ok_or("Expected color spec")?;
    if word == "rgb" {
        let hex = tokens.next_quoted_string()?;
        parse_hex_color(&hex)
    } else {
        // Named color
        match word.as_str() {
            "red"    => Ok(Color { r: 255, g: 0,   b: 0   }),
            "green"  => Ok(Color { r: 0,   g: 128, b: 0   }),
            "blue"   => Ok(Color { r: 0,   g: 0,   b: 255 }),
            "black"  => Ok(Color { r: 0,   g: 0,   b: 0   }),
            "white"  => Ok(Color { r: 255, g: 255, b: 255 }),
            _        => Err(format!("Unknown color: {word}")),
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: #{hex}"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
    Ok(Color { r, g, b })
}

fn parse_using_spec(tokens: &mut Tokenizer) -> Result<UsingSpec, String> {
    // Parse "1:2" or "1:2:3" or "1:($2*1000)"
    let mut columns = Vec::new();
    loop {
        tokens.skip_whitespace();
        if tokens.peek_char() == Some('(') {
            tokens.expect_char('(')?;
            // collect until ')'
            let start = tokens.pos;
            let mut depth = 1;
            while tokens.pos < tokens.input.len() && depth > 0 {
                match tokens.input.as_bytes()[tokens.pos] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    tokens.pos += 1;
                }
            }
            let expr_str = &tokens.input[start..tokens.pos];
            tokens.expect_char(')')?;
            let expr = parse_expr(expr_str)?;
            columns.push(UsingColumn::Expr(expr));
        } else {
            let n = tokens.next_number()? as usize;
            columns.push(UsingColumn::Index(n));
        }

        tokens.skip_whitespace();
        if tokens.remaining().starts_with(':') {
            tokens.pos += 1;
        } else {
            break;
        }
    }
    Ok(UsingSpec { columns })
}

// --- set parser ---

fn parse_set(tokens: &mut Tokenizer) -> Result<SetCommand, String> {
    let prop_word = tokens.next_word().ok_or("set: expected property name")?;
    let props = &[
        "xrange", "yrange", "title", "xlabel", "ylabel",
        "terminal", "output", "key", "xtics", "ytics",
        "border", "font", "samples",
    ];
    let prop_name = resolve(&prop_word, props)?;

    match prop_name {
        "xrange" => parse_range(tokens).map(SetCommand::XRange),
        "yrange" => parse_range(tokens).map(SetCommand::YRange),
        "title"  => tokens.next_quoted_string().map(SetCommand::Title),
        "xlabel" => tokens.next_quoted_string().map(SetCommand::XLabel),
        "ylabel" => tokens.next_quoted_string().map(SetCommand::YLabel),
        "terminal" | "term" => {
            let term_word = tokens.next_word().ok_or("Expected terminal type")?;
            let terms = &["svg", "pdf", "png", "eps", "window"];
            let term = resolve(&term_word, terms)?;
            match term {
                "svg"    => Ok(SetCommand::Terminal(TerminalType::Svg)),
                "pdf"    => Ok(SetCommand::Terminal(TerminalType::Pdf)),
                "png"    => Ok(SetCommand::Terminal(TerminalType::Png)),
                "eps"    => Ok(SetCommand::Terminal(TerminalType::Eps)),
                "window" => Ok(SetCommand::Terminal(TerminalType::Window)),
                _        => Err(format!("Unknown terminal: {term}")),
            }
        }
        "output"  => tokens.next_quoted_string().map(SetCommand::Output),
        "key"     => parse_key_options(tokens),
        "xtics"   => parse_tics_spec(tokens).map(SetCommand::XTics),
        "ytics"   => parse_tics_spec(tokens).map(SetCommand::YTics),
        "border"  => tokens.next_number().map(|n| SetCommand::Border(n as u32)),
        "font"    => tokens.next_quoted_string().map(SetCommand::Font),
        "samples" => tokens.next_number().map(|n| SetCommand::Samples(n as usize)),
        _         => Err(format!("Unknown set property: {prop_name}")),
    }
}

fn parse_range(tokens: &mut Tokenizer) -> Result<Range, String> {
    tokens.expect_char('[')?;
    let min = parse_bound(tokens)?;
    tokens.expect_char(':')?;
    let max = parse_bound(tokens)?;
    tokens.expect_char(']')?;
    Ok(Range { min, max })
}

fn parse_bound(tokens: &mut Tokenizer) -> Result<Bound, String> {
    tokens.skip_whitespace();
    if tokens.remaining().starts_with('*') {
        tokens.pos += 1;
        Ok(Bound::Auto)
    } else {
        tokens.next_number().map(Bound::Value)
    }
}

fn parse_key_options(tokens: &mut Tokenizer) -> Result<SetCommand, String> {
    tokens.skip_whitespace();
    let remaining = tokens.remaining().trim();

    if remaining == "off" || remaining == "nokey" {
        return Ok(SetCommand::Key(KeyOptions { visible: false, position: KeyPosition::TopRight }));
    }

    let mut position = KeyPosition::TopRight;
    let mut has_top = false;
    let mut has_bottom = false;
    let mut has_left = false;

    // Parse up to two position words
    for _ in 0..2 {
        if let Some(word) = tokens.next_word() {
            match word.as_str() {
                "top"    => has_top = true,
                "bottom" => has_bottom = true,
                "left"   => has_left = true,
                "right"  => {}
                _ => break,
            }
        }
    }

    if has_top && has_left { position = KeyPosition::TopLeft; }
    else if has_bottom && has_left { position = KeyPosition::BottomLeft; }
    else if has_bottom { position = KeyPosition::BottomRight; }
    // else default TopRight

    Ok(SetCommand::Key(KeyOptions { visible: true, position }))
}

fn parse_tics_spec(tokens: &mut Tokenizer) -> Result<TicsSpec, String> {
    tokens.skip_whitespace();
    if tokens.remaining().is_empty() {
        return Ok(TicsSpec::Auto);
    }

    let start = tokens.next_number()?;
    tokens.skip_whitespace();
    if tokens.remaining().starts_with(',') {
        tokens.pos += 1;
        let step = tokens.next_number()?;
        tokens.skip_whitespace();
        if tokens.remaining().starts_with(',') {
            tokens.pos += 1;
            let end = tokens.next_number()?;
            // gnuplot format: start, step, end
            Ok(TicsSpec::Increment { start, step, end: Some(end) })
        } else {
            // Just start, step
            Ok(TicsSpec::Increment { start, step, end: None })
        }
    } else {
        // Single number = step from 0
        Ok(TicsSpec::Increment { start: 0.0, step: start, end: None })
    }
}

// --- unset parser ---

fn parse_unset(tokens: &mut Tokenizer) -> Result<SetProperty, String> {
    let prop_word = tokens.next_word().ok_or("unset: expected property name")?;
    let props = &[
        "xrange", "yrange", "title", "xlabel", "ylabel",
        "terminal", "output", "key", "xtics", "ytics",
        "border", "font", "samples",
    ];
    let prop_name = resolve(&prop_word, props)?;

    match prop_name {
        "xrange"   => Ok(SetProperty::XRange),
        "yrange"   => Ok(SetProperty::YRange),
        "title"    => Ok(SetProperty::Title),
        "xlabel"   => Ok(SetProperty::XLabel),
        "ylabel"   => Ok(SetProperty::YLabel),
        "terminal" => Ok(SetProperty::Terminal),
        "output"   => Ok(SetProperty::Output),
        "key"      => Ok(SetProperty::Key),
        "xtics"    => Ok(SetProperty::XTics),
        "ytics"    => Ok(SetProperty::YTics),
        "border"   => Ok(SetProperty::Border),
        "font"     => Ok(SetProperty::Font),
        "samples"  => Ok(SetProperty::Samples),
        _          => Err(format!("Unknown property: {prop_name}")),
    }
}
```

Note: `parse_command` returns `Result<Option<Command>>` — `None` for comments and empty lines. The tests call a wrapper: update tests to use `parse_command(input)?.unwrap()` pattern, or add a convenience wrapper. For simplicity, adjust the test helper:

The public `parse_command` function in the tests should unwrap the `Option`. Update the test function signatures accordingly — tests that expect `Some(cmd)` should use:
```rust
let cmd = parse_command("plot sin(x)").unwrap().unwrap();
```

And for comments/empty lines:
```rust
assert!(parse_command("# comment").unwrap().is_none());
assert!(parse_command("").unwrap().is_none());
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib parser::tests`
Expected: All 22 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/parser/mod.rs
git commit -m "feat: implement recursive descent command parser with abbreviation support"
```

---

### Task 7: Session State

**Files:**
- Create: `src/engine/session.rs`

- [ ] **Step 1: Write tests**

```rust
// src/engine/session.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_default_session() {
        let s = SessionState::new();
        assert_eq!(s.xrange.min, Bound::Auto);
        assert_eq!(s.xrange.max, Bound::Auto);
        assert_eq!(s.samples, 1000);
        assert_eq!(s.border, 3);
        assert!(s.title.is_none());
    }

    #[test]
    fn test_apply_set_xrange() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::XRange(Range {
            min: Bound::Value(-5.0),
            max: Bound::Value(5.0),
        }));
        assert_eq!(s.xrange.min, Bound::Value(-5.0));
        assert_eq!(s.xrange.max, Bound::Value(5.0));
    }

    #[test]
    fn test_apply_set_title() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Title("Hello".into()));
        assert_eq!(s.title.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_apply_unset() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Title("Hello".into()));
        s.apply_unset(SetProperty::Title);
        assert!(s.title.is_none());
    }

    #[test]
    fn test_apply_set_samples() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Samples(500));
        assert_eq!(s.samples, 500);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::session`
Expected: FAIL

- [ ] **Step 3: Implement SessionState**

```rust
// src/engine/session.rs
use crate::parser::ast::*;

/// Cumulative state from set commands (gnuplot session state).
pub struct SessionState {
    pub xrange: Range,
    pub yrange: Range,
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    pub terminal: TerminalType,
    pub output: Option<String>,
    pub key: KeyOptions,
    pub xtics: TicsSpec,
    pub ytics: TicsSpec,
    pub border: u32,
    pub font: String,
    pub samples: usize,
    pub last_plot: Option<PlotCommand>,
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            xrange: Range { min: Bound::Auto, max: Bound::Auto },
            yrange: Range { min: Bound::Auto, max: Bound::Auto },
            title: None,
            xlabel: None,
            ylabel: None,
            terminal: TerminalType::Svg, // default for now; will be context-dependent later
            output: None,
            key: KeyOptions::default(),
            xtics: TicsSpec::Auto,
            ytics: TicsSpec::Auto,
            border: 3, // left + bottom (gnuplot default)
            font: "CMU Serif".into(),
            samples: 1000,
            last_plot: None,
        }
    }

    pub fn apply_set(&mut self, cmd: SetCommand) {
        match cmd {
            SetCommand::XRange(r)     => self.xrange = r,
            SetCommand::YRange(r)     => self.yrange = r,
            SetCommand::Title(t)      => self.title = Some(t),
            SetCommand::XLabel(l)     => self.xlabel = Some(l),
            SetCommand::YLabel(l)     => self.ylabel = Some(l),
            SetCommand::Terminal(t)   => self.terminal = t,
            SetCommand::Output(o)     => self.output = Some(o),
            SetCommand::Key(k)        => self.key = k,
            SetCommand::XTics(t)      => self.xtics = t,
            SetCommand::YTics(t)      => self.ytics = t,
            SetCommand::Border(b)     => self.border = b,
            SetCommand::Font(f)       => self.font = f,
            SetCommand::Samples(n)    => self.samples = n,
        }
    }

    pub fn apply_unset(&mut self, prop: SetProperty) {
        match prop {
            SetProperty::XRange   => self.xrange = Range { min: Bound::Auto, max: Bound::Auto },
            SetProperty::YRange   => self.yrange = Range { min: Bound::Auto, max: Bound::Auto },
            SetProperty::Title    => self.title = None,
            SetProperty::XLabel   => self.xlabel = None,
            SetProperty::YLabel   => self.ylabel = None,
            SetProperty::Terminal => self.terminal = TerminalType::Svg,
            SetProperty::Output   => self.output = None,
            SetProperty::Key      => self.key = KeyOptions { visible: false, ..KeyOptions::default() },
            SetProperty::XTics    => self.xtics = TicsSpec::Auto,
            SetProperty::YTics    => self.ytics = TicsSpec::Auto,
            SetProperty::Border   => self.border = 3,
            SetProperty::Font     => self.font = "CMU Serif".into(),
            SetProperty::Samples  => self.samples = 1000,
        }
    }
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib engine::session`
Expected: All 5 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/session.rs
git commit -m "feat: implement session state for cumulative set/unset commands"
```

---

### Task 8: Autoscale & Tick Computation

**Files:**
- Create: `src/engine/autoscale.rs`

- [ ] **Step 1: Write tests**

```rust
// src/engine/autoscale.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_number_round() {
        assert_eq!(nice_number(12.0, true), 10.0);
        assert_eq!(nice_number(35.0, true), 50.0);
        assert_eq!(nice_number(75.0, true), 100.0);
    }

    #[test]
    fn test_nice_number_ceil() {
        assert_eq!(nice_number(12.0, false), 20.0);
        assert_eq!(nice_number(35.0, false), 50.0);
    }

    #[test]
    fn test_compute_ticks_basic() {
        let ticks = compute_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.first().unwrap() >= &0.0);
        assert!(ticks.last().unwrap() <= &10.0);
        // All ticks should be "nice" numbers
        for t in &ticks {
            assert_eq!(*t, (*t * 1e10).round() / 1e10); // no floating point garbage
        }
    }

    #[test]
    fn test_compute_ticks_negative_range() {
        let ticks = compute_ticks(-5.0, 5.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.contains(&0.0));
    }

    #[test]
    fn test_autoscale_range() {
        let (min, max) = autoscale_range(-0.98, 1.02);
        // Should expand to nice bounds
        assert!(min <= -0.98);
        assert!(max >= 1.02);
    }

    #[test]
    fn test_autoscale_zero_range() {
        // When min == max, should expand to something sensible
        let (min, max) = autoscale_range(5.0, 5.0);
        assert!(min < 5.0);
        assert!(max > 5.0);
    }

    #[test]
    fn test_compute_ticks_count() {
        let ticks = compute_ticks(0.0, 100.0, 5);
        // Should produce roughly 5 ticks
        assert!(ticks.len() >= 3 && ticks.len() <= 10);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine::autoscale`
Expected: FAIL

- [ ] **Step 3: Implement autoscale**

```rust
// src/engine/autoscale.rs

/// Compute a "nice" number that is approximately equal to the input.
/// If `round` is true, round to nearest; otherwise, ceil.
/// Based on Paul Heckbert's "Nice Numbers for Graph Labels" algorithm
/// (used by gnuplot internally).
pub fn nice_number(x: f64, round: bool) -> f64 {
    let exp = x.abs().log10().floor();
    let frac = x / 10.0_f64.powf(exp);

    let nice_frac = if round {
        if frac < 1.5 {
            1.0
        } else if frac < 3.0 {
            2.0
        } else if frac < 7.0 {
            5.0
        } else {
            10.0
        }
    } else {
        if frac <= 1.0 {
            1.0
        } else if frac <= 2.0 {
            2.0
        } else if frac <= 5.0 {
            5.0
        } else {
            10.0
        }
    };

    nice_frac * 10.0_f64.powf(exp)
}

/// Compute tick positions for the given range.
/// `desired_ticks` is an approximate count of how many ticks to target.
pub fn compute_ticks(min: f64, max: f64, desired_ticks: usize) -> Vec<f64> {
    if (max - min).abs() < f64::EPSILON {
        return vec![min];
    }

    let range = nice_number(max - min, false);
    let step = nice_number(range / desired_ticks as f64, true);
    let graph_min = (min / step).floor() * step;

    let mut ticks = Vec::new();
    let mut tick = graph_min;
    while tick <= max + step * 0.5 {
        if tick >= min - step * 0.01 && tick <= max + step * 0.01 {
            // Round to avoid floating point noise
            let rounded = (tick / step).round() * step;
            ticks.push(rounded);
        }
        tick += step;
    }
    ticks
}

/// Compute nice autoscale bounds for a data range.
/// Returns (nice_min, nice_max) that fully contain [data_min, data_max].
pub fn autoscale_range(data_min: f64, data_max: f64) -> (f64, f64) {
    if (data_max - data_min).abs() < f64::EPSILON {
        // Degenerate range: expand by 10% or ±1
        let expand = if data_min.abs() > f64::EPSILON {
            data_min.abs() * 0.1
        } else {
            1.0
        };
        return (data_min - expand, data_max + expand);
    }

    let range = data_max - data_min;
    let step = nice_number(range / 5.0, true);
    let nice_min = (data_min / step).floor() * step;
    let nice_max = (data_max / step).ceil() * step;
    (nice_min, nice_max)
}
```

- [ ] **Step 4: Run tests**

Run: `cargo test --lib engine::autoscale`
Expected: All 7 tests PASS

- [ ] **Step 5: Commit**

```bash
git add src/engine/autoscale.rs
git commit -m "feat: implement autoscale and tick computation (Heckbert algorithm)"
```

---

### Task 9: Plot Model & Engine

**Files:**
- Create: `src/engine/model.rs`
- Create: `src/engine/mod.rs` (replace stub)

- [ ] **Step 1: Write tests**

```rust
// src/engine/model.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_model_has_axes() {
        let model = PlotModel {
            width: 800.0,
            height: 600.0,
            title: None,
            x_axis: Axis {
                label: None,
                range: (0.0, 10.0),
                ticks: vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0],
            },
            y_axis: Axis {
                label: None,
                range: (-1.0, 1.0),
                ticks: vec![-1.0, -0.5, 0.0, 0.5, 1.0],
            },
            series: vec![],
            key: KeyConfig { visible: true, position: KeyPos::TopRight },
            border: 3,
        };
        assert_eq!(model.x_axis.range, (0.0, 10.0));
    }
}

// src/engine/mod.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;
    use crate::engine::session::SessionState;

    #[test]
    fn test_build_model_sin_x() {
        let mut session = SessionState::new();
        session.xrange = Range {
            min: Bound::Value(-std::f64::consts::PI * 2.0),
            max: Bound::Value(std::f64::consts::PI * 2.0),
        };
        let plot = PlotCommand {
            series: vec![PlotSeries::Expression {
                expr: crate::parser::expr_parser::parse_expr("sin(x)").unwrap(),
                style: PlotStyle::default(),
            }],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        assert_eq!(model.series.len(), 1);
        assert!(!model.series[0].points.is_empty());
        // y values of sin should be in [-1, 1]
        for pt in &model.series[0].points {
            assert!(pt.1 >= -1.0 - 1e-10 && pt.1 <= 1.0 + 1e-10);
        }
    }

    #[test]
    fn test_build_model_autoscale_y() {
        let mut session = SessionState::new();
        session.xrange = Range {
            min: Bound::Value(0.0),
            max: Bound::Value(10.0),
        };
        let plot = PlotCommand {
            series: vec![PlotSeries::Expression {
                expr: crate::parser::expr_parser::parse_expr("x**2").unwrap(),
                style: PlotStyle::default(),
            }],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        // y range should cover 0 to ~100
        assert!(model.y_axis.range.1 >= 100.0);
    }

    #[test]
    fn test_build_model_default_xrange() {
        let session = SessionState::new();
        let plot = PlotCommand {
            series: vec![PlotSeries::Expression {
                expr: crate::parser::expr_parser::parse_expr("sin(x)").unwrap(),
                style: PlotStyle::default(),
            }],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        // Default xrange for expression plot should be [-10:10] (gnuplot default)
        assert_eq!(model.x_axis.range, (-10.0, 10.0));
    }

    #[test]
    fn test_build_model_multiple_series() {
        let session = SessionState::new();
        let plot = PlotCommand {
            series: vec![
                PlotSeries::Expression {
                    expr: crate::parser::expr_parser::parse_expr("sin(x)").unwrap(),
                    style: PlotStyle::default(),
                },
                PlotSeries::Expression {
                    expr: crate::parser::expr_parser::parse_expr("cos(x)").unwrap(),
                    style: PlotStyle { kind: StyleKind::Points, ..PlotStyle::default() },
                },
            ],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        assert_eq!(model.series.len(), 2);
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib engine`
Expected: FAIL

- [ ] **Step 3: Implement PlotModel types**

```rust
// src/engine/model.rs

/// The fully resolved, renderable plot model.
pub struct PlotModel {
    pub width: f64,   // in points (default 800)
    pub height: f64,  // in points (default 600)
    pub title: Option<String>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub series: Vec<SeriesData>,
    pub key: KeyConfig,
    pub border: u32,
}

pub struct Axis {
    pub label: Option<String>,
    pub range: (f64, f64), // (min, max) in data coordinates
    pub ticks: Vec<f64>,
}

pub struct SeriesData {
    pub points: Vec<(f64, f64)>,  // (x, y) in data coordinates
    pub style: SeriesStyle,
    pub label: Option<String>,    // legend label
}

pub struct SeriesStyle {
    pub kind: SeriesStyleKind,
    pub color: (u8, u8, u8),     // RGB
    pub line_width: f64,
    pub point_size: f64,
}

pub enum SeriesStyleKind {
    Lines,
    Points,
    LinesPoints,
    Dots,
    Impulses,
    Boxes,
    ErrorBars,
    FilledCurves,
}

pub struct KeyConfig {
    pub visible: bool,
    pub position: KeyPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPos {
    TopLeft,
    TopRight,
    BottomLeft,
    BottomRight,
}
```

- [ ] **Step 4: Implement build_plot_model**

```rust
// src/engine/mod.rs
pub mod model;
pub mod evaluator;
pub mod autoscale;
pub mod session;

use crate::parser::ast::*;
use model::*;
use session::SessionState;

/// gnuplot podo palette (default color cycle)
const PODO_COLORS: &[(u8, u8, u8)] = &[
    (0x00, 0x72, 0xB2), // blue
    (0xE6, 0x9F, 0x00), // orange
    (0x00, 0x9E, 0x73), // green
    (0xCC, 0x79, 0xA7), // pink
    (0x56, 0xB4, 0xE9), // light blue
    (0xD5, 0x5E, 0x00), // red-orange
    (0xF0, 0xE4, 0x42), // yellow
    (0x00, 0x00, 0x00), // black
];

/// Build a renderable PlotModel from a PlotCommand and session state.
pub fn build_plot_model(plot: &PlotCommand, session: &SessionState) -> Result<PlotModel, String> {
    // Determine x range
    let (x_min, x_max) = match (&session.xrange.min, &session.xrange.max) {
        (Bound::Value(lo), Bound::Value(hi)) => (*lo, *hi),
        _ => (-10.0, 10.0), // gnuplot default for expression plots
    };

    // Evaluate all series
    let mut all_series = Vec::new();
    let mut y_data_min = f64::INFINITY;
    let mut y_data_max = f64::NEG_INFINITY;

    for (idx, s) in plot.series.iter().enumerate() {
        match s {
            PlotSeries::Expression { expr, style } => {
                let mut points = Vec::with_capacity(session.samples);
                for i in 0..session.samples {
                    let x = x_min + (x_max - x_min) * i as f64 / (session.samples - 1) as f64;
                    match evaluator::evaluate(expr, x) {
                        Ok(y) if y.is_finite() => {
                            if y < y_data_min { y_data_min = y; }
                            if y > y_data_max { y_data_max = y; }
                            points.push((x, y));
                        }
                        _ => {} // skip NaN/Inf/errors
                    }
                }

                let color = style.line_color.as_ref()
                    .map(|c| (c.r, c.g, c.b))
                    .unwrap_or(PODO_COLORS[idx % PODO_COLORS.len()]);

                all_series.push(SeriesData {
                    points,
                    style: SeriesStyle {
                        kind: convert_style_kind(style.kind),
                        color,
                        line_width: style.line_width.unwrap_or(1.5),
                        point_size: style.point_size.unwrap_or(3.0),
                    },
                    label: style.title.clone(),
                });
            }
            PlotSeries::DataFile { .. } => {
                return Err("Data file plots not yet implemented in this plan".into());
            }
        }
    }

    // Autoscale y range
    let (y_min, y_max) = match (&session.yrange.min, &session.yrange.max) {
        (Bound::Value(lo), Bound::Value(hi)) => (*lo, *hi),
        _ => {
            if y_data_min.is_finite() && y_data_max.is_finite() {
                autoscale::autoscale_range(y_data_min, y_data_max)
            } else {
                (-1.0, 1.0)
            }
        }
    };

    // Compute ticks
    let x_ticks = match &session.xtics {
        TicsSpec::Auto => autoscale::compute_ticks(x_min, x_max, 5),
        TicsSpec::Increment { start, step, end } => {
            let end_val = end.unwrap_or(x_max);
            let mut ticks = Vec::new();
            let mut t = *start;
            while t <= end_val + step * 0.01 {
                ticks.push(t);
                t += step;
            }
            ticks
        }
        TicsSpec::List(items) => items.iter().map(|(v, _)| *v).collect(),
    };

    let y_ticks = match &session.ytics {
        TicsSpec::Auto => autoscale::compute_ticks(y_min, y_max, 5),
        TicsSpec::Increment { start, step, end } => {
            let end_val = end.unwrap_or(y_max);
            let mut ticks = Vec::new();
            let mut t = *start;
            while t <= end_val + step * 0.01 {
                ticks.push(t);
                t += step;
            }
            ticks
        }
        TicsSpec::List(items) => items.iter().map(|(v, _)| *v).collect(),
    };

    let key_pos = match session.key.position {
        KeyPosition::TopLeft     => KeyPos::TopLeft,
        KeyPosition::TopRight    => KeyPos::TopRight,
        KeyPosition::BottomLeft  => KeyPos::BottomLeft,
        KeyPosition::BottomRight => KeyPos::BottomRight,
    };

    Ok(PlotModel {
        width: 800.0,
        height: 600.0,
        title: session.title.clone(),
        x_axis: Axis { label: session.xlabel.clone(), range: (x_min, x_max), ticks: x_ticks },
        y_axis: Axis { label: session.ylabel.clone(), range: (y_min, y_max), ticks: y_ticks },
        series: all_series,
        key: KeyConfig { visible: session.key.visible, position: key_pos },
        border: session.border,
    })
}

fn convert_style_kind(kind: StyleKind) -> SeriesStyleKind {
    match kind {
        StyleKind::Lines       => SeriesStyleKind::Lines,
        StyleKind::Points      => SeriesStyleKind::Points,
        StyleKind::LinesPoints => SeriesStyleKind::LinesPoints,
        StyleKind::Dots        => SeriesStyleKind::Dots,
        StyleKind::Impulses    => SeriesStyleKind::Impulses,
        StyleKind::Boxes       => SeriesStyleKind::Boxes,
        StyleKind::ErrorBars   => SeriesStyleKind::ErrorBars,
        StyleKind::FilledCurves => SeriesStyleKind::FilledCurves,
    }
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib engine`
Expected: All tests PASS (model + autoscale + session + engine)

- [ ] **Step 6: Commit**

```bash
git add src/engine/mod.rs src/engine/model.rs
git commit -m "feat: implement plot model builder with expression sampling and autoscale"
```

---

### Task 10: Renderer Trait & SVG Backend

**Files:**
- Create: `src/renderer/mod.rs` (replace stub)
- Create: `src/renderer/svg.rs`

- [ ] **Step 1: Write tests**

```rust
// src/renderer/svg.rs — tests at bottom
#[cfg(test)]
mod tests {
    use super::*;
    use crate::engine::model::*;

    fn make_simple_model() -> PlotModel {
        PlotModel {
            width: 800.0,
            height: 600.0,
            title: Some("Test Plot".into()),
            x_axis: Axis {
                label: Some("x".into()),
                range: (0.0, 10.0),
                ticks: vec![0.0, 2.0, 4.0, 6.0, 8.0, 10.0],
            },
            y_axis: Axis {
                label: Some("y".into()),
                range: (0.0, 100.0),
                ticks: vec![0.0, 20.0, 40.0, 60.0, 80.0, 100.0],
            },
            series: vec![SeriesData {
                points: vec![(0.0, 0.0), (5.0, 25.0), (10.0, 100.0)],
                style: SeriesStyle {
                    kind: SeriesStyleKind::Lines,
                    color: (0, 114, 178),
                    line_width: 1.5,
                    point_size: 3.0,
                },
                label: Some("x^2".into()),
            }],
            key: KeyConfig { visible: true, position: KeyPos::TopRight },
            border: 3,
        }
    }

    #[test]
    fn test_svg_output_is_valid_xml() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.starts_with("<svg"));
        assert!(svg.contains("</svg>"));
    }

    #[test]
    fn test_svg_contains_title() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.contains("Test Plot"));
    }

    #[test]
    fn test_svg_contains_polyline() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.contains("<polyline") || svg.contains("<path"));
    }

    #[test]
    fn test_svg_contains_tick_labels() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        // Should contain tick labels like "0", "20", "40"...
        assert!(svg.contains(">0<"));
    }

    #[test]
    fn test_svg_contains_axis_labels() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.contains(">x<"));
        assert!(svg.contains(">y<"));
    }

    #[test]
    fn test_svg_contains_legend() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.contains("x^2"));
    }

    #[test]
    fn test_svg_no_title_when_none() {
        let mut model = make_simple_model();
        model.title = None;
        let svg = render_svg(&model);
        assert!(!svg.contains("Test Plot"));
    }

    #[test]
    fn test_svg_points_style() {
        let mut model = make_simple_model();
        model.series[0].style.kind = SeriesStyleKind::Points;
        let svg = render_svg(&model);
        assert!(svg.contains("<circle") || svg.contains("circle"));
    }
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --lib renderer::svg`
Expected: FAIL

- [ ] **Step 3: Implement renderer trait**

```rust
// src/renderer/mod.rs
pub mod svg;
```

- [ ] **Step 4: Implement SVG renderer**

```rust
// src/renderer/svg.rs
use crate::engine::model::*;
use std::fmt::Write;

/// Layout constants (in SVG pixels, matching points conceptually)
const MARGIN_LEFT: f64 = 80.0;
const MARGIN_RIGHT: f64 = 20.0;
const MARGIN_TOP: f64 = 50.0;
const MARGIN_BOTTOM: f64 = 60.0;
const TICK_LEN: f64 = 6.0;
const FONT_SIZE: f64 = 14.0;
const TITLE_FONT_SIZE: f64 = 18.0;
const LEGEND_FONT_SIZE: f64 = 12.0;
const LEGEND_LINE_LEN: f64 = 25.0;
const LEGEND_PADDING: f64 = 8.0;
const LEGEND_ROW_HEIGHT: f64 = 18.0;

/// Render a PlotModel to an SVG string.
pub fn render_svg(model: &PlotModel) -> String {
    let mut svg = String::with_capacity(4096);

    let w = model.width;
    let h = model.height;
    let plot_x = MARGIN_LEFT;
    let plot_y = MARGIN_TOP;
    let plot_w = w - MARGIN_LEFT - MARGIN_RIGHT;
    let plot_h = h - MARGIN_TOP - MARGIN_BOTTOM;

    // SVG header
    write!(svg,
        r#"<svg xmlns="http://www.w3.org/2000/svg" width="{w}" height="{h}" viewBox="0 0 {w} {h}" font-family="serif" font-size="{FONT_SIZE}">"#
    ).unwrap();
    writeln!(svg).unwrap();

    // Background
    writeln!(svg, r#"<rect width="{w}" height="{h}" fill="white"/>"#).unwrap();

    // Clip path for plot area
    writeln!(svg, r#"<defs><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#).unwrap();

    // Title
    if let Some(title) = &model.title {
        let tx = w / 2.0;
        let ty = MARGIN_TOP / 2.0 + TITLE_FONT_SIZE / 3.0;
        writeln!(svg,
            r#"<text x="{tx}" y="{ty}" text-anchor="middle" font-size="{TITLE_FONT_SIZE}" font-weight="bold">{}</text>"#,
            escape_xml(title)
        ).unwrap();
    }

    // Coordinate transform helpers
    let x_to_svg = |x: f64| -> f64 {
        plot_x + (x - model.x_axis.range.0) / (model.x_axis.range.1 - model.x_axis.range.0) * plot_w
    };
    let y_to_svg = |y: f64| -> f64 {
        plot_y + plot_h - (y - model.y_axis.range.0) / (model.y_axis.range.1 - model.y_axis.range.0) * plot_h
    };

    // Border (axes)
    let has_bottom = model.border & 1 != 0;
    let has_left = model.border & 2 != 0;

    if has_bottom {
        writeln!(svg,
            r#"<line x1="{plot_x}" y1="{}" x2="{}" y2="{}" stroke="black" stroke-width="1"/>"#,
            plot_y + plot_h, plot_x + plot_w, plot_y + plot_h
        ).unwrap();
    }
    if has_left {
        writeln!(svg,
            r#"<line x1="{plot_x}" y1="{plot_y}" x2="{plot_x}" y2="{}" stroke="black" stroke-width="1"/>"#,
            plot_y + plot_h
        ).unwrap();
    }

    // X ticks
    for &tick in &model.x_axis.ticks {
        let sx = x_to_svg(tick);
        let sy = plot_y + plot_h;
        // Tick mark (inward)
        writeln!(svg,
            r#"<line x1="{sx}" y1="{sy}" x2="{sx}" y2="{}" stroke="black" stroke-width="0.5"/>"#,
            sy - TICK_LEN
        ).unwrap();
        // Label
        writeln!(svg,
            r#"<text x="{sx}" y="{}" text-anchor="middle" font-size="{FONT_SIZE}">{}</text>"#,
            sy + FONT_SIZE + 4.0,
            format_tick(tick)
        ).unwrap();
    }

    // Y ticks
    for &tick in &model.y_axis.ticks {
        let sx = plot_x;
        let sy = y_to_svg(tick);
        // Tick mark (inward)
        writeln!(svg,
            r#"<line x1="{sx}" y1="{sy}" x2="{}" y2="{sy}" stroke="black" stroke-width="0.5"/>"#,
            sx + TICK_LEN
        ).unwrap();
        // Label
        writeln!(svg,
            r#"<text x="{}" y="{}" text-anchor="end" font-size="{FONT_SIZE}" dominant-baseline="central">{}</text>"#,
            sx - 6.0,
            sy,
            format_tick(tick)
        ).unwrap();
    }

    // X axis label
    if let Some(label) = &model.x_axis.label {
        let lx = plot_x + plot_w / 2.0;
        let ly = h - 8.0;
        writeln!(svg,
            r#"<text x="{lx}" y="{ly}" text-anchor="middle" font-size="{FONT_SIZE}">{}</text>"#,
            escape_xml(label)
        ).unwrap();
    }

    // Y axis label (rotated)
    if let Some(label) = &model.y_axis.label {
        let lx = 16.0;
        let ly = plot_y + plot_h / 2.0;
        writeln!(svg,
            r#"<text x="{lx}" y="{ly}" text-anchor="middle" font-size="{FONT_SIZE}" transform="rotate(-90,{lx},{ly})">{}</text>"#,
            escape_xml(label)
        ).unwrap();
    }

    // Data series (clipped)
    writeln!(svg, r#"<g clip-path="url(#plot-area)">"#).unwrap();
    for series in &model.series {
        let color = format!("rgb({},{},{})", series.style.color.0, series.style.color.1, series.style.color.2);

        match series.style.kind {
            SeriesStyleKind::Lines | SeriesStyleKind::LinesPoints => {
                if series.points.len() >= 2 {
                    let mut points_str = String::new();
                    for (x, y) in &series.points {
                        let sx = x_to_svg(*x);
                        let sy = y_to_svg(*y);
                        write!(points_str, "{sx:.2},{sy:.2} ").unwrap();
                    }
                    writeln!(svg,
                        r#"<polyline points="{}" fill="none" stroke="{color}" stroke-width="{}"/>"#,
                        points_str.trim(),
                        series.style.line_width
                    ).unwrap();
                }
                // Also draw points for LinesPoints
                if matches!(series.style.kind, SeriesStyleKind::LinesPoints) {
                    draw_points(&mut svg, series, &x_to_svg, &y_to_svg, &color);
                }
            }
            SeriesStyleKind::Points | SeriesStyleKind::Dots => {
                let r = if matches!(series.style.kind, SeriesStyleKind::Dots) { 1.0 } else { series.style.point_size };
                draw_points_with_radius(&mut svg, series, &x_to_svg, &y_to_svg, &color, r);
            }
            SeriesStyleKind::Impulses => {
                let base_y = y_to_svg(0.0_f64.max(model.y_axis.range.0));
                for (x, y) in &series.points {
                    let sx = x_to_svg(*x);
                    let sy = y_to_svg(*y);
                    writeln!(svg,
                        r#"<line x1="{sx:.2}" y1="{base_y:.2}" x2="{sx:.2}" y2="{sy:.2}" stroke="{color}" stroke-width="{}" />"#,
                        series.style.line_width
                    ).unwrap();
                }
            }
            SeriesStyleKind::Boxes => {
                let base_y = y_to_svg(0.0_f64.max(model.y_axis.range.0));
                let bar_width = if series.points.len() > 1 {
                    (x_to_svg(series.points[1].0) - x_to_svg(series.points[0].0)).abs() * 0.8
                } else {
                    10.0
                };
                for (x, y) in &series.points {
                    let sx = x_to_svg(*x) - bar_width / 2.0;
                    let sy = y_to_svg(*y);
                    let box_h = (base_y - sy).abs();
                    let top = sy.min(base_y);
                    writeln!(svg,
                        r#"<rect x="{sx:.2}" y="{top:.2}" width="{bar_width:.2}" height="{box_h:.2}" fill="{color}" stroke="{color}" stroke-width="0.5"/>"#
                    ).unwrap();
                }
            }
            // ErrorBars and FilledCurves: render as lines for now (TODO: full implementation)
            SeriesStyleKind::ErrorBars | SeriesStyleKind::FilledCurves => {
                if series.points.len() >= 2 {
                    let mut points_str = String::new();
                    for (x, y) in &series.points {
                        let sx = x_to_svg(*x);
                        let sy = y_to_svg(*y);
                        write!(points_str, "{sx:.2},{sy:.2} ").unwrap();
                    }
                    writeln!(svg,
                        r#"<polyline points="{}" fill="none" stroke="{color}" stroke-width="{}"/>"#,
                        points_str.trim(),
                        series.style.line_width
                    ).unwrap();
                }
            }
        }
    }
    writeln!(svg, "</g>").unwrap();

    // Legend
    if model.key.visible {
        let labeled: Vec<_> = model.series.iter()
            .filter(|s| s.label.is_some())
            .collect();
        if !labeled.is_empty() {
            let legend_w = labeled.iter()
                .map(|s| s.label.as_deref().unwrap_or("").len() as f64 * LEGEND_FONT_SIZE * 0.6 + LEGEND_LINE_LEN + LEGEND_PADDING * 3.0)
                .fold(0.0_f64, f64::max);
            let legend_h = labeled.len() as f64 * LEGEND_ROW_HEIGHT + LEGEND_PADDING * 2.0;

            let (lx, ly) = match model.key.position {
                KeyPos::TopRight    => (plot_x + plot_w - legend_w - 10.0, plot_y + 10.0),
                KeyPos::TopLeft     => (plot_x + 10.0, plot_y + 10.0),
                KeyPos::BottomRight => (plot_x + plot_w - legend_w - 10.0, plot_y + plot_h - legend_h - 10.0),
                KeyPos::BottomLeft  => (plot_x + 10.0, plot_y + plot_h - legend_h - 10.0),
            };

            writeln!(svg,
                r#"<rect x="{lx}" y="{ly}" width="{legend_w}" height="{legend_h}" fill="white" stroke="black" stroke-width="0.5"/>"#
            ).unwrap();

            for (i, series) in labeled.iter().enumerate() {
                let color = format!("rgb({},{},{})", series.style.color.0, series.style.color.1, series.style.color.2);
                let row_y = ly + LEGEND_PADDING + (i as f64 + 0.5) * LEGEND_ROW_HEIGHT;
                let line_x1 = lx + LEGEND_PADDING;
                let line_x2 = line_x1 + LEGEND_LINE_LEN;

                writeln!(svg,
                    r#"<line x1="{line_x1}" y1="{row_y}" x2="{line_x2}" y2="{row_y}" stroke="{color}" stroke-width="2"/>"#
                ).unwrap();

                let text_x = line_x2 + LEGEND_PADDING;
                writeln!(svg,
                    r#"<text x="{text_x}" y="{row_y}" dominant-baseline="central" font-size="{LEGEND_FONT_SIZE}">{}</text>"#,
                    escape_xml(series.label.as_deref().unwrap_or(""))
                ).unwrap();
            }
        }
    }

    writeln!(svg, "</svg>").unwrap();
    svg
}

fn draw_points(svg: &mut String, series: &SeriesData, x_to_svg: &dyn Fn(f64) -> f64, y_to_svg: &dyn Fn(f64) -> f64, color: &str) {
    draw_points_with_radius(svg, series, x_to_svg, y_to_svg, color, series.style.point_size);
}

fn draw_points_with_radius(svg: &mut String, series: &SeriesData, x_to_svg: &dyn Fn(f64) -> f64, y_to_svg: &dyn Fn(f64) -> f64, color: &str, radius: f64) {
    for (x, y) in &series.points {
        let sx = x_to_svg(*x);
        let sy = y_to_svg(*y);
        writeln!(svg,
            r#"<circle cx="{sx:.2}" cy="{sy:.2}" r="{radius}" fill="{color}"/>"#
        ).unwrap();
    }
}

fn format_tick(val: f64) -> String {
    if val == 0.0 {
        "0".into()
    } else if val.abs() >= 1000.0 || val.abs() < 0.01 {
        format!("{val:.2e}")
    } else if val.fract().abs() < 1e-10 {
        format!("{}", val as i64)
    } else {
        format!("{val:.2}")
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}
```

- [ ] **Step 5: Run tests**

Run: `cargo test --lib renderer::svg`
Expected: All 8 tests PASS

- [ ] **Step 6: Commit**

```bash
git add src/renderer/mod.rs src/renderer/svg.rs
git commit -m "feat: implement SVG renderer with axes, ticks, legend, and series drawing"
```

---

### Task 11: CLI Integration

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: Write integration test**

```rust
// tests/integration/plot_basic.rs
use std::process::Command;

#[test]
fn test_plot_sin_x_produces_svg() {
    let output = Command::new("cargo")
        .args(["run", "--", "/dev/stdin"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"set terminal svg\nplot sin(x)\n").unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run kaniplot");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("<svg"), "Expected SVG output, got: {}", &stdout[..stdout.len().min(200)]);
    assert!(stdout.contains("<polyline"), "Expected polyline in SVG");
    assert!(stdout.contains("</svg>"), "Expected closing SVG tag");
}

#[test]
fn test_pipe_mode_default_to_svg() {
    // When piped, default terminal should produce output to stdout
    let output = Command::new("cargo")
        .args(["run"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(b"plot sin(x)\n").unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run kaniplot");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // In pipe mode, default terminal is SVG for now (PNG later)
    assert!(stdout.contains("<svg"), "Expected SVG output in pipe mode");
}

#[test]
fn test_script_with_set_commands() {
    let script = r#"
set title "Sine Wave"
set xlabel "x"
set ylabel "y"
set xrange [-6.28:6.28]
set terminal svg
plot sin(x)
"#;
    let output = Command::new("cargo")
        .args(["run"])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .and_then(|mut child| {
            use std::io::Write;
            child.stdin.take().unwrap().write_all(script.as_bytes()).unwrap();
            child.wait_with_output()
        })
        .expect("Failed to run kaniplot");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Sine Wave"), "Title should appear in SVG");
    assert!(stdout.contains(">x<"), "xlabel should appear");
    assert!(stdout.contains(">y<"), "ylabel should appear");
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --test integration`
Expected: FAIL (or not found — we need to set up the test harness)

Note: Integration tests need a `tests/integration/mod.rs` or use individual test files. For simplicity, use `tests/integration.rs` as a single file:

Restructure to `tests/integration.rs` (single file):

```bash
# Move to tests/integration.rs
```

- [ ] **Step 3: Implement main.rs**

```rust
// src/main.rs
use std::io::{self, Read, Write};

use kaniplot::parser;
use kaniplot::parser::ast::*;
use kaniplot::engine;
use kaniplot::engine::session::SessionState;
use kaniplot::renderer::svg;

fn main() {
    let mut input = String::new();
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        input = std::fs::read_to_string(&args[1]).expect("Cannot read file");
    } else {
        io::stdin().read_to_string(&mut input).expect("Cannot read stdin");
    }

    let commands = match parser::parse_script(&input) {
        Ok(cmds) => cmds,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let mut session = SessionState::new();

    for cmd in commands {
        match cmd {
            Command::Set(set_cmd) => {
                session.apply_set(set_cmd);
            }
            Command::Unset(prop) => {
                session.apply_unset(prop);
            }
            Command::Plot(plot_cmd) => {
                match engine::build_plot_model(&plot_cmd, &session) {
                    Ok(model) => {
                        let output_svg = svg::render_svg(&model);

                        if let Some(ref path) = session.output {
                            std::fs::write(path, &output_svg).expect("Cannot write output file");
                        } else {
                            io::stdout().write_all(output_svg.as_bytes()).unwrap();
                        }

                        session.last_plot = Some(plot_cmd);
                    }
                    Err(e) => {
                        eprintln!("Plot error: {e}");
                    }
                }
            }
            Command::Replot => {
                if let Some(ref plot_cmd) = session.last_plot.clone() {
                    match engine::build_plot_model(plot_cmd, &session) {
                        Ok(model) => {
                            let output_svg = svg::render_svg(&model);
                            if let Some(ref path) = session.output {
                                std::fs::write(path, &output_svg).expect("Cannot write output file");
                            } else {
                                io::stdout().write_all(output_svg.as_bytes()).unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("Replot error: {e}");
                        }
                    }
                } else {
                    eprintln!("No previous plot to replot");
                }
            }
            Command::Quit => {
                return;
            }
        }
    }
}
```

- [ ] **Step 4: Run integration tests**

Run: `cargo test --test integration`
Expected: All 3 integration tests PASS

- [ ] **Step 5: Manual smoke test**

```bash
echo 'set title "Hello kaniplot"
set xlabel "x"
set ylabel "sin(x)"
plot sin(x)' | cargo run > /tmp/test.svg && open /tmp/test.svg
```

Expected: Browser opens with a sine wave plot, title, and axis labels.

- [ ] **Step 6: Commit**

```bash
git add src/main.rs tests/
git commit -m "feat: integrate CLI with parser, engine, and SVG renderer (end-to-end)"
```

---

### Task 12: Final Polish & Push

- [ ] **Step 1: Run all tests**

```bash
cargo test
```

Expected: All unit + integration tests PASS

- [ ] **Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

Fix any warnings.

- [ ] **Step 3: Commit any clippy fixes**

```bash
git add -A
git commit -m "fix: address clippy warnings"
```

- [ ] **Step 4: Push to feature branch and create PR**

```bash
git checkout -b feat/core-pipeline
git push -u origin feat/core-pipeline
gh pr create --title "feat: core pipeline (parser + engine + SVG renderer)" \
  --body "$(cat <<'EOF'
## Summary
- Implement gnuplot-compatible command parser with abbreviation support
- Expression PEG grammar (pest) with full operator precedence
- Expression evaluator with trig/math builtins
- Plot engine with autoscale and tick computation
- SVG renderer with axes, ticks, legend, and multiple series styles
- CLI for script and pipe mode

## Test plan
- [ ] `cargo test` — all unit + integration tests pass
- [ ] `echo 'plot sin(x)' | cargo run > test.svg` — produces valid SVG
- [ ] `echo 'plot sin(x), cos(x) w points' | cargo run > test.svg` — multiple series
- [ ] SVG opens correctly in browser

🤖 Generated with [Claude Code](https://claude.com/claude-code)
EOF
)"
```

---

## Future Plans

After this PR is merged, the following plans should be created:

1. **Plan 2: Data File Loading** — `using`, `index`, `every`, file reading, column expressions
2. **Plan 3: LaTeX Math Renderer** — `$...$` parsing, font embedding, glyph layout
3. **Plan 4: Additional Backends** — PDF (`pdf-writer`), PNG (`tiny-skia`), EPS, Window (`minifb`)
4. **Plan 5: REPL & CLI Polish** — `rustyline`, history, tab completion, interactive features
