# PNG/PDF バックエンド 設計ドキュメント

## 概要

kaniplot に PNG と PDF の出力形式を追加する。既存の SVG レンダラーを唯一の描画レイヤーとし、`resvg` / `svg2pdf` による変換で PNG/PDF を生成する「SVG-first」方式を採用する。

## アーキテクチャ

```
PlotModel
    │
    ▼
render_svg() → SVG 文字列
    │
    ▼
resvg::Tree::from_str() → usvg ツリー
    ├→ tiny-skia でラスタライズ → PNG (デフォルト 150 DPI)
    └→ svg2pdf で変換 → PDF
```

描画ロジックの重複がゼロで、SVG と完全に同じ見た目が保証される。

## ファイル構成

### 新規作成

| ファイル | 責務 |
|---------|------|
| `src/renderer/png.rs` | SVG → PNG 変換。`resvg` + `tiny-skia` によるラスタライズ |
| `src/renderer/pdf.rs` | SVG → PDF 変換。`svg2pdf` + `usvg` による変換 |

### 変更

| ファイル | 変更内容 |
|---------|---------|
| `src/renderer/mod.rs` | `OutputFormat` enum と `render_to_format()` 関数を追加。`pub mod png; pub mod pdf;` |
| `src/main.rs` | `Plot` と `Replot` 両方の `render_svg` 呼び出しを `render_to_format()` 経由に変更。`TerminalType` → `OutputFormat` のマッピング |
| `Cargo.toml` | `resvg`, `usvg`, `tiny-skia`, `svg2pdf` を依存に追加 |

## API 設計

### OutputFormat

```rust
// src/renderer/mod.rs
pub enum OutputFormat {
    Svg,
    Png { dpi: u32 },
    Pdf,
}

pub fn render_to_format(model: &PlotModel, format: &OutputFormat) -> Result<Vec<u8>, String> {
    let svg_string = svg::render_svg(model);
    match format {
        OutputFormat::Svg => Ok(svg_string.into_bytes()),
        OutputFormat::Png { dpi } => png::svg_to_png(&svg_string, *dpi),
        OutputFormat::Pdf => pdf::svg_to_pdf(&svg_string),
    }
}
```

### PNG 変換

```rust
// src/renderer/png.rs
pub fn svg_to_png(svg: &str, dpi: u32) -> Result<Vec<u8>, String> {
    if dpi == 0 {
        return Err("DPI must be greater than 0".to_string());
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
```

### PDF 変換

```rust
// src/renderer/pdf.rs
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let tree = super::make_usvg_tree(svg)?;

    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default())
        .map_err(|e| format!("PDF conversion error: {e}"))
}
```

## main.rs の変更

```rust
// 変更前
let output_svg = svg::render_svg(&model);
if let Some(ref path) = session.output {
    std::fs::write(path, &output_svg).expect("Cannot write output file");
} else {
    io::stdout().write_all(output_svg.as_bytes()).unwrap();
}

// 変更後
let format = match session.terminal {
    TerminalType::Svg => OutputFormat::Svg,
    TerminalType::Png => OutputFormat::Png { dpi: 150 },
    TerminalType::Pdf => OutputFormat::Pdf,
    _ => OutputFormat::Svg,
};
let output = renderer::render_to_format(&model, &format)
    .unwrap_or_else(|e| { eprintln!("Render error: {e}"); std::process::exit(1); });

if let Some(ref path) = session.output {
    std::fs::write(path, &output).expect("Cannot write output file");
} else {
    io::stdout().write_all(&output).unwrap();
}
```

## 依存クレート

| クレート | バージョン | 用途 |
|---------|----------|------|
| `resvg` | 0.44 | SVG → ラスタ変換エンジン |
| `usvg` | 0.44 | SVG パーサー（resvg の入力形式）。resvg と同バージョンで統一 |
| `tiny-skia` | 0.11 | 2D ラスタライザ（resvg 0.44 が依存する互換バージョン） |
| `svg2pdf` | 0.12 | SVG → PDF 変換（usvg 0.44 互換） |
| `fontdb` | 0.22 | フォントデータベース。`usvg::Options` の `fontdb` フィールドに渡す |

## フォント対応

`resvg` は SVG 内の `@font-face` による Base64 埋め込みフォントを読み込める。数式レンダリング（Latin Modern Math）は追加作業なしで PNG/PDF に反映される。

`resvg` がフォントを解決できない場合のフォールバックとして、`fontdb` クレートでシステムフォントをロードし `usvg::Options` に渡す。この処理は `png.rs` と `pdf.rs` の両方で共通するため、ヘルパー関数を `renderer/mod.rs` に置く：

```rust
// src/renderer/mod.rs
fn make_usvg_tree(svg: &str) -> Result<usvg::Tree, String> {
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    let options = usvg::Options::default();
    usvg::Tree::from_str(svg, &options, &fontdb)
        .map_err(|e| format!("SVG parse error: {e}"))
}
```

`png::svg_to_png` と `pdf::svg_to_pdf` はこの `make_usvg_tree()` を呼ぶ。

## 出力解像度

- PNG デフォルト: 150 DPI（`set terminal png` で変更可能にする予定だが、今回のスコープでは固定）
- PDF: ベクターなので DPI 不要。SVG のサイズ（デフォルト 800x600）をポイントに変換

## gnuplot 互換コマンド

既存のパーサーが `set terminal png` / `set terminal pdf` を認識済み。`TerminalType::Png` / `TerminalType::Pdf` が `SessionState` に保存される。今回はこれを実際に機能させる。

## テスト戦略

- **PNG 変換テスト:** `render_to_format()` で PNG バイト列が生成され、PNG ヘッダー（`\x89PNG`）が含まれるか
- **PDF 変換テスト:** `render_to_format()` で PDF バイト列が生成され、PDF ヘッダー（`%PDF`）が含まれるか
- **DPI テスト:** 150 DPI の PNG が 96 DPI より大きいピクセルサイズになるか
- **統合テスト:** `set terminal png\nset output "test.png"\nplot sin(x)` で有効な PNG が生成されるか
- **統合テスト:** `set terminal pdf\nset output "test.pdf"\nplot sin(x)` で有効な PDF が生成されるか
- **数式テスト:** 数式を含むプロットの PNG/PDF 出力が正常に生成されるか
- **Replot テスト:** `replot` コマンドでも PNG/PDF が正しく生成されるか

## スコープ外

- EPS バックエンド
- Window（GUI）バックエンド
- `set terminal png size 1024,768` によるサイズ指定（将来対応）
- DPI のユーザー指定（将来 `set terminal png dpi 300` で対応可能）
