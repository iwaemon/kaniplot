# kaniplot 設計ドキュメント

gnuplot互換の文法で、LaTeX数式をネイティブサポートする軽量プロットツール。

## 概要

### 動機

gnuplotの基本的なコマンド体系は優れているが、以下の不満がある：

1. タイトル・軸ラベルでLaTeX数式を使うには `epslatex` ターミナル等を経由する必要がある
2. デフォルトフォントが論文品質ではない（Computer Modern / STIXを使いたい）
3. `{/:Italic}` のような独自記法が不自然。`$...$` で囲めばイタリックになるべき

### ゴール

- gnuplot主要コマンドと高い互換性を持つCLIプロットツール
- `$...$` でLaTeX数式をネイティブレンダリング
- Computer Modern / STIXフォントをデフォルトで使用
- 起動が速い軽量なネイティブバイナリ
- PDF / SVG / PNG / EPS / ウィンドウ表示の全出力対応

### 技術選定

- **言語:** Rust
- **理由:** 起動速度が速い、シングルバイナリ配布、外部ランタイム不要、安全性

## アーキテクチャ

```
┌─────────────────────────────────────────────┐
│  CLI フロントエンド (main.rs)                │
│  - REPL（インタラクティブモード）             │
│  - スクリプトファイル読み込み                 │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  パーサー (parser/)                          │
│  - gnuplot互換コマンドの解析                  │
│  - 手書き再帰下降パーサー + pest（式言語）    │
│  - AST（抽象構文木）生成                      │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  プロットエンジン (engine/)                   │
│  - ASTからプロットモデルを構築                │
│  - 式評価器（数式関数のサンプリング）         │
│  - データ読み込み・変換                       │
│  - gnuplot準拠の自動レンジ・目盛り計算        │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  数式レンダラー (math/)                      │
│  - $...$ パーサー（LaTeXサブセット）          │
│  - 数式レイアウト → パス変換                  │
│  - フォントメトリクス（MATH テーブル参照）    │
└──────────────┬──────────────────────────────┘
               │
┌──────────────▼──────────────────────────────┐
│  レンダラー (renderer/)                      │
│  - 共通 Renderer トレイト                     │
│  - バックエンド: PDF / SVG / PNG / EPS /      │
│    ウィンドウ                                 │
└─────────────────────────────────────────────┘
```

データフローは一方向：パーサー → プロットモデル → レンダラー。数式レンダラーはプロットモデルとレンダラーの間に位置し、数式テキストをパス（グリフアウトライン）に変換してからレンダラーに渡す。

## パーサーとgnuplot互換コマンド

### パーサー実装方針

gnuplotの文法は文脈依存性が高い（`set` の各サブオプションで構文が異なる、略語解決等）ため、コマンドレベルのパーサーは **手書き再帰下降パーサー** で実装する。数式評価用の式言語（`sin(x) + x**2` 等）のみ `pest` PEGを使用する。

### 対応コマンド（初期スコープ）

| コマンド | 例 |
|---|---|
| `plot` | `plot "data.txt" using 1:2 with lines` |
| `set` / `unset` | `set xlabel "$x$"`, `set xrange [-10:10]` |
| `set title` | `set title "$E = mc^2$"` |
| `set xtics / ytics` | `set xtics 0, 0.5, 10` |
| `set key` | `set key top left` |
| `set terminal` | `set terminal pdf` |
| `set output` | `set output "graph.pdf"` |
| `replot` | 前回のプロットを再描画 |
| 数式プロット | `plot sin(x), x**2` |

**初期スコープから除外（将来対応）:**
- `splot`（3Dプロット）— 視点制御(`set view`)、z軸、曲面描画等の設計が別途必要なため後回し
- `set multiplot` — 複数サブプロットのレイアウト
- ユーザー定義関数・変数 — `f(x) = x**2`, `a = 5` 等

### プロットスタイル（初期スコープ）

| スタイル | 説明 |
|---|---|
| `lines` | 折れ線（デフォルト） |
| `points` | 散布図 |
| `linespoints` | 折れ線 + マーカー |
| `dots` | ドット |
| `impulses` | 縦線（棒） |
| `boxes` | 箱（棒グラフ） |
| `errorbars` | エラーバー付き |
| `filledcurves` | 塗りつぶし曲線 |

### 色・線スタイルシステム

- デフォルトカラーサイクル: gnuplot 5系の `podo` パレットに準拠
- `linecolor rgb "#FF0000"` / `linecolor "red"` で色指定
- `linewidth` でデフォルト1.0、数値指定
- `dashtype` で実線/破線/点線等
- `pointtype` / `pointsize` でマーカー形状・サイズ

### gnuplot準拠のデフォルト値

- `xrange` / `yrange`: `[*:*]`（データから自動決定、autoscale）
- 目盛り: 自動で「きりのいい数値」に配置（gnuplotと同じアルゴリズム）
- ボーダー: 左と下の軸線のみ（gnuplotデフォルトの `set border 3` 相当）
- 目盛り方向: 内向き（inward）
- 凡例（key）: 右上に表示

### 略語システム

gnuplotと同じ略語をサポート。コマンド名・オプション名を先頭からの任意長プレフィックスで一意に特定する。

- `se xra[0:1]` → `set xrange [0:1]`
- `rep` → `replot`
- `p` → `plot`
- `sp` → `splot`

曖昧な場合（複数候補にマッチ）はエラーメッセージで候補一覧を表示。

### 独自拡張

- 文字列中の `$...$` を自動的にLaTeX数式として解釈
- `set font "CMU Serif"` / `set font "STIX"` でフォント切り替え
- デフォルトフォントはComputer Modern（バイナリ埋め込み）

## 式評価器

### 概要

`plot sin(x), x**2 + 3*cos(x)` のような数式プロットを実現するための式評価エンジン。

### 式パーサー

`pest` PEG文法で実装。以下の演算子を優先順位順にサポート：

| 優先順位 | 演算子 | 結合性 |
|---|---|---|
| 1（最低） | `? :` (三項) | 右 |
| 2 | `\|\|` | 左 |
| 3 | `&&` | 左 |
| 4 | `==`, `!=`, `<`, `>`, `<=`, `>=` | 左 |
| 5 | `+`, `-` | 左 |
| 6 | `*`, `/`, `%` | 左 |
| 7 | `**` | 右 |
| 8（最高） | 単項 `-`, `!` | 右 |

### 変数と定数

- `x` : plotの独立変数
- `pi` : 円周率
- `e` : ネイピア数

### 組み込み関数

`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `log`, `log10`, `sqrt`, `abs`, `ceil`, `floor`, `int`

### サンプリング戦略

- デフォルトサンプル数: 1000点（gnuplot準拠、`set samples` で変更可能）
- xrange内で等間隔にサンプリング
- 将来拡張: 急峻な変化点付近での適応的サンプリング

## データファイル読み込み

### ファイルフォーマット

- 空白区切り（スペース/タブ）のカラムデータ
- `#` で始まる行はコメント
- 空行はデータブロックの区切り（`index` で選択可能）
- 欠損値（`?` または空カラム）は描画をスキップ

### `using` 指定

- カラム番号指定: `using 1:2`, `using 1:2:3`（エラーバー用）
- カラム式: `using 1:($2*1000)` — `$N` でN番目のカラムを参照、括弧内で四則演算が可能

### データ区切り

- `index N` でN番目のデータブロック（空行区切り）を選択
- `every N` でN行おきにプロット

## AST 定義スケッチ

```rust
/// トップレベルコマンド
enum Command {
    Plot(PlotCommand),
    Set(SetCommand),
    Unset(SetProperty),     // 型安全なプロパティ指定
    Replot,
    Quit,
}

/// plot コマンド
struct PlotCommand {
    series: Vec<PlotSeries>,
}

enum PlotSeries {
    Expression {
        expr: Expr,                // 数式 (sin(x) 等)
        style: PlotStyle,
    },
    DataFile {
        path: String,
        using: Option<UsingSpec>,  // using 1:2
        index: Option<usize>,
        every: Option<usize>,
        style: PlotStyle,
    },
}

struct PlotStyle {
    kind: StyleKind,               // lines, points, etc.
    line_color: Option<Color>,
    line_width: Option<f64>,
    dash_type: Option<DashType>,
    point_type: Option<u32>,
    point_size: Option<f64>,
    title: Option<String>,         // 凡例ラベル
}

enum StyleKind {
    Lines, Points, LinesPoints, Dots,
    Impulses, Boxes, ErrorBars, FilledCurves,
}

/// set/unset で使うプロパティ識別子
enum SetProperty {
    XRange, YRange, Title, XLabel, YLabel,
    Terminal, Output, Key, XTics, YTics,
    Border, Font, Samples,
}

/// set コマンド
enum SetCommand {
    XRange(Range),
    YRange(Range),
    Title(String),
    XLabel(String),
    YLabel(String),
    Terminal(TerminalType),
    Output(String),
    Key(KeyOptions),
    XTics(TicsSpec),
    YTics(TicsSpec),
    Border(u32),
    Font(String),
    Samples(usize),
    // ... 他のオプション
}

/// セッション状態（set コマンドの累積状態を保持）
struct SessionState {
    xrange: Range,          // デフォルト: [*:*] (autoscale)
    yrange: Range,
    title: Option<String>,
    xlabel: Option<String>,
    ylabel: Option<String>,
    terminal: TerminalType, // デフォルト: TTY→Window, パイプ→PNG
    output: Option<String>, // None = stdout
    key: KeyOptions,        // デフォルト: 右上表示
    xtics: TicsSpec,        // デフォルト: 自動
    ytics: TicsSpec,
    border: u32,            // デフォルト: 3 (左+下)
    font: String,           // デフォルト: "CMU Serif"
    samples: usize,         // デフォルト: 1000
    last_plot: Option<PlotCommand>, // replot用
}

/// 数式 AST
enum Expr {
    Number(f64),
    Variable(String),           // x, pi, e
    ColumnRef(usize),           // $1, $2 等（using式内でのカラム参照）
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    FuncCall(String, Vec<Expr>), // sin(x), atan2(y,x)
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}
```

## 数式レンダラー

### LaTeXサブセット対応範囲

| 記法 | 例 | 結果 |
|---|---|---|
| 上付き | `$x^2$`, `$x^{10}$` | x² |
| 下付き | `$x_i$`, `$x_{ij}$` | xᵢ |
| 分数 | `$\frac{a}{b}$` | a/b |
| ギリシャ文字 | `$\alpha$`, `$\omega$` | α, ω |
| 演算子 | `$\sum$`, `$\int$`, `$\prod$` | Σ, ∫, Π |
| アクセント | `$\hat{x}$`, `$\bar{x}$`, `$\vec{x}$` | x̂, x̄, x⃗ |
| 括弧 | `$\left( \right)$` | 自動サイズ調整 |
| テキスト | `$\mathrm{Re}$` | 立体（ローマン） |

### フォント戦略

- **Computer Modern (Latin Modern Math OTF)** をデフォルトとしてバイナリに静的埋め込み（SIL Open Font License）
- **STIX Two Math** を代替フォントとして同梱
- `$...$` 内はデフォルトでイタリック（数式モード）、`$...$` 外は通常テキスト
- フォントメトリクス読み取りには `ttf-parser` を使用（OpenType MATHテーブル対応）
- グリフのアウトライン（パス）は `ttf-parser` から直接取得。ラスタライズはPNG/ウィンドウバックエンドで `tiny-skia` が担当
- 埋め込みフォントは最小構成: テキスト用1ウェイト（Regular）+ 数式用（Math OTF）。STIX Twoはオプション（`--features stix` でビルド時選択）

### 数式レイアウトエンジン

- TeXの組版ルール（Knuthの *The TeXbook* Appendix G）を簡略化して実装
- `ttf-parser` で OpenType MATHテーブルからメトリクスを取得（上付き/下付き位置、分数線の太さ、デリミタ構築情報等）
- 数式レイアウトの出力はグリフアウトライン（パス）のリスト。各バックエンドはパスとして描画するだけでよい
- 完全なTeXエンジンではなく、上記表の範囲に絞った軽量実装

## Renderer トレイト定義スケッチ

```rust
/// デバイス座標系での位置
struct Point {
    x: f64,
    y: f64,
}

/// 描画パス（数式グリフ、複雑な図形に使用）
struct Path {
    commands: Vec<PathCommand>,
}

enum PathCommand {
    MoveTo(Point),
    LineTo(Point),
    CurveTo(Point, Point, Point), // 3次ベジェ
    ClosePath,
}

/// 全バックエンドが実装するトレイト
trait Renderer {
    fn begin(&mut self, width: f64, height: f64);
    fn end(&mut self);

    // プリミティブ描画
    fn draw_line(&mut self, from: Point, to: Point, style: &LineStyle);
    fn draw_rect(&mut self, origin: Point, size: Point, style: &FillStyle);
    fn draw_circle(&mut self, center: Point, radius: f64, style: &FillStyle);
    fn draw_path(&mut self, path: &Path, style: &FillStyle);

    // テキスト描画（$...$を含まない通常テキスト）
    fn draw_text(&mut self, pos: Point, text: &str, style: &TextStyle);

    // 数式描画（math/ が変換したパスリストを描画）
    // → draw_path を複数回呼ぶヘルパーとしてデフォルト実装
    fn draw_math_glyphs(&mut self, pos: Point, glyphs: &[PositionedPath]) {
        for glyph in glyphs {
            let offset_path = glyph.path.translate(pos.x + glyph.x, pos.y + glyph.y);
            self.draw_path(&offset_path, &glyph.style);
        }
    }
}

/// 数式レンダラーが出力する配置済みグリフパス
struct PositionedPath {
    x: f64,
    y: f64,
    path: Path,
    style: FillStyle,
}
```

数式はレンダラーに渡る前にパスに変換済みのため、各バックエンドは `draw_path` さえ実装すれば数式を描画できる。

### 座標系

- **エンジン座標系:** 原点は左下、単位はポイント（1pt = 1/72インチ）。プロットモデルがデータ座標→エンジン座標変換を行う
- **デバイス座標変換:** 各バックエンドがエンジン座標を自身の座標系に変換
  - SVG / PNG / ウィンドウ: 原点を左上に反転（Y軸反転）
  - PDF: そのまま（PDFの座標系は左下原点）
  - EPS: そのまま（PostScriptも左下原点）

### テキスト描画パイプライン

通常テキスト（`$...$` を含まない）の描画は各バックエンドで異なる：

- **SVG:** `<text>` 要素として出力（フォント名を指定、閲覧環境にフォントが必要）
- **PDF:** `pdf-writer` + `subsetter` でフォントサブセットを埋め込み、テキストをグリフIDで配置
- **PNG / ウィンドウ:** `ttf-parser` でグリフアウトラインを取得し `tiny-skia` でラスタライズ
- **EPS:** PostScript `show` オペレータでテキスト出力（フォント埋め込みまたは名前参照）

### ポリライン描画

データ系列の折れ線は `draw_polyline` で効率的に描画：

```rust
fn draw_polyline(&mut self, points: &[Point], style: &LineStyle) {
    // デフォルト実装: draw_line の連続呼び出し
    for pair in points.windows(2) {
        self.draw_line(pair[0], pair[1], style);
    }
}
```

各バックエンドはオーバーライド可能：SVGは `<polyline>` 、PDFは単一パス、EPSは `moveto`/`lineto` シーケンスで効率化。

## レンダリングバックエンド

### バックエンド一覧

| バックエンド | クレート/方式 | 備考 |
|---|---|---|
| **SVG** | 自前生成（文字列結合） | パスを `<path d="...">` として出力 |
| **PDF** | `pdf-writer` + `subsetter` | フォントサブセット埋め込み |
| **PNG** | `tiny-skia`（CPUラスタライザ） | GPU不要、pure Rust、依存少 |
| **EPS** | 自前生成（PostScript直書き） | テキスト形式で比較的単純 |
| **ウィンドウ** | `minifb` | 起動速度重視で最軽量を選択 |

### インタラクティブウィンドウ機能

- ズーム（マウスホイール）
- パン（ドラッグ）
- 座標表示（マウス位置）
- ウィンドウリサイズで再描画

注: `minifb` はフレームバッファのみ提供するため、描画はすべて `tiny-skia` でラスタライズしてからバッファに転送する。ズーム/パン時は再ラスタライズが必要。

### 依存の軽さ

- `tiny-skia`: pure Rust、外部Cライブラリ不要
- `minifb`: 軽量（macOS: Cocoa, Linux: X11/Wayland, Windows: WinAPI直接）
- Cairoのような重い外部依存を避ける

## CLI と REPL

### 起動モード

```bash
# スクリプト実行
kaniplot script.plt

# 標準入力から（デフォルトターミナルはPNG→stdout）
echo 'plot sin(x)' | kaniplot > out.png

# REPLモード（引数なしで起動）
kaniplot
> set terminal pdf
> set output "out.pdf"
> plot sin(x) with lines
> rep
```

### パイプモード

標準入力がTTYでない場合（パイプ経由）:
- デフォルトターミナルは `png`（stdoutにPNGバイナリを出力）
- `set terminal` / `set output` で明示指定があればそちらに従う

### REPL機能

- `rustyline` クレートで行編集・履歴対応（上下キーで過去コマンド呼び出し）
- コマンド履歴を `~/.kaniplot_history` に保存
- タブ補完（コマンド名、オプション名）
- `q` / `quit` / `exit` で終了

### デフォルトターミナル

- TTYモード（REPL）: ウィンドウ表示（gnuplotの `set terminal wxt` 相当）
- パイプモード: PNG出力（stdout）
- `set terminal pdf` / `set output` で出力切り替え
- `replot` / `rep` でターミナル変更後に再描画

### エラー処理とステート管理

- パースエラー時は該当箇所と問題点を表示
- 略語が曖昧な場合は候補一覧を表示
- 未対応コマンドは明示的に「未対応」と表示（黙って無視しない）
- `set` コマンドの状態は `plot` の成否に関わらず保持（gnuplot準拠）
- エラー後もREPLセッションは継続

## プロジェクト構成

```
kaniplot/
├── Cargo.toml
├── src/
│   ├── main.rs            # CLI エントリポイント・REPL
│   ├── parser/            # gnuplot互換パーサー
│   │   ├── mod.rs         # 手書き再帰下降パーサー（コマンドレベル）
│   │   ├── ast.rs         # AST定義
│   │   └── expr.pest      # 式言語のPEG文法
│   ├── engine/            # プロットモデル構築
│   │   ├── mod.rs
│   │   ├── model.rs       # 軸、系列、スタイル等の構造体
│   │   ├── evaluator.rs   # 式評価器
│   │   ├── data.rs        # データファイル読み込み
│   │   ├── autoscale.rs   # gnuplot準拠の自動レンジ・目盛り計算
│   │   └── functions.rs   # sin, cos等の組み込み関数
│   ├── math/              # LaTeX数式レンダラー
│   │   ├── mod.rs
│   │   ├── tex_parser.rs  # $...$ パーサー
│   │   └── layout.rs      # 数式レイアウト → パス変換
│   ├── renderer/          # 出力バックエンド
│   │   ├── mod.rs         # Renderer トレイト定義
│   │   ├── svg.rs
│   │   ├── pdf.rs
│   │   ├── png.rs
│   │   ├── eps.rs
│   │   └── window.rs      # インタラクティブウィンドウ
│   └── fonts/             # 埋め込みフォントデータ
│       ├── mod.rs
│       └── embedded.rs    # include_bytes! でOTF埋め込み
├── fonts/                 # フォントファイル（ビルド時埋め込み）
│   ├── latinmodern-math.otf
│   └── STIXTwoMath.otf   # オプション（feature flag）
└── tests/
    ├── unit/              # パーサー、式評価器、数式レイアウトの単体テスト
    └── integration/       # gnuplotスクリプトとの出力比較テスト
```

### 主要依存クレート

| 用途 | クレート |
|---|---|
| 式パーサー | `pest` |
| 行編集・REPL | `rustyline` |
| PNGラスタライズ | `tiny-skia` |
| PDF出力 | `pdf-writer` + `subsetter` |
| フォント解析（MATHテーブル） | `ttf-parser` |
| テキスト/数式ラスタライズ（PNG/ウィンドウ） | `tiny-skia`（パス描画として処理） |
| ウィンドウ表示 | `minifb` |
| コマンドパーサー | 自前実装（再帰下降） |
| 数式パーサー・レイアウト | 自前実装 |
| SVG / EPS | 自前生成 |

### テスト戦略

- **単体テスト:** パーサー（コマンド解析・略語解決）、式評価器、数式レイアウトエンジン
- **統合テスト:** gnuplotスクリプトを入力し、SVG出力のテキスト内容を比較（SVGはテキスト形式なのでdiffしやすい）。リファレンスSVGは手動で検証済みのものをリポジトリに含める
- **ビジュアルテスト:** PNG出力のピクセル差分比較（許容閾値付き）。CIでの回帰検出用

### ビルド

- `cargo build --release` でシングルバイナリ生成
- フォントは `include_bytes!` でバイナリに埋め込み（外部ファイル不要）
- `--features stix` で STIX Two Math フォントも埋め込み（デフォルトはLatin Modern Mathのみでバイナリサイズ削減）
- クロスコンパイル対応（`cross` クレート）
