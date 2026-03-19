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
        .unwrap()
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
        let exp = build_power(exp_pair)?;
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
            let digits = &inner.as_str()[1..];
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

#[cfg(test)]
mod tests {
    use super::*;

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
