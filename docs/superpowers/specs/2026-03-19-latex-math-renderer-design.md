# LaTeX Math Renderer 設計ドキュメント

## 概要

kaniplot のタイトル・軸ラベル内で `$...$` に囲まれたテキストを LaTeX 数式としてレンダリングする機能。SVG バックエンドでは `<text>`/`<tspan>` ベースで描画し、Latin Modern Math フォントを Base64 で SVG に埋め込むことで、どの環境でも Computer Modern 書体で表示される。

## アーキテクチャ

3層構造 + フォント埋め込み:

```
文字列 "$E = mc^2$"
    │
    ▼
LaTeX パーサー (math/parser.rs)
    │  → 数式 AST (MathNode ツリー)
    ▼
レイアウトエンジン (math/layout.rs)
    │  → Vec<LayoutGlyph> (文字, x, y, サイズ, スタイル)
    ▼
SVG 統合 (renderer/svg.rs)
    │  → <text>/<tspan> 要素として SVG に出力
    ▼
フォント埋め込み (fonts/mod.rs)
    → @font-face Base64 を SVG <defs> に挿入
```

## LaTeX パーサー

### 入力

`$...$` の内側の文字列（ドル記号は除去済み）。

### 数式 AST

```rust
enum MathNode {
    Char(char),                                    // 通常文字 (イタリック)
    Symbol(char),                                  // Unicode シンボル (\alpha → α)
    Number(String),                                // 数字列 (立体)
    Operator(char),                                // 演算子 +, -, =, etc. (立体)
    Group(Vec<MathNode>),                          // {abc}
    Superscript(Box<MathNode>, Box<MathNode>),     // x^2
    Subscript(Box<MathNode>, Box<MathNode>),       // x_i
    SubSuperscript(Box<MathNode>, Box<MathNode>, Box<MathNode>), // x_i^2
    Frac(Vec<MathNode>, Vec<MathNode>),            // \frac{a}{b}
    Accent(AccentKind, Box<MathNode>),             // \hat{x}
    TextRoman(String),                             // \mathrm{Re}
}

enum AccentKind {
    Hat,    // \hat  → U+0302
    Bar,    // \bar  → U+0304
    Vec,    // \vec  → U+20D7
    Dot,    // \dot  → U+0307
    Tilde,  // \tilde → U+0303
}
```

### 対応 LaTeX コマンド

| カテゴリ | コマンド | Unicode |
|---------|---------|---------|
| ギリシャ小文字 | `\alpha` `\beta` `\gamma` `\delta` `\epsilon` `\zeta` `\eta` `\theta` `\iota` `\kappa` `\lambda` `\mu` `\nu` `\xi` `\pi` `\rho` `\sigma` `\tau` `\upsilon` `\phi` `\chi` `\psi` `\omega` | α β γ δ ε ζ η θ ι κ λ μ ν ξ π ρ σ τ υ φ χ ψ ω |
| ギリシャ大文字 | `\Gamma` `\Delta` `\Theta` `\Lambda` `\Xi` `\Pi` `\Sigma` `\Upsilon` `\Phi` `\Psi` `\Omega` | Γ Δ Θ Λ Ξ Π Σ Υ Φ Ψ Ω |
| 大型演算子 | `\sum` `\prod` `\int` | Σ Π ∫ |
| 関係演算子 | `\leq` `\geq` `\neq` `\approx` `\equiv` `\sim` | ≤ ≥ ≠ ≈ ≡ ∼ |
| その他 | `\infty` `\partial` `\nabla` `\pm` `\mp` `\times` `\cdot` `\ldots` | ∞ ∂ ∇ ± ∓ × · … |

### パース規則

- `^` の後: 1文字 or `{...}` グループ → 上付き
- `_` の後: 1文字 or `{...}` グループ → 下付き
- `x_i^2` は `SubSuperscript` に統合
- `\command` は symbols テーブルで Unicode に解決。未知のコマンドはエラー
- `{...}` は `Group` にまとめる
- 数字の連続 (`0-9`, `.`) は `Number` としてまとめる
- `+`, `-`, `=`, `<`, `>`, `(`, `)`, `,` は `Operator`

## レイアウトエンジン

### 入力・出力

```rust
struct LayoutGlyph {
    text: String,       // 描画する文字列
    x: f64,             // ベースラインからの x オフセット (em 単位)
    y: f64,             // ベースラインからの y オフセット (em 単位, 上が負)
    font_size_ratio: f64, // ベースフォントサイズに対する比率 (1.0 = 等倍)
    italic: bool,       // イタリックかどうか
    is_math_font: bool, // Latin Modern Math を使うか
}

struct LayoutResult {
    glyphs: Vec<LayoutGlyph>,
    width: f64,         // 全体幅 (em 単位)
    height: f64,        // 全体高さ (em 単位)
    baseline: f64,      // ベースライン位置 (em 単位)
}
```

### レイアウト規則

| ノード | 処理 |
|-------|------|
| `Char` | イタリックで配置。幅は文字幅テーブル参照 |
| `Symbol` | Latin Modern Math フォントで配置 |
| `Number` | 立体 (italic=false) で配置 |
| `Operator` | 立体で配置 |
| `Superscript` | base を配置後、上付きをサイズ 0.7 倍・y を -0.4em シフトで配置 |
| `Subscript` | base を配置後、下付きをサイズ 0.7 倍・y を +0.2em シフトで配置 |
| `SubSuperscript` | base の後に上付き・下付き両方を配置 |
| `Frac` | 分子を上 (-0.7em)、分数線 (水平線)、分母を下 (+0.3em)。全体幅は max(分子幅, 分母幅)。サイズ 0.8 倍 |
| `Accent` | ベース文字の後に結合文字を重ねて配置 |
| `TextRoman` | 立体 (italic=false) で配置 |
| `Group` | 子ノードを順に横に並べて配置 |

### 文字幅

簡易的な固定幅テーブルを使用:
- 通常文字 (a-z, A-Z): 0.55em
- 数字 (0-9): 0.5em
- 演算子 (+, -, = 等): 0.6em（前後にスペース 0.15em）
- ギリシャ文字: 0.6em
- 大型演算子 (Σ, ∫ 等): 0.8em

将来 Plan 4 で `ttf-parser` を導入すればフォントメトリクスから正確な幅を取得できる。

## フォント埋め込み

### ファイル構成

```
fonts/
  latinmodern-math.woff2    # Latin Modern Math (SIL OFL)

src/fonts/
  mod.rs                    # include_bytes! + Base64 エンコード
```

### 実装

```rust
// src/fonts/mod.rs
const FONT_DATA: &[u8] = include_bytes!("../../fonts/latinmodern-math.woff2");

pub fn font_base64() -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(FONT_DATA)
}

pub fn svg_font_face_style() -> String {
    format!(
        r#"@font-face {{ font-family: "Latin Modern Math"; src: url(data:font/woff2;base64,{}) format("woff2"); }}"#,
        font_base64()
    )
}
```

依存: `base64` クレート。

### SVG への挿入

`render_svg` の `<defs>` セクション内に:

```xml
<defs>
  <style>
    @font-face {
      font-family: "Latin Modern Math";
      src: url(data:font/woff2;base64,...) format("woff2");
    }
  </style>
  <clipPath id="plot-area">...</clipPath>
</defs>
```

数式テキストが含まれる場合のみ挿入（不要な場合は SVG サイズを節約）。

## SVG 統合

### `$...$` 検出

タイトル・ラベル文字列を `$` で分割:
- 偶数インデックス: 通常テキスト → `<tspan>` (serif フォント)
- 奇数インデックス: 数式 → LaTeX パーサー → レイアウト → `<tspan>` 群 (Latin Modern Math)

例: `"Energy: $E = mc^2$"` → `["Energy: ", "E = mc^2"]`

### 描画

通常テキストは従来通り `<text>` で出力。数式部分は:

```xml
<text x="400" y="25" text-anchor="middle" font-size="18">
  Energy:
  <tspan font-family="Latin Modern Math" font-style="italic">E</tspan>
  <tspan font-family="Latin Modern Math"> = </tspan>
  <tspan font-family="Latin Modern Math" font-style="italic">m</tspan>
  <tspan font-family="Latin Modern Math" font-style="italic">c</tspan>
  <tspan font-family="Latin Modern Math" font-size="12.6" dy="-7.2" font-style="italic">2</tspan>
  <tspan dy="7.2" font-size="18"></tspan>
</text>
```

`dy` でシフトした後、リセット用の空 `<tspan>` で戻す。

## テスト戦略

- **パーサー単体テスト:** 各 LaTeX 構文 → 正しい AST ノード
- **レイアウト単体テスト:** AST → 正しい位置・サイズの LayoutGlyph 列
- **シンボルテスト:** `\alpha` → `'α'` 等のマッピング
- **SVG 統合テスト:** `$...$` を含むタイトルが正しい `<tspan>` に変換されるか
- **SVG にフォント埋め込みテスト:** 数式使用時に `@font-face` が含まれるか
- **統合テスト:** `set title "$E = mc^2$"` → SVG 出力に数式要素が含まれるか

## 注意事項

- **`dy` 累積管理:** SVG の `<tspan dy="...">` は相対オフセット。分数やネストした上下付きでは累積 `dy` を追跡し、各グリフ後にリセットする必要がある
- **リテラル `$`:** タイトル内のリテラル `$` は現時点では非対応（`\$` エスケープは将来対応可能）
- **凡例ラベル:** `PlotStyle.title`（凡例テキスト）内の `$...$` も対応する
- **ライセンス:** Latin Modern Math は SIL OFL。`fonts/OFL.txt` にライセンステキストを同梱する

## スコープ外（Plan 4 以降）

- `ttf-parser` による正確なフォントメトリクス
- グリフアウトライン（パス）ベースの描画
- `\left(` `\right)` 自動サイズ調整デリミタ
- PDF/PNG/EPS バックエンドでの数式描画
- STIX Two Math フォント対応
