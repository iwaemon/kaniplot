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
            PlotSeries::DataFile { .. } => {
                return Err("Data file plots not yet implemented".into());
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
}
