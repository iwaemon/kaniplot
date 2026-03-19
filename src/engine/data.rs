use crate::parser::ast::{UsingColumn, UsingSpec};
use super::evaluator::evaluate_with_columns;

/// A block of data rows (separated by blank lines in the data file).
pub struct DataBlock {
    pub rows: Vec<Vec<f64>>,
}

/// Parse a gnuplot-style data file into a list of `DataBlock`s.
///
/// - Lines starting with `#` are treated as comments and skipped.
/// - Blank lines separate blocks.
/// - Columns are whitespace-delimited.
/// - `?` in any column is treated as `f64::NAN`.
pub fn parse_data_file(content: &str) -> Result<Vec<DataBlock>, String> {
    let mut blocks: Vec<DataBlock> = Vec::new();
    let mut current_rows: Vec<Vec<f64>> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Blank line: finish the current block (if any)
        if trimmed.is_empty() {
            if !current_rows.is_empty() {
                blocks.push(DataBlock { rows: current_rows });
                current_rows = Vec::new();
            }
            continue;
        }

        // Parse whitespace-delimited columns
        let row: Result<Vec<f64>, String> = trimmed
            .split_whitespace()
            .map(|token| {
                if token == "?" {
                    Ok(f64::NAN)
                } else {
                    token.parse::<f64>().map_err(|_| format!("Cannot parse value: {token:?}"))
                }
            })
            .collect();

        current_rows.push(row?);
    }

    // Don't forget the last block (file may not end with a blank line)
    if !current_rows.is_empty() {
        blocks.push(DataBlock { rows: current_rows });
    }

    Ok(blocks)
}

/// Evaluate a single `UsingColumn` against a data row.
fn eval_using_column(col: &UsingColumn, row: &[f64]) -> Result<f64, String> {
    match col {
        UsingColumn::Index(n) => {
            let i = *n;
            if i == 0 || i > row.len() {
                Err(format!("Column index {i} out of range (row has {} columns)", row.len()))
            } else {
                Ok(row[i - 1])
            }
        }
        UsingColumn::Expr(expr) => evaluate_with_columns(expr, row),
    }
}

/// Extract (x, y) point pairs from a `DataBlock`.
///
/// - When `using` is `None`, defaults to columns 1 and 2 (gnuplot default).
/// - Column indices follow the gnuplot 1-based convention.
/// - `every` is an optional step size; only every Nth row is included.
/// - Rows where any selected value is NaN yield `None` in the output.
pub fn extract_points(
    block: &DataBlock,
    using: Option<&UsingSpec>,
    every: Option<usize>,
) -> Result<Vec<Option<(f64, f64)>>, String> {
    let step = every.unwrap_or(1).max(1);
    let mut result = Vec::new();

    for (i, row) in block.rows.iter().enumerate() {
        if i % step != 0 {
            continue;
        }

        let (x, y) = match using {
            None => {
                // Default: columns 1 and 2
                if row.len() < 2 {
                    return Err(format!(
                        "Row {i} has only {} column(s); need at least 2 for default using 1:2",
                        row.len()
                    ));
                }
                (row[0], row[1])
            }
            Some(spec) => {
                if spec.columns.len() < 2 {
                    return Err(format!(
                        "UsingSpec must have at least 2 columns, got {}",
                        spec.columns.len()
                    ));
                }
                let x = eval_using_column(&spec.columns[0], row)?;
                let y = eval_using_column(&spec.columns[1], row)?;
                (x, y)
            }
        };

        if x.is_nan() || y.is_nan() {
            result.push(None);
        } else {
            result.push(Some((x, y)));
        }
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::{UsingColumn, UsingSpec};
    use crate::parser::expr_parser::parse_expr;

    // ── parse_data_file tests ────────────────────────────────────────────────

    #[test]
    fn test_parse_simple_two_columns() {
        let content = "1.0 2.0\n3.0 4.0\n5.0 6.0\n";
        let blocks = parse_data_file(content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rows.len(), 3);
        assert_eq!(blocks[0].rows[0], vec![1.0, 2.0]);
        assert_eq!(blocks[0].rows[1], vec![3.0, 4.0]);
        assert_eq!(blocks[0].rows[2], vec![5.0, 6.0]);
    }

    #[test]
    fn test_parse_tab_separated() {
        let content = "1.0\t2.0\n3.0\t4.0\n";
        let blocks = parse_data_file(content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rows.len(), 2);
        assert_eq!(blocks[0].rows[0], vec![1.0, 2.0]);
        assert_eq!(blocks[0].rows[1], vec![3.0, 4.0]);
    }

    #[test]
    fn test_parse_comments_skipped() {
        let content = "# This is a comment\n1.0 2.0\n# Another comment\n3.0 4.0\n";
        let blocks = parse_data_file(content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rows.len(), 2);
        assert_eq!(blocks[0].rows[0], vec![1.0, 2.0]);
        assert_eq!(blocks[0].rows[1], vec![3.0, 4.0]);
    }

    #[test]
    fn test_parse_empty_lines_create_blocks() {
        let content = "1.0 2.0\n3.0 4.0\n\n5.0 6.0\n7.0 8.0\n";
        let blocks = parse_data_file(content).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].rows.len(), 2);
        assert_eq!(blocks[1].rows.len(), 2);
        assert_eq!(blocks[0].rows[0], vec![1.0, 2.0]);
        assert_eq!(blocks[1].rows[0], vec![5.0, 6.0]);
    }

    #[test]
    fn test_parse_missing_value_question_mark() {
        let content = "1.0 ?\n? 4.0\n";
        let blocks = parse_data_file(content).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rows.len(), 2);
        assert!(blocks[0].rows[0][1].is_nan());
        assert!(blocks[0].rows[1][0].is_nan());
        assert_eq!(blocks[0].rows[1][1], 4.0);
    }

    // ── extract_points tests ─────────────────────────────────────────────────

    #[test]
    fn test_extract_points_default_using() {
        let block = DataBlock {
            rows: vec![
                vec![1.0, 10.0],
                vec![2.0, 20.0],
                vec![3.0, 30.0],
            ],
        };
        let pts = extract_points(&block, None, None).unwrap();
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Some((1.0, 10.0)));
        assert_eq!(pts[1], Some((2.0, 20.0)));
        assert_eq!(pts[2], Some((3.0, 30.0)));
    }

    #[test]
    fn test_extract_points_with_using() {
        // using 1:3 — pick columns 1 and 3
        let block = DataBlock {
            rows: vec![
                vec![1.0, 99.0, 10.0],
                vec![2.0, 99.0, 20.0],
            ],
        };
        let using = UsingSpec {
            columns: vec![UsingColumn::Index(1), UsingColumn::Index(3)],
        };
        let pts = extract_points(&block, Some(&using), None).unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], Some((1.0, 10.0)));
        assert_eq!(pts[1], Some((2.0, 20.0)));
    }

    #[test]
    fn test_extract_points_missing_skipped() {
        let block = DataBlock {
            rows: vec![
                vec![1.0, 10.0],
                vec![2.0, f64::NAN],
                vec![3.0, 30.0],
            ],
        };
        let pts = extract_points(&block, None, None).unwrap();
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Some((1.0, 10.0)));
        assert_eq!(pts[1], None);
        assert_eq!(pts[2], Some((3.0, 30.0)));
    }

    #[test]
    fn test_extract_points_every() {
        // every 2 — take rows at indices 0, 2, 4
        let block = DataBlock {
            rows: vec![
                vec![1.0, 10.0],
                vec![2.0, 20.0],
                vec![3.0, 30.0],
                vec![4.0, 40.0],
                vec![5.0, 50.0],
            ],
        };
        let pts = extract_points(&block, None, Some(2)).unwrap();
        assert_eq!(pts.len(), 3);
        assert_eq!(pts[0], Some((1.0, 10.0)));
        assert_eq!(pts[1], Some((3.0, 30.0)));
        assert_eq!(pts[2], Some((5.0, 50.0)));
    }

    #[test]
    fn test_extract_points_using_expr() {
        // using ($1 * 2):($2 + 1)
        let block = DataBlock {
            rows: vec![
                vec![3.0, 4.0],
                vec![5.0, 6.0],
            ],
        };
        let using = UsingSpec {
            columns: vec![
                UsingColumn::Expr(parse_expr("$1 * 2").unwrap()),
                UsingColumn::Expr(parse_expr("$2 + 1").unwrap()),
            ],
        };
        let pts = extract_points(&block, Some(&using), None).unwrap();
        assert_eq!(pts.len(), 2);
        assert_eq!(pts[0], Some((6.0, 5.0)));
        assert_eq!(pts[1], Some((10.0, 7.0)));
    }
}
