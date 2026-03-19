# REPL & LaTeX パーサー改善 設計ドキュメント

## 概要

kaniplot にインタラクティブ REPL モードを追加し、LaTeX 数式パーサーを改善する。REPL は `rustyline` による行編集・履歴付き。LaTeX パーサーは演算子コマンド（`\sin` 等）、`|` 文字、`\sqrt` を新たにサポートする。

## 1. インタラクティブ REPL

### 動作

```
$ kaniplot              # 引数なし + TTY → REPL モード
kaniplot> set terminal png
kaniplot> set output "plot.png"
kaniplot> plot sin(x)
kaniplot> quit
```

- 引数なし + stdin が TTY の場合に REPL モードに入る
- パイプ入力（`echo '...' | kaniplot`）は従来通りバッチモード
- ファイル引数（`kaniplot script.gp`）も従来通り

### 実装

- `std::io::IsTerminal` トレイト（Rust 1.70+）で TTY 判定
- `rustyline::Editor` でプロンプト表示・行読み取り
- プロンプト: `"kaniplot> "`
- 各行を `parser::parse_script()` で解析し即時実行
- `quit`/`exit` または Ctrl+D (EOF) で終了
- 履歴ファイル: `~/.kaniplot_history`（読み込み失敗は無視）

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `Cargo.toml` | `rustyline = "15"` 追加 |
| `src/main.rs` | REPL ループ追加。TTY 判定で分岐 |

### エラーハンドリング

- パースエラー: `eprintln!` でメッセージ表示、REPL は継続
- プロットエラー: 同上
- `rustyline` エラー（Ctrl+C）: 行をキャンセルして次のプロンプト表示

## 2. LaTeX 演算子コマンド

### 追加コマンド

| コマンド | 表示 | スタイル |
|---------|------|---------|
| `\sin` | sin | ローマン体 |
| `\cos` | cos | ローマン体 |
| `\tan` | tan | ローマン体 |
| `\log` | log | ローマン体 |
| `\exp` | exp | ローマン体 |
| `\ln` | ln | ローマン体 |
| `\lim` | lim | ローマン体 |
| `\max` | max | ローマン体 |
| `\min` | min | ローマン体 |

### 実装

`src/math/parser.rs` のコマンドディスパッチで、上記コマンドを認識し `MathNode::TextRoman(String)` として返す。既存の `TextRoman` ノードがそのまま使える（ローマン体・非イタリック）。

```rust
// parser.rs のコマンドディスパッチ内
"sin" | "cos" | "tan" | "log" | "exp" | "ln" | "lim" | "max" | "min" => {
    nodes.push(MathNode::TextRoman(cmd.to_string()));
}
```

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `src/math/parser.rs` | 演算子コマンドの認識を追加 |

## 3. `|` 文字対応

### 問題

現在 `|` は数式パーサーで認識されず、`$|\psi|^2$` が正しくパースされない。

### 実装

`src/math/parser.rs` で `|` を `MathNode::Operator('|')` として扱う。

```rust
'|' => nodes.push(MathNode::Operator('|')),
```

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `src/math/parser.rs` | `|` を Operator として認識 |

## 4. `\sqrt{x}` 対応

### AST

```rust
// src/math/parser.rs
enum MathNode {
    // ... 既存ノード ...
    Sqrt(Vec<MathNode>),  // \sqrt{x}
}
```

### パーサー

`\sqrt` コマンドの後に `{...}` グループを読み取り、`MathNode::Sqrt(children)` を生成。

### レイアウト

```rust
// src/math/layout.rs
MathNode::Sqrt(children) => {
    // √ 記号を配置
    self.push("√".to_string(), 0.6, false, true);
    // 子ノードをレイアウト（位置を記録）
    let start_x = self.x;
    for child in children {
        self.layout_node(child);
    }
    let end_x = self.x;
    // オーバーラインの幅を LayoutGlyph のメタデータとして記録
}
```

### SVG レンダリング

`\sqrt` のオーバーライン（根号の上の水平線）は `<tspan>` では表現できない。以下の方式で対応：

- `LayoutGlyph` に `overline: Option<f64>` フィールドを追加（オーバーラインの幅、em 単位）
- `render_math_text` で `overline` が `Some` のグリフを検出し、`<line>` 要素を数式テキストの後に出力
- オーバーラインの y 位置は文字の上端（ベースラインから -0.75em）

```xml
<!-- √x の例 -->
<text x="400" y="25" text-anchor="middle" font-size="18">
  <tspan font-family="Latin Modern Math">√</tspan>
  <tspan font-family="Latin Modern Math" font-style="italic">x</tspan>
</text>
<line x1="..." y1="..." x2="..." y2="..." stroke="black" stroke-width="0.5"/>
```

### ファイル変更

| ファイル | 変更内容 |
|---------|---------|
| `src/math/parser.rs` | `Sqrt` ノード追加、`\sqrt` コマンド認識 |
| `src/math/layout.rs` | `Sqrt` レイアウト処理、`overline` フィールド追加 |
| `src/renderer/svg.rs` | `render_math_text` でオーバーライン `<line>` 出力 |

## テスト戦略

### REPL
- `rustyline` のテストは難しいため、統合テストで `echo "plot sin(x)" | kaniplot` のパイプモードが従来通り動作することを確認
- REPL 固有のテストは手動確認

### LaTeX 演算子
- パーサーテスト: `\sin` → `TextRoman("sin")`
- レイアウトテスト: `\sin(x)` のグリフ列が正しいか（sin がローマン体、x がイタリック）
- SVG テスト: `$\sin(x)$` が正しい `<tspan>` に変換されるか

### `|` 文字
- パーサーテスト: `|\psi|^2` → 正しい AST
- SVG テスト: `$|\psi|^2$` が正しくレンダリングされるか

### `\sqrt`
- パーサーテスト: `\sqrt{x}` → `Sqrt([Char('x')])`
- レイアウトテスト: √ 記号 + 子ノードの位置が正しいか
- SVG テスト: オーバーライン `<line>` が出力されるか

## 依存クレート

| クレート | バージョン | 用途 |
|---------|----------|------|
| `rustyline` | 15 | REPL の行編集・履歴 |

## スコープ外

- タブ補完（コマンド名の補完）
- REPL 内のヘルプコマンド（`help` で使い方表示）
- `\sqrt[3]{x}` — n 乗根（将来対応可能）
- `\overline{x}` — 一般的なオーバーライン
- `\text{}` — テキストモード
