pub fn svg_to_png(svg: &str, dpi: u32) -> Result<Vec<u8>, String> {
    if dpi == 0 || dpi > 10000 {
        return Err("DPI must be between 1 and 10000".to_string());
    }
    let tree = super::make_usvg_tree(svg)?;

    let scale = dpi as f32 / 96.0;
    let size = tree.size();
    let width = (size.width() * scale) as u32;
    let height = (size.height() * scale) as u32;

    let mut pixmap = tiny_skia::Pixmap::new(width, height)
        .ok_or("Failed to create pixmap")?;
    pixmap.fill(tiny_skia::Color::WHITE);

    let transform = tiny_skia::Transform::from_scale(scale, scale);
    resvg::render(&tree, transform, &mut pixmap.as_mut());

    pixmap.encode_png().map_err(|e| format!("PNG encode error: {e}"))
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
            font_sizes: FontSizes::default(),
        }
    }

    #[test]
    fn test_svg_to_png_produces_valid_png() {
        let svg = svg::render_svg(&simple_model());
        let png_data = svg_to_png(&svg, 150).unwrap();
        assert_eq!(&png_data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    }

    #[test]
    fn test_svg_to_png_dpi_zero_returns_error() {
        let svg = svg::render_svg(&simple_model());
        let result = svg_to_png(&svg, 0);
        assert!(result.is_err());
    }

    #[test]
    fn test_higher_dpi_produces_larger_image() {
        let svg = svg::render_svg(&simple_model());
        let png_96 = svg_to_png(&svg, 96).unwrap();
        let png_150 = svg_to_png(&svg, 150).unwrap();
        assert!(png_150.len() > png_96.len());
    }
}
