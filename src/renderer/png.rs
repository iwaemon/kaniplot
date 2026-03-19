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
