pub mod model;
pub mod evaluator;
pub mod autoscale;
pub mod session;
pub mod data;

use crate::parser::ast::*;
use model::*;
use session::SessionState;

/// gnuplot podo palette (default color cycle)
const PODO_COLORS: &[(u8, u8, u8)] = &[
    (0x00, 0x72, 0xB2),
    (0xE6, 0x9F, 0x00),
    (0x00, 0x9E, 0x73),
    (0xCC, 0x79, 0xA7),
    (0x56, 0xB4, 0xE9),
    (0xD5, 0x5E, 0x00),
    (0xF0, 0xE4, 0x42),
    (0x00, 0x00, 0x00),
];

/// Build a renderable PlotModel from a PlotCommand and session state.
pub fn build_plot_model(plot: &PlotCommand, session: &SessionState) -> Result<PlotModel, String> {
    type PreloadedPoints = Vec<Option<(f64, f64)>>;

    // First pass: pre-load data files to determine x range from data
    let mut preloaded: Vec<Option<PreloadedPoints>> = Vec::with_capacity(plot.series.len());
    let mut x_data_min = f64::INFINITY;
    let mut x_data_max = f64::NEG_INFINITY;

    for s in &plot.series {
        match s {
            PlotSeries::Expression { .. } => {
                preloaded.push(None);
            }
            PlotSeries::DataFile { path, using, index, every, .. } => {
                let content = std::fs::read_to_string(path)
                    .map_err(|e| format!("Cannot read data file {path:?}: {e}"))?;
                let blocks = data::parse_data_file(&content)?;
                let block_idx = index.unwrap_or(0);
                let block = blocks.get(block_idx)
                    .ok_or_else(|| format!("Data file {path:?} has no block at index {block_idx}"))?;
                let points = data::extract_points(block, using.as_ref(), *every)?;
                for (x, _) in points.iter().flatten() {
                    if x.is_finite() {
                        if *x < x_data_min { x_data_min = *x; }
                        if *x > x_data_max { x_data_max = *x; }
                    }
                }
                preloaded.push(Some(points));
            }
        }
    }

    // Determine final x range
    let (x_min, x_max) = match (&session.xrange.min, &session.xrange.max) {
        (Bound::Value(lo), Bound::Value(hi)) => (*lo, *hi),
        _ => {
            if x_data_min.is_finite() && x_data_max.is_finite() {
                (x_data_min, x_data_max)
            } else {
                (-10.0, 10.0) // gnuplot default for expression plots
            }
        }
    };

    // Second pass: build all series
    let mut all_series = Vec::new();
    let mut y_data_min = f64::INFINITY;
    let mut y_data_max = f64::NEG_INFINITY;

    for (idx, (s, preloaded_data)) in plot.series.iter().zip(preloaded).enumerate() {
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
                        point_size: style.point_size.unwrap_or(4.5),
                    },
                    label: style.title.clone(),
                });
            }
            PlotSeries::DataFile { style, .. } => {
                let raw_points = preloaded_data.expect("DataFile must have preloaded data");
                let mut points = Vec::with_capacity(raw_points.len());
                for (x, y) in raw_points.into_iter().flatten() {
                    if y.is_finite() {
                        if y < y_data_min { y_data_min = y; }
                        if y > y_data_max { y_data_max = y; }
                    }
                    points.push((x, y));
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
                        point_size: style.point_size.unwrap_or(4.5),
                    },
                    label: style.title.clone(),
                });
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

    #[test]
    fn test_build_model_data_file() {
        use std::io::Write;
        let dir = std::env::temp_dir().join("kaniplot_test");
        std::fs::create_dir_all(&dir).unwrap();
        let data_path = dir.join("test_data.dat");
        let mut f = std::fs::File::create(&data_path).unwrap();
        writeln!(f, "1 1\n2 4\n3 9\n4 16\n5 25").unwrap();

        let session = SessionState::new();
        let plot = PlotCommand {
            series: vec![PlotSeries::DataFile {
                path: data_path.to_str().unwrap().to_string(),
                using: None, index: None, every: None,
                style: PlotStyle::default(),
            }],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        assert_eq!(model.series.len(), 1);
        assert_eq!(model.series[0].points.len(), 5);
        assert_eq!(model.series[0].points[0], (1.0, 1.0));
        assert_eq!(model.series[0].points[4], (5.0, 25.0));
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
                using: Some(UsingSpec { columns: vec![UsingColumn::Index(1), UsingColumn::Index(3)] }),
                index: None, every: None,
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
                using: None, index: None, every: Some(2),
                style: PlotStyle::default(),
            }],
        };
        let model = build_plot_model(&plot, &session).unwrap();
        assert_eq!(model.series[0].points.len(), 3);
        assert_eq!(model.series[0].points[0], (1.0, 1.0));
        assert_eq!(model.series[0].points[1], (3.0, 3.0));
        std::fs::remove_file(&data_path).ok();
    }
}
