use crate::engine::model::*;
use crate::math;
use std::fmt::Write;

/// Layout constants (in SVG pixels, matching points conceptually)
const MARGIN_LEFT: f64 = 110.0;
const MARGIN_RIGHT: f64 = 30.0;
const MARGIN_TOP: f64 = 65.0;
const MARGIN_BOTTOM: f64 = 80.0;
const TICK_LEN: f64 = 6.0;
const FONT_SIZE: f64 = 21.0;
const TITLE_FONT_SIZE: f64 = 27.0;
const LEGEND_FONT_SIZE: f64 = 18.0;
const LEGEND_LINE_LEN: f64 = 25.0;
const LEGEND_PADDING: f64 = 10.0;
const LEGEND_ROW_HEIGHT: f64 = 24.0;

/// Returns true if the text contains any `$` delimiters (math regions).
fn model_has_math(model: &PlotModel) -> bool {
    let has_dollar = |s: &str| s.contains('$');

    if model.title.as_deref().map(has_dollar).unwrap_or(false) {
        return true;
    }
    if model.x_axis.label.as_deref().map(has_dollar).unwrap_or(false) {
        return true;
    }
    if model.y_axis.label.as_deref().map(has_dollar).unwrap_or(false) {
        return true;
    }
    for series in &model.series {
        if series.label.as_deref().map(has_dollar).unwrap_or(false) {
            return true;
        }
    }
    false
}

/// Convert a character to its Unicode Mathematical Italic equivalent.
/// Returns None if no mapping exists (the char should use font-style="italic" instead).
fn to_math_italic(c: char) -> Option<char> {
    match c {
        // Lowercase Latin a-z → Mathematical Italic (U+1D44E–U+1D467)
        // Exception: 'h' → U+210E (Planck constant / italic h)
        'a'..='g' => Some(char::from_u32(0x1D44E + (c as u32 - 'a' as u32)).unwrap()),
        'h' => Some('\u{210E}'),
        'i'..='z' => Some(char::from_u32(0x1D44E + (c as u32 - 'a' as u32)).unwrap()),
        // Uppercase Latin A-Z → Mathematical Italic (U+1D434–U+1D44D)
        'A'..='Z' => Some(char::from_u32(0x1D434 + (c as u32 - 'A' as u32)).unwrap()),
        // Greek lowercase → Mathematical Italic Greek (U+1D6FC–U+1D714)
        'α' => Some('\u{1D6FC}'), // alpha
        'β' => Some('\u{1D6FD}'), // beta
        'γ' => Some('\u{1D6FE}'), // gamma
        'δ' => Some('\u{1D6FF}'), // delta
        'ε' => Some('\u{1D700}'), // epsilon
        'ζ' => Some('\u{1D701}'), // zeta
        'η' => Some('\u{1D702}'), // eta
        'θ' => Some('\u{1D703}'), // theta
        'ι' => Some('\u{1D704}'), // iota
        'κ' => Some('\u{1D705}'), // kappa
        'λ' => Some('\u{1D706}'), // lambda (note: λ U+03BB)
        'μ' => Some('\u{1D707}'), // mu
        'ν' => Some('\u{1D708}'), // nu
        'ξ' => Some('\u{1D709}'), // xi
        'π' => Some('\u{1D70B}'), // pi
        'ρ' => Some('\u{1D70C}'), // rho
        'ς' => Some('\u{1D70D}'), // final sigma
        'σ' => Some('\u{1D70E}'), // sigma
        'τ' => Some('\u{1D70F}'), // tau
        'υ' => Some('\u{1D710}'), // upsilon
        'φ' => Some('\u{1D711}'), // phi
        'χ' => Some('\u{1D712}'), // chi
        'ψ' => Some('\u{1D713}'), // psi
        'ω' => Some('\u{1D714}'), // omega
        // Variant Greek
        'ϑ' => Some('\u{1D717}'), // vartheta
        'ϕ' => Some('\u{1D719}'), // varphi
        'ϖ' => Some('\u{1D71B}'), // varpi
        'ϱ' => Some('\u{1D71A}'), // varrho
        _ => None,
    }
}

/// Convert a string to Mathematical Italic Unicode where possible.
fn to_math_italic_str(s: &str) -> String {
    s.chars()
        .map(|c| to_math_italic(c).unwrap_or(c))
        .collect()
}

/// Render a mixed text+math string into SVG `<tspan>` elements.
///
/// The text is split on `$`: even-indexed segments are plain text (XML-escaped),
/// odd-indexed segments are LaTeX math expressions rendered via the math engine.
fn render_math_text(text: &str, font_size: f64) -> String {
    let mut out = String::new();
    // Cumulative dy offset from the natural baseline (in px).
    let mut cum_dy: f64 = 0.0;

    for (i, segment) in text.split('$').enumerate() {
        if segment.is_empty() {
            continue;
        }
        if i % 2 == 0 {
            // Plain text segment — reset dy if needed, then emit text.
            if cum_dy != 0.0 {
                // Reset to baseline
                write!(out, r#"<tspan dy="{:.3}">{}</tspan>"#, -cum_dy, escape_xml(segment)).unwrap();
                cum_dy = 0.0;
            } else {
                write!(out, "<tspan>{}</tspan>", escape_xml(segment)).unwrap();
            }
        } else {
            // Math segment
            let nodes = match math::parser::parse_math(segment) {
                Ok(n) => n,
                Err(_) => {
                    // Fallback: render as plain text
                    write!(out, "<tspan>{}</tspan>", escape_xml(segment)).unwrap();
                    continue;
                }
            };
            let layout = math::layout::layout_math(&nodes);

            // Track the current y-offset (in em) relative to the main baseline.
            // We must manage dy transitions between glyphs.
            let mut prev_y_em: f64 = 0.0; // in em units (from math layout baseline = main baseline)

            for glyph in &layout.glyphs {
                let glyph_y_em = glyph.y; // em relative to math baseline
                // dy from previous glyph position (in px)
                let dy_px = (glyph_y_em - prev_y_em) * font_size;
                cum_dy += dy_px;

                let glyph_font_size = font_size * glyph.font_size_ratio;

                let mut attrs = String::new();
                if dy_px != 0.0 {
                    write!(attrs, r#" dy="{:.3}""#, dy_px).unwrap();
                }
                if glyph.is_math_font {
                    write!(attrs, r#" font-family="Latin Modern Math""#).unwrap();
                }
                // For math font glyphs, use Unicode Mathematical Italic characters
                // instead of font-style="italic" (which requires an italic font variant
                // that Latin Modern Math doesn't have).
                let display_text = if glyph.italic && glyph.is_math_font {
                    to_math_italic_str(&glyph.text)
                } else {
                    if glyph.italic {
                        write!(attrs, r#" font-style="italic""#).unwrap();
                    }
                    glyph.text.clone()
                };
                if (glyph.font_size_ratio - 1.0).abs() > 1e-9 {
                    write!(attrs, r#" font-size="{:.3}""#, glyph_font_size).unwrap();
                }

                write!(out, "<tspan{}>{}</tspan>", attrs, escape_xml(&display_text)).unwrap();
                prev_y_em = glyph_y_em;
            }

            // After the math segment, we need to return to baseline for subsequent text.
            // We'll do this lazily: track cum_dy and reset when we next output plain text or at end.
            // If this is the last segment, also reset (so the text element baseline is consistent).
            // For now, just track cum_dy — reset will happen in the next plain text segment.
        }
    }

    // If we ended with dy offset still active, reset it with an empty tspan.
    if cum_dy != 0.0 {
        write!(out, r#"<tspan dy="{:.3}"></tspan>"#, -cum_dy).unwrap();
    }

    out
}

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

    // Defs section: clip path + optional font embedding
    if model_has_math(model) {
        let font_style = crate::fonts::svg_font_face_style();
        writeln!(svg,
            r#"<defs><style>{}</style><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#,
            font_style
        ).unwrap();
    } else {
        writeln!(svg,
            r#"<defs><clipPath id="plot-area"><rect x="{plot_x}" y="{plot_y}" width="{plot_w}" height="{plot_h}"/></clipPath></defs>"#
        ).unwrap();
    }

    // Title
    if let Some(title) = &model.title {
        let tx = w / 2.0;
        let ty = MARGIN_TOP / 2.0 + TITLE_FONT_SIZE / 3.0;
        writeln!(svg,
            r#"<text x="{tx}" y="{ty}" text-anchor="middle" font-size="{TITLE_FONT_SIZE}" font-weight="bold">{}</text>"#,
            render_math_text(title, TITLE_FONT_SIZE)
        ).unwrap();
    }

    // Coordinate transform helpers
    let x_to_svg = |x: f64| -> f64 {
        plot_x + (x - model.x_axis.range.0) / (model.x_axis.range.1 - model.x_axis.range.0) * plot_w
    };
    let y_to_svg = |y: f64| -> f64 {
        plot_y + plot_h - (y - model.y_axis.range.0) / (model.y_axis.range.1 - model.y_axis.range.0) * plot_h
    };

    // Border (axes): bit 0=bottom, 1=left, 2=top, 3=right
    let has_bottom = model.border & 1 != 0;
    let has_left = model.border & 2 != 0;
    let has_top = model.border & 4 != 0;
    let has_right = model.border & 8 != 0;

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
    if has_top {
        writeln!(svg,
            r#"<line x1="{plot_x}" y1="{plot_y}" x2="{}" y2="{plot_y}" stroke="black" stroke-width="1"/>"#,
            plot_x + plot_w
        ).unwrap();
    }
    if has_right {
        writeln!(svg,
            r#"<line x1="{}" y1="{plot_y}" x2="{}" y2="{}" stroke="black" stroke-width="1"/>"#,
            plot_x + plot_w, plot_x + plot_w, plot_y + plot_h
        ).unwrap();
    }

    // X ticks (bottom)
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

    // X ticks (top, no labels)
    if has_top {
        for &tick in &model.x_axis.ticks {
            let sx = x_to_svg(tick);
            writeln!(svg,
                r#"<line x1="{sx}" y1="{plot_y}" x2="{sx}" y2="{}" stroke="black" stroke-width="0.5"/>"#,
                plot_y + TICK_LEN
            ).unwrap();
        }
    }

    // Y ticks (left)
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

    // Y ticks (right, no labels)
    if has_right {
        for &tick in &model.y_axis.ticks {
            let sx = plot_x + plot_w;
            let sy = y_to_svg(tick);
            writeln!(svg,
                r#"<line x1="{sx}" y1="{sy}" x2="{}" y2="{sy}" stroke="black" stroke-width="0.5"/>"#,
                sx - TICK_LEN
            ).unwrap();
        }
    }

    // X axis label
    if let Some(label) = &model.x_axis.label {
        let lx = plot_x + plot_w / 2.0;
        let ly = h - 8.0;
        writeln!(svg,
            r#"<text x="{lx}" y="{ly}" text-anchor="middle" font-size="{FONT_SIZE}">{}</text>"#,
            render_math_text(label, FONT_SIZE)
        ).unwrap();
    }

    // Y axis label (rotated)
    if let Some(label) = &model.y_axis.label {
        let lx = 16.0;
        let ly = plot_y + plot_h / 2.0;
        writeln!(svg,
            r#"<text x="{lx}" y="{ly}" text-anchor="middle" font-size="{FONT_SIZE}" transform="rotate(-90,{lx},{ly})">{}</text>"#,
            render_math_text(label, FONT_SIZE)
        ).unwrap();
    }

    // Data series (clipped)
    writeln!(svg, r#"<g clip-path="url(#plot-area)">"#).unwrap();
    for series in &model.series {
        let color = format!("rgb({},{},{})", series.style.color.0, series.style.color.1, series.style.color.2);

        match series.style.kind {
            SeriesStyleKind::Lines | SeriesStyleKind::LinesPoints => {
                if series.points.len() >= 2 {
                    // Break polyline at discontinuities (large y jumps)
                    let y_range = model.y_axis.range.1 - model.y_axis.range.0;
                    let threshold = y_range * 1e4;
                    let mut segments: Vec<Vec<(f64, f64)>> = vec![vec![]];
                    let mut prev_y: Option<f64> = None;
                    for (x, y) in &series.points {
                        if let Some(py) = prev_y {
                            if (y - py).abs() > threshold {
                                segments.push(vec![]);
                            }
                        }
                        segments.last_mut().unwrap().push((*x, *y));
                        prev_y = Some(*y);
                    }
                    for seg in &segments {
                        if seg.len() >= 2 {
                            let mut points_str = String::new();
                            for (x, y) in seg {
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
                    r#"<line x1="{line_x1}" y1="{row_y}" x2="{line_x2}" y2="{row_y}" stroke="{color}" stroke-width="{}"/>"#,
                    series.style.line_width
                ).unwrap();

                let text_x = line_x2 + LEGEND_PADDING;
                writeln!(svg,
                    r#"<text x="{text_x}" y="{row_y}" dominant-baseline="central" font-size="{LEGEND_FONT_SIZE}">{}</text>"#,
                    render_math_text(series.label.as_deref().unwrap_or(""), LEGEND_FONT_SIZE)
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
        format!("{val:.0}")
    } else {
        let s = format!("{val:.6}");
        let s = s.trim_end_matches('0');
        let s = s.trim_end_matches('.');
        s.to_string()
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
     .replace('<', "&lt;")
     .replace('>', "&gt;")
     .replace('"', "&quot;")
}

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
                    point_size: 4.5,
                },
                label: Some("x^2".into()),
            }],
            key: KeyConfig { visible: true, position: KeyPos::TopRight },
            border: 15,
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
        assert!(svg.contains(">0<"));
    }

    #[test]
    fn test_svg_contains_axis_labels() {
        let model = make_simple_model();
        let svg = render_svg(&model);
        assert!(svg.contains(">x<") || svg.contains(">x</tspan>"));
        assert!(svg.contains(">y<") || svg.contains(">y</tspan>"));
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
        assert!(svg.contains("<circle"));
    }

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
        assert!(svg.contains("\u{1D714}"), "Should contain math italic omega symbol");
        assert!(svg.contains("@font-face"), "Should embed font");
    }

    #[test]
    fn test_svg_math_in_legend() {
        let mut model = make_simple_model();
        model.series[0].label = Some("$\\alpha$ curve".into());
        let svg = render_svg(&model);
        assert!(svg.contains("\u{1D6FC}"), "Should contain math italic alpha symbol");
    }
}
