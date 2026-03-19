# kaniplot

gnuplot 互換のプロッティングツール（Rust 製）。数式からSVGグラフを生成します。

## インストール

```bash
git clone https://github.com/ShumpeiSatworker/kaniplot.git
cd kaniplot
cargo build --release
```

バイナリは `target/release/kaniplot` に生成されます。

## 使い方

### スクリプトファイルを実行

```bash
kaniplot script.gp
```

### パイプモード（標準入力）

```bash
echo 'plot sin(x)' | kaniplot > output.svg
```

### ヘルプを表示

```bash
kaniplot --help
```

## 基本的なコマンド

### plot — グラフを描画

```gnuplot
plot sin(x)
plot sin(x), cos(x), x**2
plot sin(x) with lines title "Sine"
plot x**2 with points linecolor rgb "#FF0000"
```

### set / unset — プロパティの設定・解除

```gnuplot
set title "My Plot"
set xlabel "Time (s)"
set ylabel "Amplitude"
set xrange [-3.14:3.14]
set yrange [-1.5:1.5]
set output "plot.svg"
set key top left
set samples 2000
set border 3

unset title
```

### replot — 前回のプロットを再描画

```gnuplot
plot sin(x)
set title "Updated"
replot
```

## データファイルのプロット

空白区切りのカラムデータをプロットできます。

```gnuplot
plot "data.dat" with lines
plot "data.dat" using 1:3 with points
plot "data.dat" using 1:($2*1000) with lines
plot "data.dat" index 1 with lines
plot "data.dat" every 2 with lines
```

### データファイルの形式

- 空白（スペース/タブ）区切りのカラムデータ
- `#` で始まる行はコメント（スキップ）
- 空行はデータブロックの区切り
- `?` は欠損値（描画をスキップ）

```
# 実験データ
1  2.3
2  4.1
3  5.8

# 別のデータセット
1  10.0
2  20.5
```

### using — カラム選択

```gnuplot
plot "data.dat" using 1:2          # 1列目をx、2列目をy
plot "data.dat" using 1:3          # 1列目をx、3列目をy
plot "data.dat" using 1:($2*100)   # カラム式：$Nで列参照
```

### index — ブロック選択

空行で区切られたデータブロックを番号（0始まり）で選択します。

```gnuplot
plot "data.dat" index 0    # 最初のブロック
plot "data.dat" index 1    # 2番目のブロック
```

### every — 行の間引き

```gnuplot
plot "data.dat" every 2    # 2行おきにプロット
```

### データとモデルの重ね描き

```gnuplot
plot "data.dat" with points title "Data", x**2 with lines title "Model"
```

## プロットスタイル

| スタイル | 説明 |
|----------|------|
| `lines` | 折れ線（デフォルト） |
| `points` | 散布図 |
| `linespoints` | 折れ線 + 点 |
| `dots` | 小さい点 |
| `impulses` | 棒グラフ（垂直線） |
| `boxes` | 箱グラフ |

```gnuplot
plot sin(x) with points, cos(x) with impulses
```

## サポートされている関数

`sin`, `cos`, `tan`, `asin`, `acos`, `atan`, `atan2`, `exp`, `log`, `log10`, `sqrt`, `abs`, `ceil`, `floor`, `int`

## 演算子

| 演算子 | 説明 |
|--------|------|
| `+` `-` `*` `/` `%` | 四則演算・剰余 |
| `**` | べき乗 |
| `==` `!=` `<` `>` `<=` `>=` | 比較 |
| `&&` `\|\|` `!` | 論理演算 |
| `? :` | 三項演算子 |

## スクリプト例

```gnuplot
set title "Trigonometric Functions"
set xlabel "x"
set ylabel "y"
set xrange [-6.28:6.28]
set yrange [-1.5:1.5]
set output "trig.svg"
set samples 1000

plot sin(x) title "sin(x)", cos(x) title "cos(x)"
```

```bash
kaniplot trig.gp
# -> trig.svg が生成される
```

## コマンド省略形

gnuplot と同様にコマンドを省略できます：

```gnuplot
p sin(x)          # plot sin(x)
se tit "Hello"    # set title "Hello"
se xr [-5:5]      # set xrange [-5:5]
```

## LaTeX 数式

タイトル・軸ラベル・凡例で `$...$` を使って数式を記述できます。

```gnuplot
set title "$E = mc^2$"
set xlabel "$\omega$ (rad/s)"
set ylabel "$|\psi|^2$"
plot sin(x) title "$\sin(x)$"
```

### サポートされている記法

| 記法 | 例 | 結果 |
|------|-----|------|
| 上付き | `$x^2$`, `$x^{10}$` | x² |
| 下付き | `$x_i$` | xᵢ |
| 分数 | `$\frac{a}{b}$` | a/b |
| ギリシャ文字 | `$\alpha$`, `$\Omega$` | α, Ω |
| アクセント | `$\hat{x}$`, `$\bar{x}$` | x̂, x̄ |
| 演算子 | `$\sum$`, `$\int$` | Σ, ∫ |
| ローマン体 | `$\mathrm{Re}$` | Re |

数式は Latin Modern Math フォントで描画されます（SVG に埋め込み済み）。

### シェルでの注意

`echo` でパイプする場合、`\alpha` などのバックスラッシュが制御文字として解釈されることがあります。スクリプトファイルか `printf` を使用してください：

```bash
# NG: echo はバックスラッシュを解釈してしまう
echo 'set xlabel "$\alpha$"' | kaniplot    # \a が BEL 文字になる

# OK: printf を使う
printf 'set xlabel "$\\alpha$"\nplot sin(x)\n' | kaniplot

# OK: スクリプトファイルを使う（推奨）
kaniplot script.gp
```

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

## ライセンス

MIT
