use crate::parser::ast::*;
use std::f64::consts::{E, PI};

/// Evaluate an expression with the given x value.
pub fn evaluate(expr: &Expr, x: f64) -> Result<f64, String> {
    eval_inner(expr, Some(x), &[])
}

/// Evaluate an expression substituting column values from a data row.
/// Column references like $1, $2 are resolved from `row` (1-based).
pub fn evaluate_with_columns(expr: &Expr, row: &[f64]) -> Result<f64, String> {
    eval_inner(expr, None, row)
}

fn eval_inner(expr: &Expr, x_val: Option<f64>, columns: &[f64]) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name) => match name.as_str() {
            "x" => x_val.ok_or_else(|| "Variable 'x' not available in data context".to_string()),
            "pi" => Ok(PI),
            "e" => Ok(E),
            _ => Err(format!("Unknown variable: {name}")),
        },
        Expr::ColumnRef(idx) => {
            let i = *idx;
            if i == 0 || i > columns.len() {
                Err(format!("Column index ${i} out of range (row has {} columns)", columns.len()))
            } else {
                Ok(columns[i - 1])
            }
        }
        Expr::UnaryOp(op, operand) => {
            let val = eval_inner(operand, x_val, columns)?;
            match op {
                UnaryOp::Neg => Ok(-val),
                UnaryOp::Not => Ok(if val == 0.0 { 1.0 } else { 0.0 }),
            }
        }
        Expr::BinaryOp(lhs, op, rhs) => {
            let l = eval_inner(lhs, x_val, columns)?;
            let r = eval_inner(rhs, x_val, columns)?;
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
            let vals: Result<Vec<f64>, _> = args.iter().map(|a| eval_inner(a, x_val, columns)).collect();
            let vals = vals?;
            call_builtin(name, &vals)
        }
        Expr::Ternary(cond, true_branch, false_branch) => {
            let c = eval_inner(cond, x_val, columns)?;
            if c != 0.0 {
                eval_inner(true_branch, x_val, columns)
            } else {
                eval_inner(false_branch, x_val, columns)
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::expr_parser::parse_expr;
    use std::f64::consts::PI;

    fn eval(input: &str, x: f64) -> f64 {
        let expr = parse_expr(input).unwrap();
        evaluate(&expr, x).unwrap()
    }

    #[test] fn test_constant() { assert_eq!(eval("42", 0.0), 42.0); }
    #[test] fn test_variable_x() { assert_eq!(eval("x", 3.0), 3.0); }
    #[test] fn test_pi_constant() { assert!((eval("pi", 0.0) - PI).abs() < 1e-10); }
    #[test] fn test_arithmetic() { assert_eq!(eval("2 + 3 * 4", 0.0), 14.0); }
    #[test] fn test_power() { assert_eq!(eval("2**10", 0.0), 1024.0); }
    #[test] fn test_sin() { assert!((eval("sin(pi)", 0.0)).abs() < 1e-10); }
    #[test] fn test_cos() { assert!((eval("cos(0)", 0.0) - 1.0).abs() < 1e-10); }
    #[test] fn test_nested() { let val = eval("sin(x)**2 + cos(x)**2", 1.5); assert!((val - 1.0).abs() < 1e-10); }
    #[test] fn test_unary_neg() { assert_eq!(eval("-5", 0.0), -5.0); }
    #[test] fn test_ternary() { assert_eq!(eval("x > 0 ? 1 : -1", 5.0), 1.0); assert_eq!(eval("x > 0 ? 1 : -1", -5.0), -1.0); }
    #[test] fn test_atan2() { let val = eval("atan2(1, 1)", 0.0); assert!((val - PI / 4.0).abs() < 1e-10); }
    #[test] fn test_abs() { assert_eq!(eval("abs(-7)", 0.0), 7.0); }
    #[test] fn test_sqrt() { assert_eq!(eval("sqrt(9)", 0.0), 3.0); }
    #[test] fn test_log_exp() { assert!((eval("log(exp(1))", 0.0) - 1.0).abs() < 1e-10); }

    // Tests for evaluate_with_columns
    #[test]
    fn test_evaluate_with_columns_simple() {
        // $1 + $2 with row [10.0, 20.0, 30.0] = 30.0
        let expr = parse_expr("$1 + $2").unwrap();
        let result = evaluate_with_columns(&expr, &[10.0, 20.0, 30.0]).unwrap();
        assert_eq!(result, 30.0);
    }

    #[test]
    fn test_evaluate_with_columns_multiply() {
        // $2 * 1000 with row [1.0, 0.5] = 500.0
        let expr = parse_expr("$2 * 1000").unwrap();
        let result = evaluate_with_columns(&expr, &[1.0, 0.5]).unwrap();
        assert_eq!(result, 500.0);
    }

    #[test]
    fn test_evaluate_with_columns_function() {
        // sin($1) with row [0.0] ≈ 0.0
        let expr = parse_expr("sin($1)").unwrap();
        let result = evaluate_with_columns(&expr, &[0.0]).unwrap();
        assert!(result.abs() < 1e-10);
    }
}
