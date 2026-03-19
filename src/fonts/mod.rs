use base64::Engine;

const FONT_DATA: &[u8] = include_bytes!("../../fonts/latinmodern-math.otf");

/// Return the font data as a Base64-encoded string.
pub fn font_base64() -> String {
    base64::engine::general_purpose::STANDARD.encode(FONT_DATA)
}

/// Return a CSS @font-face rule with the embedded font.
pub fn svg_font_face_style() -> String {
    format!(
        r#"@font-face {{ font-family: "Latin Modern Math"; src: url(data:font/opentype;base64,{}) format("opentype"); }}"#,
        font_base64()
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_font_data_not_empty() {
        assert!(!FONT_DATA.is_empty());
    }

    #[test]
    fn test_base64_encoding() {
        let b64 = font_base64();
        assert!(!b64.is_empty());
        let decoded = base64::engine::general_purpose::STANDARD.decode(&b64).unwrap();
        assert_eq!(decoded.len(), FONT_DATA.len());
    }

    #[test]
    fn test_font_face_style() {
        let style = svg_font_face_style();
        assert!(style.contains("@font-face"));
        assert!(style.contains("Latin Modern Math"));
        assert!(style.contains("data:font/opentype;base64,"));
    }
}
