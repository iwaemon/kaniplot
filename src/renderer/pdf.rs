pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let mut options = svg2pdf::usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    let tree = svg2pdf::usvg::Tree::from_str(svg, &options)
        .map_err(|e| format!("SVG parse error: {e}"))?;

    svg2pdf::to_pdf(
        &tree,
        svg2pdf::ConversionOptions::default(),
        svg2pdf::PageOptions::default(),
    )
    .map_err(|e| format!("PDF conversion error: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::renderer::svg;
    use crate::engine::model::*;

    fn simple_model() -> PlotModel {
        PlotModel {
            width: 800.0,
            height: 600.0,
            title: Some("Test".into()),
            x_axis: Axis {
                label: None,
                range: (0.0, 1.0),
                ticks: vec![0.0, 0.5, 1.0],
            },
            y_axis: Axis {
                label: None,
                range: (0.0, 1.0),
                ticks: vec![0.0, 0.5, 1.0],
            },
            series: vec![],
            key: KeyConfig { visible: false, position: KeyPos::TopRight },
            border: 15,
        }
    }

    #[test]
    fn test_svg_to_pdf_produces_valid_pdf() {
        let svg = svg::render_svg(&simple_model());
        let pdf_data = svg_to_pdf(&svg).unwrap();
        assert_eq!(&pdf_data[0..5], b"%PDF-");
    }
}
