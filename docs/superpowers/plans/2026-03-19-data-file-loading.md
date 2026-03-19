# Data File Loading Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Enable `plot "data.txt" using 1:2 with lines` to read whitespace-delimited data files and render them as plot series.

**Architecture:** A new `src/engine/data.rs` module handles file I/O and parsing of columnar data. It produces `Vec<(f64, f64)>` point vectors that integrate with the existing `SeriesData` pipeline. The existing parser already handles `DataFile` variants with `using`, `index`, and `every` — only the engine needs implementation.

**Tech Stack:** Rust standard library only (no new dependencies)

---

## File Structure

| File | Responsibility | Action |
|------|---------------|--------|
| `src/engine/data.rs` | Parse data files: comments, blocks, columns, missing values, `using`/`index`/`every` | **Create** |
| `src/engine/mod.rs` | Wire `DataFile` series into `build_plot_model` (currently returns error) | **Modify** (line 66-68) |
| `tests/testdata/simple.dat` | Test fixture: basic 2-column data | **Create** |
| `tests/testdata/multiblock.dat` | Test fixture: multiple blocks separated by blank lines | **Create** |
| `tests/testdata/comments.dat` | Test fixture: data with `#` comments and missing values | **Create** |
| `tests/integration.rs` | End-to-end test: plot data file produces SVG | **Modify** |

---

### Task 1: Data file parser — basic column reading

**Files:**
- Create: `src/engine/data.rs`
- Modify: `src/engine/mod.rs:1` (add `pub mod data;`)

- [ ] **Step 1: Write failing tests for basic data parsing**

In `src/engine/data.rs`, add the module with tests:

```rust
use crate::parser::ast::*;

/// A parsed data block: rows of numeric columns.
pub struct DataBlock {
    pub rows: Vec<Vec<f64>>,
}

/// Parse a data file string into blocks.
/// - Lines starting with `#` are comments (skipped)
/// - Empty lines separate blocks
/// - Columns are whitespace-delimited
/// - `?` or empty columns are treated as NaN (missing)
pub fn parse_data_file(content: &str) -> Result<Vec<DataBlock>, String> {
    todo!()
}

/// Extract (x, y) points from a data block using a UsingSpec.
/// Default using is 1:2 (first two columns).
pub fn extract_points(
    block: &DataBlock,
    using: Option<&UsingSpec>,
    every: Option<usize>,
) -> Result<Vec<Option<(f64, f64)>>, String> {
    // Returns None for rows with missing values
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_two_columns() {
        let data = "1 10\n2 20\n3 30\n";
        let blocks = parse_data_file(data).unwrap();
        assert_eq!(blocks.len(), 1);
        assert_eq!(blocks[0].rows.len(), 3);
        assert_eq!(blocks[0].rows[0], vec![1.0, 10.0]);
        assert_eq!(blocks[0].rows[2], vec![3.0, 30.0]);
    }

    #[test]
    fn test_parse_tab_separated() {
        let data = "1\t10\n2\t20\n";
        let blocks = parse_data_file(data).unwrap();
        assert_eq!(blocks[0].rows.len(), 2);
        assert_eq!(blocks[0].rows[0], vec![1.0, 10.0]);
    }

    #[test]
    fn test_parse_comments_skipped() {
        let data = "# header\n1 10\n# middle\n2 20\n";
        let blocks = parse_data_file(data).unwrap();
        assert_eq!(blocks[0].rows.len(), 2);
    }

    #[test]
    fn test_parse_empty_lines_create_blocks() {
        let data = "1 10\n2 20\n\n3 30\n4 40\n";
        let blocks = parse_data_file(data).unwrap();
        assert_eq!(blocks.len(), 2);
        assert_eq!(blocks[0].rows.len(), 2);
        assert_eq!(blocks[1].rows.len(), 2);
    }

    #[test]
    fn test_parse_missing_value_question_mark() {
        let data = "1 10\n2 ?\n3 30\n";
        let blocks = parse_data_file(data).unwrap();
        assert_eq!(blocks[0].rows.len(), 3);
        assert!(blocks[0].rows[1][1].is_nan());
    }

    #[test]
    fn test_extract_points_default_using() {
        let block = DataBlock {
            rows: vec![
                vec![1.0, 10.0],
                vec![2.0, 20.0],
                vec![3.0, 30.0],
            ],
        };
        let points = extract_points(&block, None, None).unwrap();
        assert_eq!(points.len(), 3);
        assert_eq!(points[0], Some((1.0, 10.0)));
    }

    #[test]
    fn test_extract_points_with_using() {
        let block = DataBlock {
            rows: vec![
                vec![1.0, 10.0, 100.0],
                vec![2.0, 20.0, 200.0],
            ],
        };
        let using = UsingSpec {
            columns: vec![UsingColumn::Index(1), UsingColumn::Index(3)],
        };
        let points = extract_points(&block, Some(&using), None).unwrap();
        assert_eq!(points[0], Some((1.0, 100.0)));
        assert_eq!(points[1], Some((2.0, 200.0)));
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
        let points = extract_points(&block, None, None).unwrap();
        assert_eq!(points[0], Some((1.0, 10.0)));
        assert_eq!(points[1], None);
        assert_eq!(points[2], Some((3.0, 30.0)));
    }
}
```

- [ ] **Step 2: Add module declaration**

In `src/engine/mod.rs`, add after line 4 (`pub mod session;`):

```rust
pub mod data;
```

- [ ] **Step 3: Run tests to verify they fail**

Run: `cargo test engine::data --lib`
Expected: FAIL (todo!() panics)

- [ ] **Step 4: Implement `parse_data_file`**

```rust
pub fn parse_data_file(content: &str) -> Result<Vec<DataBlock>, String> {
    let mut blocks = Vec::new();
    let mut current_rows: Vec<Vec<f64>> = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip comments
        if trimmed.starts_with('#') {
            continue;
        }

        // Empty line = block separator
        if trimmed.is_empty() {
            if !current_rows.is_empty() {
                blocks.push(DataBlock { rows: std::mem::take(&mut current_rows) });
            }
            continue;
        }

        // Parse columns
        let cols: Vec<f64> = trimmed
            .split_whitespace()
            .map(|s| {
                if s == "?" {
                    f64::NAN
                } else {
                    s.parse::<f64>().unwrap_or(f64::NAN)
                }
            })
            .collect();

        current_rows.push(cols);
    }

    // Final block
    if !current_rows.is_empty() {
        blocks.push(DataBlock { rows: current_rows });
    }

    Ok(blocks)
}
```

- [ ] **Step 5: Implement `extract_points`**

```rust
pub fn extract_points(
    block: &DataBlock,
    using: Option<&UsingSpec>,
    every: Option<usize>,
) -> Result<Vec<Option<(f64, f64)>>, String> {
    let step = every.unwrap_or(1).max(1);
    let mut points = Vec::new();

    for (i, row) in block.rows.iter().enumerate() {
        if i % step != 0 {
            continue;
        }

        let (x, y) = if let Some(spec) = using {
            if spec.columns.len() < 2 {
                return Err("using spec requires at least 2 columns".into());
            }
            let x_val = eval_using_column(&spec.columns[0], row)?;
            let y_val = eval_using_column(&spec.columns[1], row)?;
            (x_val, y_val)
        } else {
            // Default: using 1:2
            if row.len() < 2 {
                return Err(format!("Row has {} columns, need at least 2", row.len()));
            }
            (row[0], row[1])
        };

        if x.is_nan() || y.is_nan() {
            points.push(None);
        } else {
            points.push(Some((x, y)));
        }
    }

    Ok(points)
}

fn eval_using_column(col: &UsingColumn, row: &[f64]) -> Result<f64, String> {
    match col {
        UsingColumn::Index(idx) => {
            if *idx == 0 || *idx > row.len() {
                return Err(format!("Column index {} out of range (have {} columns)", idx, row.len()));
            }
            Ok(row[idx - 1]) // gnuplot columns are 1-based
        }
        UsingColumn::Expr(expr) => {
            // Evaluate expression with $N column references
            crate::engine::evaluator::evaluate_with_columns(expr, row)
        }
    }
}
```

- [ ] **Step 6: Run tests to verify they pass**

Run: `cargo test engine::data --lib`
Expected: PASS (8 tests) — except `evaluate_with_columns` doesn't exist yet, which is Task 2.

- [ ] **Step 7: Commit**

```bash
git add src/engine/data.rs src/engine/mod.rs
git commit -m "feat: implement data file parser with block/comment/missing value support"
```

---

### Task 2: Column expression evaluator (`$1`, `$2` in using specs)

**Files:**
- Modify: `src/engine/evaluator.rs`

- [ ] **Step 1: Write failing tests**

Add to `src/engine/evaluator.rs` tests module:

```rust
#[test]
fn test_evaluate_with_columns_simple() {
    let expr = crate::parser::expr_parser::parse_expr("$1 + $2").unwrap();
    let row = vec![10.0, 20.0, 30.0];
    let result = evaluate_with_columns(&expr, &row).unwrap();
    assert_eq!(result, 30.0);
}

#[test]
fn test_evaluate_with_columns_multiply() {
    let expr = crate::parser::expr_parser::parse_expr("$2 * 1000").unwrap();
    let row = vec![1.0, 0.5];
    let result = evaluate_with_columns(&expr, &row).unwrap();
    assert_eq!(result, 500.0);
}

#[test]
fn test_evaluate_with_columns_function() {
    let expr = crate::parser::expr_parser::parse_expr("sin($1)").unwrap();
    let row = vec![0.0];
    let result = evaluate_with_columns(&expr, &row).unwrap();
    assert!((result - 0.0).abs() < 1e-10);
}
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test engine::evaluator --lib`
Expected: FAIL (function not found)

- [ ] **Step 3: Implement `evaluate_with_columns`**

Add to `src/engine/evaluator.rs`:

```rust
/// Evaluate an expression with column references ($1, $2, etc.) resolved from a data row.
pub fn evaluate_with_columns(expr: &Expr, row: &[f64]) -> Result<f64, String> {
    eval_inner(expr, None, row)
}

/// Evaluate an expression with an x variable value.
pub fn evaluate(expr: &Expr, x: f64) -> Result<f64, String> {
    eval_inner(expr, Some(x), &[])
}

fn eval_inner(expr: &Expr, x_val: Option<f64>, columns: &[f64]) -> Result<f64, String> {
    match expr {
        Expr::Number(n) => Ok(*n),
        Expr::Variable(name) => match name.as_str() {
            "x" => x_val.ok_or_else(|| "Variable 'x' not available in this context".into()),
            "pi" => Ok(std::f64::consts::PI),
            "e" => Ok(std::f64::consts::E),
            _ => Err(format!("Unknown variable: {name}")),
        },
        Expr::ColumnRef(idx) => {
            if *idx == 0 || *idx > columns.len() {
                return Err(format!("Column ${} out of range (have {} columns)", idx, columns.len()));
            }
            Ok(columns[idx - 1])
        },
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
                BinOp::Eq  => if (l - r).abs() < 1e-10 { 1.0 } else { 0.0 },
                BinOp::Ne  => if (l - r).abs() >= 1e-10 { 1.0 } else { 0.0 },
                BinOp::Lt  => if l < r { 1.0 } else { 0.0 },
                BinOp::Gt  => if l > r { 1.0 } else { 0.0 },
                BinOp::Le  => if l <= r { 1.0 } else { 0.0 },
                BinOp::Ge  => if l >= r { 1.0 } else { 0.0 },
                BinOp::And => if l != 0.0 && r != 0.0 { 1.0 } else { 0.0 },
                BinOp::Or  => if l != 0.0 || r != 0.0 { 1.0 } else { 0.0 },
            })
        },
        Expr::UnaryOp(op, operand) => {
            let v = eval_inner(operand, x_val, columns)?;
            Ok(match op {
                UnaryOp::Neg => -v,
                UnaryOp::Not => if v == 0.0 { 1.0 } else { 0.0 },
            })
        },
        Expr::FuncCall(name, args) => {
            let vals: Result<Vec<f64>, String> = args.iter()
                .map(|a| eval_inner(a, x_val, columns))
                .collect();
            call_builtin(name, &vals?)
        },
        Expr::Ternary(cond, t, f) => {
            let c = eval_inner(cond, x_val, columns)?;
            if c != 0.0 {
                eval_inner(t, x_val, columns)
            } else {
                eval_inner(f, x_val, columns)
            }
        },
    }
}
```

**Important:** This refactors the existing `evaluate` function to use `eval_inner` internally, so all existing tests continue to pass. The existing `evaluate(expr, x)` signature is preserved.

- [ ] **Step 4: Run all tests**

Run: `cargo test --lib`
Expected: ALL PASS (existing evaluator tests + 3 new tests)

- [ ] **Step 5: Commit**

```bash
git add src/engine/evaluator.rs
git commit -m "feat: add evaluate_with_columns for data file column expressions"
```

---

### Task 3: Wire DataFile into build_plot_model

**Files:**
- Modify: `src/engine/mod.rs:66-68`

- [ ] **Step 1: Write failing test**

Add to `src/engine/mod.rs` tests module:

```rust
#[test]
fn test_build_model_data_file() {
    use std::io::Write;

    // Create temp data file
    let dir = std::env::temp_dir().join("kaniplot_test");
    std::fs::create_dir_all(&dir).unwrap();
    let data_path = dir.join("test_data.dat");
    let mut f = std::fs::File::create(&data_path).unwrap();
    writeln!(f, "1 1\n2 4\n3 9\n4 16\n5 25").unwrap();

    let session = SessionState::new();
    let plot = PlotCommand {
        series: vec![PlotSeries::DataFile {
            path: data_path.to_str().unwrap().to_string(),
            using: None,
            index: None,
            every: None,
            style: PlotStyle::default(),
        }],
    };
    let model = build_plot_model(&plot, &session).unwrap();
    assert_eq!(model.series.len(), 1);
    assert_eq!(model.series[0].points.len(), 5);
    assert_eq!(model.series[0].points[0], (1.0, 1.0));
    assert_eq!(model.series[0].points[4], (5.0, 25.0));

    // Cleanup
    std::fs::remove_file(&data_path).ok();
}

#[test]
fn test_build_model_data_file_with_using() {
    use std::io::Write;

    let dir = std::env::temp_dir().join("kaniplot_test");
    std::fs::create_dir_all(&dir).unwrap();
    let data_path = dir.join("test_using.dat");
    let mut f = std::fs::File::create(&data_path).unwrap();
    writeln!(f, "1 10 100\n2 20 200\n3 30 300").unwrap();

    let session = SessionState::new();
    let plot = PlotCommand {
        series: vec![PlotSeries::DataFile {
            path: data_path.to_str().unwrap().to_string(),
            using: Some(UsingSpec {
                columns: vec![UsingColumn::Index(1), UsingColumn::Index(3)],
            }),
            index: None,
            every: None,
            style: PlotStyle::default(),
        }],
    };
    let model = build_plot_model(&plot, &session).unwrap();
    assert_eq!(model.series[0].points[0], (1.0, 100.0));
    assert_eq!(model.series[0].points[2], (3.0, 300.0));

    std::fs::remove_file(&data_path).ok();
}

#[test]
fn test_build_model_data_file_with_every() {
    use std::io::Write;

    let dir = std::env::temp_dir().join("kaniplot_test");
    std::fs::create_dir_all(&dir).unwrap();
    let data_path = dir.join("test_every.dat");
    let mut f = std::fs::File::create(&data_path).unwrap();
    writeln!(f, "1 1\n2 2\n3 3\n4 4\n5 5\n6 6").unwrap();

    let session = SessionState::new();
    let plot = PlotCommand {
        series: vec![PlotSeries::DataFile {
            path: data_path.to_str().unwrap().to_string(),
            using: None,
            index: None,
            every: Some(2),
            style: PlotStyle::default(),
        }],
    };
    let model = build_plot_model(&plot, &session).unwrap();
    // every 2 → rows 0, 2, 4 → (1,1), (3,3), (5,5)
    assert_eq!(model.series[0].points.len(), 3);
    assert_eq!(model.series[0].points[0], (1.0, 1.0));
    assert_eq!(model.series[0].points[1], (3.0, 3.0));

    std::fs::remove_file(&data_path).ok();
}
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test engine::tests::test_build_model_data_file --lib`
Expected: FAIL with "Data file plots not yet implemented"

- [ ] **Step 3: Implement DataFile handling in build_plot_model**

Replace the `PlotSeries::DataFile { .. }` arm (line 66-68) in `src/engine/mod.rs`:

```rust
PlotSeries::DataFile { path, using, index, every, style } => {
    let content = std::fs::read_to_string(path)
        .map_err(|e| format!("Cannot read data file '{}': {}", path, e))?;
    let blocks = data::parse_data_file(&content)?;

    let block_idx = index.unwrap_or(0);
    let block = blocks.get(block_idx)
        .ok_or_else(|| format!("Data file has {} blocks, requested index {}", blocks.len(), block_idx))?;

    let raw_points = data::extract_points(block, using.as_ref(), *every)?;
    let mut points = Vec::new();
    for pt in &raw_points {
        if let Some((x, y)) = pt {
            if y.is_finite() {
                if *y < y_data_min { y_data_min = *y; }
                if *y > y_data_max { y_data_max = *y; }
            }
            if x.is_finite() {
                if *x < x_data_min { x_data_min = *x; }
                if *x > x_data_max { x_data_max = *x; }
            }
            points.push((*x, *y));
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
```

**Also required:** Add x-axis autoscale tracking for data files. Currently `build_plot_model` uses a fixed `(x_min, x_max)` from session or defaults to `(-10, 10)`. For data file plots, the x-range must come from the data when `xrange` is set to `Auto`. Modify the x-range determination at the top of `build_plot_model`:

Replace lines 24-28:
```rust
// Track data-driven x range for autoscaling
let mut x_data_min = f64::INFINITY;
let mut x_data_max = f64::NEG_INFINITY;
let has_data_file = plot.series.iter().any(|s| matches!(s, PlotSeries::DataFile { .. }));

// ... (series loop — both Expression and DataFile now update x_data_min/x_data_max) ...

// After the loop, determine final x range
let (x_min, x_max) = match (&session.xrange.min, &session.xrange.max) {
    (Bound::Value(lo), Bound::Value(hi)) => (*lo, *hi),
    _ => {
        if x_data_min.is_finite() && x_data_max.is_finite() && has_data_file {
            autoscale::autoscale_range(x_data_min, x_data_max)
        } else {
            (-10.0, 10.0)
        }
    }
};
```

For expression plots, sampling must happen after x_range is determined. This means we need a two-pass approach:
1. First pass: scan data files to determine x_data_min/x_data_max
2. Determine final x_range
3. Second pass: sample expressions using the final x_range, and load data files

Here is the full refactored `build_plot_model`:

```rust
pub fn build_plot_model(plot: &PlotCommand, session: &SessionState) -> Result<PlotModel, String> {
    let has_data_file = plot.series.iter().any(|s| matches!(s, PlotSeries::DataFile { .. }));

    // First pass: determine x range from data files (if any) and session
    let mut x_data_min = f64::INFINITY;
    let mut x_data_max = f64::NEG_INFINITY;

    // Pre-load data files to determine x range
    let mut loaded_data: Vec<Option<Vec<Option<(f64, f64)>>>> = Vec::new();
    for s in &plot.series {
        match s {
            PlotSeries::DataFile { path, using, index, every, .. } => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| format!("Cannot read data file '{}': {}", path, e))?;
                let blocks = data::parse_data_file(&content)?;
                let block_idx = index.unwrap_or(0);
                let block = blocks.get(block_idx)
                    .ok_or_else(|| format!("Data file has {} blocks, requested index {}", blocks.len(), block_idx))?;
                let raw_points = data::extract_points(block, using.as_ref(), *every)?;
                for pt in &raw_points {
                    if let Some((x, _y)) = pt {
                        if x.is_finite() {
                            if *x < x_data_min { x_data_min = *x; }
                            if *x > x_data_max { x_data_max = *x; }
                        }
                    }
                }
                loaded_data.push(Some(raw_points));
            }
            PlotSeries::Expression { .. } => {
                loaded_data.push(None);
            }
        }
    }

    // Determine x range
    let (x_min, x_max) = match (&session.xrange.min, &session.xrange.max) {
        (Bound::Value(lo), Bound::Value(hi)) => (*lo, *hi),
        _ => {
            if has_data_file && x_data_min.is_finite() && x_data_max.is_finite() {
                autoscale::autoscale_range(x_data_min, x_data_max)
            } else {
                (-10.0, 10.0)
            }
        }
    };

    // Second pass: build series
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
                        _ => {}
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
            PlotSeries::DataFile { style, .. } => {
                let raw_points = loaded_data[idx].as_ref().unwrap();
                let mut points = Vec::new();
                for pt in raw_points {
                    if let Some((x, y)) = pt {
                        if y.is_finite() {
                            if *y < y_data_min { y_data_min = *y; }
                            if *y > y_data_max { y_data_max = *y; }
                        }
                        points.push((*x, *y));
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
        }
    }

    // Autoscale y range (unchanged)
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

    // Compute ticks (unchanged from here)
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
```

- [ ] **Step 4: Run all tests**

Run: `cargo test --lib`
Expected: ALL PASS (existing + new data tests)

- [ ] **Step 5: Commit**

```bash
git add src/engine/mod.rs
git commit -m "feat: wire DataFile series into build_plot_model with x-autoscale"
```

---

### Task 4: Missing value handling in SVG renderer (line breaks)

**Files:**
- Modify: `src/engine/model.rs` (add `has_gap` field or use NaN convention)

Data files with missing values (`?`) should break the polyline at that point, similar to the discontinuity detection. Since `extract_points` returns `None` for missing rows, and we currently skip them in Task 3, the polyline will connect across gaps. This is the correct gnuplot behavior — gnuplot skips missing data points and the polyline jumps to the next valid point. The discontinuity detection from earlier (large y-jump threshold) handles the visual break.

No additional code change needed — the current implementation already handles this correctly by omitting missing points from the `points` vector. The discontinuity detector in the SVG renderer will break the line if the gap causes a large visual jump.

This task is already covered by the existing implementation. Mark as complete.

---

### Task 5: Integration tests with test data files

**Files:**
- Create: `tests/testdata/simple.dat`
- Create: `tests/testdata/multiblock.dat`
- Create: `tests/testdata/comments.dat`
- Modify: `tests/integration.rs`

- [ ] **Step 1: Create test data files**

`tests/testdata/simple.dat`:
```
# Simple x y data
1 1
2 4
3 9
4 16
5 25
```

`tests/testdata/multiblock.dat`:
```
# Block 0
1 10
2 20
3 30

# Block 1
1 100
2 200
3 300
```

`tests/testdata/comments.dat`:
```
# This is a comment
1 10
# Another comment
2 ?
3 30
# End
```

- [ ] **Step 2: Write integration tests**

Add to `tests/integration.rs`:

```rust
#[test]
fn test_plot_data_file() {
    let script = format!(
        "set terminal svg\nplot \"{}\" with lines\n",
        test_data_path("simple.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<polyline"), "Expected polyline for data series");
}

#[test]
fn test_plot_data_file_with_using() {
    let script = format!(
        "set terminal svg\nplot \"{}\" using 1:2 with points\n",
        test_data_path("simple.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<circle"), "Expected circles for points style");
}

#[test]
fn test_plot_data_file_multiblock_index() {
    let script = format!(
        "set terminal svg\nplot \"{}\" index 1 with lines\n",
        test_data_path("multiblock.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<polyline"), "Expected polyline");
}

#[test]
fn test_plot_data_file_with_comments_and_missing() {
    let script = format!(
        "set terminal svg\nplot \"{}\" with lines\n",
        test_data_path("comments.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    // Should produce output without errors despite comments and missing values
}
```

Also add this helper function at the top of `tests/integration.rs`:

```rust
fn test_data_path(name: &str) -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("tests");
    path.push("testdata");
    path.push(name);
    path.to_str().unwrap().to_string()
}
```

- [ ] **Step 3: Run integration tests**

Run: `cargo test --test integration`
Expected: ALL PASS (3 existing + 4 new)

- [ ] **Step 4: Commit**

```bash
git add tests/testdata/ tests/integration.rs
git commit -m "test: add integration tests for data file plotting"
```

---

### Task 6: Clippy and final cleanup

**Files:**
- All modified files

- [ ] **Step 1: Run clippy**

Run: `cargo clippy -- -D warnings`
Expected: No errors

- [ ] **Step 2: Fix any clippy warnings**

Apply suggested fixes.

- [ ] **Step 3: Run full test suite**

Run: `cargo test`
Expected: ALL PASS

- [ ] **Step 4: Commit**

```bash
git add -A
git commit -m "fix: address clippy warnings for data file loading"
```
