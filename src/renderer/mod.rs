// src/renderer/mod.rs
pub mod svg;
pub mod png;

use crate::engine::model::PlotModel;

pub enum OutputFormat {
    Svg,
    Png { dpi: u32 },
}

pub fn render_to_format(model: &PlotModel, format: &OutputFormat) -> Result<Vec<u8>, String> {
    let svg_string = svg::render_svg(model);
    match format {
        OutputFormat::Svg => Ok(svg_string.into_bytes()),
        OutputFormat::Png { dpi } => png::svg_to_png(&svg_string, *dpi),
    }
}

pub(crate) fn make_usvg_tree(svg: &str) -> Result<usvg::Tree, String> {
    let mut options = usvg::Options::default();
    options.fontdb_mut().load_system_fonts();
    usvg::Tree::from_str(svg, &options)
        .map_err(|e| format!("SVG parse error: {e}"))
}
