# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

kaniplot is a gnuplot-compatible plotting tool written in Rust that generates SVG, PNG, and PDF output. It supports LaTeX math rendering in titles/labels using Unicode Mathematical Italic glyphs and an embedded Latin Modern Math font.

## Build & Development Commands

```bash
cargo build                    # Debug build
cargo build --release          # Release build
cargo install --path .         # Install to ~/.cargo/bin/kaniplot

cargo test                     # Run all tests (unit + integration + doc)
cargo test --lib               # Unit tests only
cargo test --test integration  # Integration tests only
cargo test <test_name>         # Run a single test by name

cargo clippy                   # Lint
cargo fmt --check              # Check formatting
cargo fmt                      # Apply formatting

# Usage
echo 'plot sin(x)' | kaniplot > output.svg
kaniplot script.gp
```

## Architecture

The system follows a pipeline: **Parse → Session State → PlotModel → Render**.

### Modules

- **`parser/`** — Tokenizes gnuplot scripts into `Command` AST nodes. Uses a hand-written tokenizer for commands and Pest (PEG grammar in `expr.pest`) for math expressions. Supports gnuplot abbreviations (e.g., `p` → `plot`, `se` → `set`).

- **`engine/`** — Core computation layer:
  - `session.rs`: `SessionState` accumulates `set`/`unset` commands (ranges, labels, terminal, samples)
  - `mod.rs`: `build_plot_model()` orchestrates data loading, auto-scaling, and expression evaluation into a `PlotModel`
  - `evaluator.rs`: Evaluates math expressions with variables (`x`, `pi`, `e`), operators, ternary, and 14 built-in functions
  - `data.rs`: Parses whitespace-delimited data files (supports `#` comments, `?` missing values, multi-block with blank-line separators)
  - `model.rs`: `PlotModel` is the renderable intermediate representation

- **`renderer/`** — Multi-format output:
  - `svg.rs`: Primary format, generates SVG with embedded Base64 font
  - `png.rs`: Rasterizes SVG at 150 DPI via `resvg`
  - `pdf.rs`: Converts SVG to vector PDF via `svg2pdf`

- **`math/`** — LaTeX math rendering for `$...$` delimited text in titles/labels:
  - `parser.rs`: Recursive descent parser for LaTeX subset (superscript, subscript, `\frac`, Greek letters, accents)
  - `layout.rs`: Glyph positioning and sizing
  - `symbols.rs`: Unicode Mathematical Italic mappings (U+1D44E–U+1D714)

- **`fonts/`** — Embeds Latin Modern Math OTF as Base64

- **`main.rs`** — CLI entry point handling file/stdin input and output routing

### Key Types

- `Command` (AST node) → `SessionState` (accumulated state) → `PlotModel` (renderable model) → SVG/PNG/PDF output

## Testing

- **144 unit tests** in module files, **14 integration tests** in `tests/integration.rs`
- Test data files in `tests/testdata/`
- Known failure: `math::layout::tests::test_mathrm_not_italic` (font handling edge case)

## Dependencies

No external deps for CLI parsing or core logic. Key rendering dependencies: `pest`/`pest_derive` (PEG parsing), `resvg`/`usvg`/`tiny-skia` (PNG rasterization), `svg2pdf` (PDF), `base64` (font embedding).
