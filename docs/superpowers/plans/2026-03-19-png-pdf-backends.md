# PNG/PDF バックエンド Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** SVG 出力を resvg/svg2pdf で変換し、`set terminal png` / `set terminal pdf` で PNG/PDF 出力を可能にする。

**Architecture:** SVG-first 方式。既存の `render_svg()` で SVG 文字列を生成し、`resvg` で PNG に、`svg2pdf` で PDF に変換する。描画ロジックの重複はゼロ。

**Tech Stack:** resvg, usvg, tiny-skia, svg2pdf, fontdb

---

## File Structure

| ファイル | 責務 | 操作 |
|---------|------|------|
| `Cargo.toml` | 依存クレート追加 | 変更 |
| `src/renderer/mod.rs` | `OutputFormat` enum, `render_to_format()`, `make_usvg_tree()` | 変更 |
| `src/renderer/png.rs` | `svg_to_png()` — SVG→PNG 変換 | 新規 |
| `src/renderer/pdf.rs` | `svg_to_pdf()` — SVG→PDF 変換 | 新規 |
| `src/main.rs` | Plot/Replot の出力を `render_to_format()` 経由に変更 | 変更 |
| `tests/integration.rs` | PNG/PDF 統合テスト追加 | 変更 |

---

### Task 1: 依存クレート追加と PNG 変換モジュール

**Files:**
- Modify: `Cargo.toml`
- Create: `src/renderer/png.rs`
- Modify: `src/renderer/mod.rs`

- [ ] **Step 1: Cargo.toml に依存を追加**

`Cargo.toml` の `[dependencies]` セクションに以下を追加：

```toml
resvg = "0.44"
usvg = "0.44"
tiny-skia = "0.11"
fontdb = "0.22"
```

- [ ] **Step 2: ビルドが通ることを確認**

Run: `cargo build`
Expected: 依存ダウンロード + コンパイル成功

- [ ] **Step 3: renderer/mod.rs に OutputFormat と make_usvg_tree を追加**

`src/renderer/mod.rs` を以下に変更：

```rust
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
    let mut fontdb = fontdb::Database::new();
    fontdb.load_system_fonts();
    let options = usvg::Options::default();
    usvg::Tree::from_str(svg, &options, &fontdb)
        .map_err(|e| format!("SVG parse error: {e}"))
}
```

- [ ] **Step 4: src/renderer/png.rs を作成**

```rust
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

- [ ] **Step 5: ビルドが通ることを確認**

Run: `cargo build`
Expected: コンパイル成功

注意: `usvg::Tree::from_str` の引数が spec と異なる可能性がある。実際の API に合わせて `make_usvg_tree` を調整すること。`resvg::render` の引数も同様。crates.io のドキュメントを確認して正しい API を使うこと。

- [ ] **Step 6: コミット**

```bash
git add Cargo.toml Cargo.lock src/renderer/mod.rs src/renderer/png.rs
git commit -m "feat: add PNG renderer with resvg conversion"
```

---

### Task 2: PNG 変換のテスト

**Files:**
- Modify: `tests/integration.rs`
- Modify: `src/renderer/png.rs`

- [ ] **Step 1: 単体テストを png.rs に追加**

`src/renderer/png.rs` の末尾に追加：

```rust
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
    fn test_svg_to_png_produces_valid_png() {
        let svg = svg::render_svg(&simple_model());
        let png_data = svg_to_png(&svg, 150).unwrap();
        // PNG magic bytes
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
```

- [ ] **Step 2: テスト実行**

Run: `cargo test --lib renderer::png`
Expected: 3 tests pass

- [ ] **Step 3: ファイル出力用ヘルパーと統合テストを追加**

`tests/integration.rs` に `run_kaniplot_to_file` ヘルパーと PNG テストを追加：

```rust
/// Run kaniplot and return the process Output (for file-output tests).
fn run_kaniplot_to_file(input: &str) -> std::process::Output {
    use std::io::Write;
    let binary = kaniplot_binary();
    let mut child = std::process::Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {:?}: {}", binary, e));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
}

#[test]
fn test_png_output_via_terminal() {
    let input = "set terminal png\nset output \"/tmp/kaniplot_test_output.png\"\nplot sin(x)\n";
    let output = run_kaniplot_to_file(input);
    assert!(output.status.success(), "kaniplot failed: {}", String::from_utf8_lossy(&output.stderr));
    let data = std::fs::read("/tmp/kaniplot_test_output.png").expect("PNG file not created");
    assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47], "Not a valid PNG");
    std::fs::remove_file("/tmp/kaniplot_test_output.png").ok();
}
```

注意: この統合テストは Task 3 で `main.rs` を変更した後に通る。先に書いておき、Task 3 完了後にまとめて確認する。

- [ ] **Step 4: コミット**

```bash
git add src/renderer/png.rs tests/integration.rs
git commit -m "test: add PNG conversion tests"
```

---

### Task 3: main.rs を render_to_format() 経由に変更

**Files:**
- Modify: `src/main.rs`

- [ ] **Step 1: main.rs の import を変更**

変更前:
```rust
use kaniplot::renderer::svg;
```

変更後:
```rust
use kaniplot::renderer::{self, OutputFormat};
```

- [ ] **Step 2: render_output ヘルパー関数を追加**

`main()` の前に以下の関数を追加：

```rust
fn render_output(model: &kaniplot::engine::model::PlotModel, session: &SessionState) {
    let format = match session.terminal {
        TerminalType::Svg => OutputFormat::Svg,
        TerminalType::Png => OutputFormat::Png { dpi: 150 },
        _ => OutputFormat::Svg,
    };
    let output = match renderer::render_to_format(model, &format) {
        Ok(data) => data,
        Err(e) => {
            eprintln!("Render error: {e}");
            return;
        }
    };

    if let Some(ref path) = session.output {
        std::fs::write(path, &output).expect("Cannot write output file");
    } else {
        io::stdout().write_all(&output).unwrap();
    }
}
```

- [ ] **Step 3: Plot ブランチを変更**

変更前 (`src/main.rs:62-78`):
```rust
Command::Plot(plot_cmd) => {
    match engine::build_plot_model(&plot_cmd, &session) {
        Ok(model) => {
            let output_svg = svg::render_svg(&model);

            if let Some(ref path) = session.output {
                std::fs::write(path, &output_svg).expect("Cannot write output file");
            } else {
                io::stdout().write_all(output_svg.as_bytes()).unwrap();
            }

            session.last_plot = Some(plot_cmd);
        }
        Err(e) => {
            eprintln!("Plot error: {e}");
        }
    }
}
```

変更後:
```rust
Command::Plot(plot_cmd) => {
    match engine::build_plot_model(&plot_cmd, &session) {
        Ok(model) => {
            render_output(&model, &session);
            session.last_plot = Some(plot_cmd);
        }
        Err(e) => {
            eprintln!("Plot error: {e}");
        }
    }
}
```

- [ ] **Step 4: Replot ブランチを変更**

変更前 (`src/main.rs:80-98`):
```rust
Command::Replot => {
    if let Some(ref plot_cmd) = session.last_plot.clone() {
        match engine::build_plot_model(plot_cmd, &session) {
            Ok(model) => {
                let output_svg = svg::render_svg(&model);
                if let Some(ref path) = session.output {
                    std::fs::write(path, &output_svg).expect("Cannot write output file");
                } else {
                    io::stdout().write_all(output_svg.as_bytes()).unwrap();
                }
            }
            Err(e) => {
                eprintln!("Replot error: {e}");
            }
        }
    } else {
        eprintln!("No previous plot to replot");
    }
}
```

変更後:
```rust
Command::Replot => {
    if let Some(ref plot_cmd) = session.last_plot.clone() {
        match engine::build_plot_model(plot_cmd, &session) {
            Ok(model) => {
                render_output(&model, &session);
            }
            Err(e) => {
                eprintln!("Replot error: {e}");
            }
        }
    } else {
        eprintln!("No previous plot to replot");
    }
}
```

- [ ] **Step 5: 全テスト実行**

Run: `cargo test`
Expected: 全テスト pass（PNG 統合テスト含む）

- [ ] **Step 6: コミット**

```bash
git add src/main.rs
git commit -m "feat: route Plot/Replot through render_to_format for multi-backend support"
```

---

### Task 4: PDF 変換モジュール

**Files:**
- Modify: `Cargo.toml`
- Create: `src/renderer/pdf.rs`
- Modify: `src/renderer/mod.rs`

- [ ] **Step 1: Cargo.toml に svg2pdf を追加**

```toml
svg2pdf = "0.12"
```

- [ ] **Step 2: src/renderer/pdf.rs を作成**

```rust
pub fn svg_to_pdf(svg: &str) -> Result<Vec<u8>, String> {
    let tree = super::make_usvg_tree(svg)?;

    svg2pdf::to_pdf(&tree, svg2pdf::ConversionOptions::default(), svg2pdf::PageOptions::default())
        .map_err(|e| format!("PDF conversion error: {e}"))
}
```

注意: `svg2pdf::to_pdf` の実際の API シグネチャを確認すること。`svg2pdf 0.12` で `Result` を返すか `Vec<u8>` を直接返すか、crates.io のドキュメントで確認して合わせる。

- [ ] **Step 3: renderer/mod.rs に PDF を追加**

`pub mod png;` の下に追加：
```rust
pub mod pdf;
```

`OutputFormat` に追加：
```rust
pub enum OutputFormat {
    Svg,
    Png { dpi: u32 },
    Pdf,
}
```

`render_to_format` の match に追加：
```rust
OutputFormat::Pdf => pdf::svg_to_pdf(&svg_string),
```

- [ ] **Step 4: main.rs の format マッピングに PDF を追加**

`render_output` 関数内:
```rust
TerminalType::Pdf => OutputFormat::Pdf,
```

- [ ] **Step 5: ビルドが通ることを確認**

Run: `cargo build`
Expected: コンパイル成功

- [ ] **Step 6: コミット**

```bash
git add Cargo.toml Cargo.lock src/renderer/pdf.rs src/renderer/mod.rs src/main.rs
git commit -m "feat: add PDF renderer with svg2pdf conversion"
```

---

### Task 5: PDF テストと統合テスト

**Files:**
- Modify: `src/renderer/pdf.rs`
- Modify: `tests/integration.rs`

- [ ] **Step 1: 単体テストを pdf.rs に追加**

`src/renderer/pdf.rs` の末尾に追加：

```rust
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
        // PDF magic bytes
        assert_eq!(&pdf_data[0..5], b"%PDF-");
    }
}
```

- [ ] **Step 2: 単体テスト実行**

Run: `cargo test --lib renderer::pdf`
Expected: 1 test pass

- [ ] **Step 3: 統合テストを追加**

`tests/integration.rs` に追加：

```rust
#[test]
fn test_pdf_output_via_terminal() {
    let input = "set terminal pdf\nset output \"/tmp/kaniplot_test_output.pdf\"\nplot sin(x)\n";
    let output = run_kaniplot_to_file(input);
    assert!(output.status.success(), "kaniplot failed: {}", String::from_utf8_lossy(&output.stderr));
    let data = std::fs::read("/tmp/kaniplot_test_output.pdf").expect("PDF file not created");
    assert_eq!(&data[0..5], b"%PDF-", "Not a valid PDF");
    std::fs::remove_file("/tmp/kaniplot_test_output.pdf").ok();
}

#[test]
fn test_png_with_math_title() {
    let input = "set terminal png\nset output \"/tmp/kaniplot_math_test.png\"\nset title \"$E = mc^2$\"\nplot sin(x)\n";
    let output = run_kaniplot_to_file(input);
    assert!(output.status.success(), "kaniplot failed: {}", String::from_utf8_lossy(&output.stderr));
    let data = std::fs::read("/tmp/kaniplot_math_test.png").expect("PNG file not created");
    assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47]);
    std::fs::remove_file("/tmp/kaniplot_math_test.png").ok();
}

#[test]
fn test_replot_png_output() {
    let input = "set terminal png\nset output \"/tmp/kaniplot_replot_test.png\"\nplot sin(x)\nreplot\n";
    let output = run_kaniplot_to_file(input);
    assert!(output.status.success(), "kaniplot failed: {}", String::from_utf8_lossy(&output.stderr));
    let data = std::fs::read("/tmp/kaniplot_replot_test.png").expect("PNG file not created after replot");
    assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47], "Replot did not produce valid PNG");
    std::fs::remove_file("/tmp/kaniplot_replot_test.png").ok();
}
```

- [ ] **Step 4: 全テスト実行**

Run: `cargo test`
Expected: 全テスト pass

- [ ] **Step 5: コミット**

```bash
git add src/renderer/pdf.rs tests/integration.rs
git commit -m "test: add PDF conversion and math rendering tests"
```

---

### Task 6: README 更新と最終確認

**Files:**
- Modify: `README.md`

- [ ] **Step 1: README の出力形式セクションを更新**

`README.md` の「出力形式」セクションを変更：

変更前:
```markdown
## 出力形式

現在は SVG 形式のみ対応しています。ブラウザで直接表示でき、テキストベースなので差分管理も容易です。
```

変更後:
```markdown
## 出力形式

```gnuplot
set terminal svg     # SVG（デフォルト）
set terminal png     # PNG（150 DPI）
set terminal pdf     # PDF（ベクター）
```

| 形式 | 特徴 |
|------|------|
| SVG | デフォルト。ブラウザで直接表示、テキストベースで差分管理も容易 |
| PNG | ラスタ画像。150 DPI。論文やスライドの貼り込みに |
| PDF | ベクター画像。印刷品質。LaTeX 文書への埋め込みに |
```

- [ ] **Step 2: 手動で動作確認**

```bash
printf 'set terminal png\nset output "/tmp/test.png"\nplot sin(x)\n' | cargo run
printf 'set terminal pdf\nset output "/tmp/test.pdf"\nplot sin(x)\n' | cargo run
```

Expected: `/tmp/test.png` と `/tmp/test.pdf` が生成される

- [ ] **Step 3: cargo install して確認**

```bash
cargo install --path . --force
```

- [ ] **Step 4: 全テスト実行**

Run: `cargo test`
Expected: 全テスト pass

- [ ] **Step 5: コミット**

```bash
git add README.md
git commit -m "docs: update README with PNG/PDF output formats"
```
