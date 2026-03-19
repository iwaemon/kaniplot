// src/parser/mod.rs
pub mod ast;
pub mod expr_parser;
pub(crate) mod abbreviation;

use ast::*;
use abbreviation::resolve;
use expr_parser::parse_expr;

/// Parse a single command line into a Command.
/// Returns Ok(None) for empty lines and comments.
pub fn parse_command(input: &str) -> Result<Option<Command>, String> {
    let input = input.trim();
    if input.is_empty() || input.starts_with('#') {
        return Ok(None);
    }

    let mut tokens = Tokenizer::new(input);
    let cmd_word = tokens.next_word().ok_or("Empty command")?;

    // "exit" is not part of the normal abbreviation table
    if cmd_word == "exit" {
        return Ok(Some(Command::Quit));
    }

    let cmds = &["plot", "set", "unset", "replot", "quit"];
    let cmd_name = resolve(&cmd_word, cmds)?;

    match cmd_name {
        "plot"   => parse_plot(&mut tokens).map(|c| Some(Command::Plot(c))),
        "set"    => parse_set(&mut tokens).map(|c| Some(Command::Set(c))),
        "unset"  => parse_unset(&mut tokens).map(|p| Some(Command::Unset(p))),
        "replot" => Ok(Some(Command::Replot)),
        "quit"   => Ok(Some(Command::Quit)),
        _        => Err(format!("Unknown command: {cmd_name}")),
    }
}

/// Parse multiple lines (script).
pub fn parse_script(input: &str) -> Result<Vec<Command>, String> {
    let mut commands = Vec::new();
    for (line_num, line) in input.lines().enumerate() {
        match parse_command(line) {
            Ok(Some(cmd)) => commands.push(cmd),
            Ok(None) => {} // empty line or comment
            Err(e) => return Err(format!("Line {}: {e}", line_num + 1)),
        }
    }
    Ok(commands)
}

// --- Tokenizer ---

struct Tokenizer<'a> {
    input: &'a str,
    pos: usize,
}

impl<'a> Tokenizer<'a> {
    fn new(input: &'a str) -> Self {
        Self { input, pos: 0 }
    }

    fn skip_whitespace(&mut self) {
        while self.pos < self.input.len()
            && self.input.as_bytes()[self.pos].is_ascii_whitespace()
        {
            self.pos += 1;
        }
    }

    fn remaining(&self) -> &'a str {
        &self.input[self.pos..]
    }

    fn next_word(&mut self) -> Option<String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() {
            return None;
        }
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_whitespace() || ch == b',' || ch == b'[' {
                break;
            }
            self.pos += 1;
        }
        if self.pos == start {
            None
        } else {
            Some(self.input[start..self.pos].to_lowercase())
        }
    }

    fn next_quoted_string(&mut self) -> Result<String, String> {
        self.skip_whitespace();
        if self.pos >= self.input.len() || self.input.as_bytes()[self.pos] != b'"' {
            return Err("Expected quoted string".into());
        }
        self.pos += 1; // skip opening quote
        let start = self.pos;
        while self.pos < self.input.len() && self.input.as_bytes()[self.pos] != b'"' {
            self.pos += 1;
        }
        if self.pos >= self.input.len() {
            return Err("Unterminated string".into());
        }
        let s = self.input[start..self.pos].to_string();
        self.pos += 1; // skip closing quote
        Ok(s)
    }

    fn next_number(&mut self) -> Result<f64, String> {
        self.skip_whitespace();
        let start = self.pos;
        while self.pos < self.input.len() {
            let ch = self.input.as_bytes()[self.pos];
            if ch.is_ascii_digit() || ch == b'.' || ch == b'-' || ch == b'+' || ch == b'e' || ch == b'E' {
                self.pos += 1;
            } else {
                break;
            }
        }
        self.input[start..self.pos]
            .parse::<f64>()
            .map_err(|e| format!("Invalid number: {e}"))
    }

    fn peek_char(&self) -> Option<char> {
        let rem = self.remaining().trim_start();
        rem.chars().next()
    }

    fn expect_char(&mut self, expected: char) -> Result<(), String> {
        self.skip_whitespace();
        if self.pos < self.input.len() && self.input[self.pos..].starts_with(expected) {
            self.pos += expected.len_utf8();
            Ok(())
        } else {
            Err(format!("Expected '{expected}' at position {}", self.pos))
        }
    }
}

// --- plot parser ---

fn parse_plot(tokens: &mut Tokenizer) -> Result<PlotCommand, String> {
    let mut series = Vec::new();

    loop {
        tokens.skip_whitespace();
        if tokens.remaining().is_empty() {
            break;
        }

        let s = parse_plot_series(tokens)?;
        series.push(s);

        tokens.skip_whitespace();
        if tokens.remaining().starts_with(',') {
            tokens.pos += 1; // consume comma
        } else {
            break;
        }
    }

    if series.is_empty() {
        return Err("plot: expected at least one series".into());
    }

    Ok(PlotCommand { series })
}

fn parse_plot_series(tokens: &mut Tokenizer) -> Result<PlotSeries, String> {
    tokens.skip_whitespace();

    // Check if it's a data file (starts with ")
    if tokens.peek_char() == Some('"') {
        let path = tokens.next_quoted_string()?;
        let mut using = None;
        let mut index = None;
        let mut every = None;
        let mut style = PlotStyle::default();

        // Parse optional modifiers
        loop {
            tokens.skip_whitespace();
            let remaining = tokens.remaining();
            if remaining.is_empty() || remaining.starts_with(',') {
                break;
            }

            let save_pos = tokens.pos;
            if let Some(word) = tokens.next_word() {
                match try_resolve_plot_option(&word) {
                    Some("using") => {
                        using = Some(parse_using_spec(tokens)?);
                    }
                    Some("index") => {
                        index = Some(tokens.next_number()? as usize);
                    }
                    Some("every") => {
                        every = Some(tokens.next_number()? as usize);
                    }
                    Some("with") => {
                        style.kind = parse_style_kind(tokens)?;
                    }
                    Some("title") => {
                        style.title = Some(tokens.next_quoted_string()?);
                    }
                    Some("linewidth") | Some("lw") => {
                        style.line_width = Some(tokens.next_number()?);
                    }
                    Some("linecolor") | Some("lc") => {
                        style.line_color = Some(parse_color(tokens)?);
                    }
                    Some("pointtype") | Some("pt") => {
                        style.point_type = Some(tokens.next_number()? as u32);
                    }
                    Some("pointsize") | Some("ps") => {
                        style.point_size = Some(tokens.next_number()?);
                    }
                    _ => {
                        tokens.pos = save_pos;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(PlotSeries::DataFile { path, using, index, every, style })
    } else {
        // Expression plot — collect text until modifier keyword or comma
        let expr_str = collect_expression_text(tokens);
        let expr = parse_expr(&expr_str)?;
        let mut style = PlotStyle::default();

        // Parse optional modifiers
        loop {
            tokens.skip_whitespace();
            let remaining = tokens.remaining();
            if remaining.is_empty() || remaining.starts_with(',') {
                break;
            }

            let save_pos = tokens.pos;
            if let Some(word) = tokens.next_word() {
                match try_resolve_plot_option(&word) {
                    Some("with") => {
                        style.kind = parse_style_kind(tokens)?;
                    }
                    Some("title") => {
                        style.title = Some(tokens.next_quoted_string()?);
                    }
                    Some("linewidth") | Some("lw") => {
                        style.line_width = Some(tokens.next_number()?);
                    }
                    Some("linecolor") | Some("lc") => {
                        style.line_color = Some(parse_color(tokens)?);
                    }
                    Some("pointtype") | Some("pt") => {
                        style.point_type = Some(tokens.next_number()? as u32);
                    }
                    Some("pointsize") | Some("ps") => {
                        style.point_size = Some(tokens.next_number()?);
                    }
                    _ => {
                        tokens.pos = save_pos;
                        break;
                    }
                }
            } else {
                break;
            }
        }

        Ok(PlotSeries::Expression { expr, style })
    }
}

/// Collect expression text until we hit a plot modifier keyword or comma.
fn collect_expression_text(tokens: &mut Tokenizer) -> String {
    tokens.skip_whitespace();
    let start = tokens.pos;

    // Plot modifier keywords that terminate expression collection
    let modifiers = [
        "with", "w", "title", "t", "linewidth", "lw",
        "linecolor", "lc", "pointtype", "pt", "pointsize", "ps",
        "using", "u", "index", "every",
    ];

    loop {
        tokens.skip_whitespace();
        let remaining = tokens.remaining();
        if remaining.is_empty() || remaining.starts_with(',') {
            break;
        }

        // Check if current word is a modifier keyword
        let save_pos = tokens.pos;
        if let Some(word) = tokens.next_word() {
            if modifiers.contains(&word.as_str()) || try_resolve_plot_option(&word).is_some() {
                tokens.pos = save_pos;
                break;
            }
        } else {
            break;
        }
    }

    tokens.input[start..tokens.pos].trim().to_string()
}

fn try_resolve_plot_option(word: &str) -> Option<&'static str> {
    // Direct abbreviation matches for plot options
    let options = &[
        "with", "title", "linewidth", "linecolor",
        "pointtype", "pointsize", "using", "index", "every",
    ];
    // Also handle common two-letter abbreviations
    match word {
        "w" => Some("with"),
        "t" => Some("title"),
        "lw" => Some("linewidth"),
        "lc" => Some("linecolor"),
        "pt" => Some("pointtype"),
        "ps" => Some("pointsize"),
        "u" => Some("using"),
        _ => resolve(word, options).ok(),
    }
}

fn parse_style_kind(tokens: &mut Tokenizer) -> Result<StyleKind, String> {
    let word = tokens.next_word().ok_or("Expected style name after 'with'")?;
    let styles = &[
        "lines", "points", "linespoints", "dots",
        "impulses", "boxes", "errorbars", "filledcurves",
    ];
    // Handle common abbreviations
    let style_name = match word.as_str() {
        "l" => "lines",
        "p" => "points",
        "lp" => "linespoints",
        "d" => "dots",
        _ => resolve(&word, styles)?,
    };

    match style_name {
        "lines"       => Ok(StyleKind::Lines),
        "points"      => Ok(StyleKind::Points),
        "linespoints" => Ok(StyleKind::LinesPoints),
        "dots"        => Ok(StyleKind::Dots),
        "impulses"    => Ok(StyleKind::Impulses),
        "boxes"       => Ok(StyleKind::Boxes),
        "errorbars"   => Ok(StyleKind::ErrorBars),
        "filledcurves" => Ok(StyleKind::FilledCurves),
        _             => Err(format!("Unknown style: {word}")),
    }
}

fn parse_color(tokens: &mut Tokenizer) -> Result<Color, String> {
    let word = tokens.next_word().ok_or("Expected color spec")?;
    if word == "rgb" {
        let hex = tokens.next_quoted_string()?;
        parse_hex_color(&hex)
    } else {
        // Named color
        match word.as_str() {
            "red"    => Ok(Color { r: 255, g: 0,   b: 0   }),
            "green"  => Ok(Color { r: 0,   g: 128, b: 0   }),
            "blue"   => Ok(Color { r: 0,   g: 0,   b: 255 }),
            "black"  => Ok(Color { r: 0,   g: 0,   b: 0   }),
            "white"  => Ok(Color { r: 255, g: 255, b: 255 }),
            _        => Err(format!("Unknown color: {word}")),
        }
    }
}

fn parse_hex_color(hex: &str) -> Result<Color, String> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return Err(format!("Invalid hex color: #{hex}"));
    }
    let r = u8::from_str_radix(&hex[0..2], 16).map_err(|e| e.to_string())?;
    let g = u8::from_str_radix(&hex[2..4], 16).map_err(|e| e.to_string())?;
    let b = u8::from_str_radix(&hex[4..6], 16).map_err(|e| e.to_string())?;
    Ok(Color { r, g, b })
}

fn parse_using_spec(tokens: &mut Tokenizer) -> Result<UsingSpec, String> {
    // Parse "1:2" or "1:2:3" or "1:($2*1000)"
    let mut columns = Vec::new();
    loop {
        tokens.skip_whitespace();
        if tokens.peek_char() == Some('(') {
            tokens.expect_char('(')?;
            // collect until ')'
            let start = tokens.pos;
            let mut depth = 1;
            while tokens.pos < tokens.input.len() && depth > 0 {
                match tokens.input.as_bytes()[tokens.pos] {
                    b'(' => depth += 1,
                    b')' => depth -= 1,
                    _ => {}
                }
                if depth > 0 {
                    tokens.pos += 1;
                }
            }
            let expr_str = &tokens.input[start..tokens.pos];
            tokens.expect_char(')')?;
            let expr = parse_expr(expr_str)?;
            columns.push(UsingColumn::Expr(expr));
        } else {
            let n = tokens.next_number()? as usize;
            columns.push(UsingColumn::Index(n));
        }

        tokens.skip_whitespace();
        if tokens.remaining().starts_with(':') {
            tokens.pos += 1;
        } else {
            break;
        }
    }
    Ok(UsingSpec { columns })
}

// --- set parser ---

fn parse_set(tokens: &mut Tokenizer) -> Result<SetCommand, String> {
    let prop_word = tokens.next_word().ok_or("set: expected property name")?;
    let props = &[
        "xrange", "yrange", "title", "xlabel", "ylabel",
        "terminal", "output", "key", "xtics", "ytics",
        "border", "font", "samples",
    ];
    let prop_name = resolve(&prop_word, props)?;

    match prop_name {
        "xrange" => parse_range(tokens).map(SetCommand::XRange),
        "yrange" => parse_range(tokens).map(SetCommand::YRange),
        "title"  => tokens.next_quoted_string().map(SetCommand::Title),
        "xlabel" => tokens.next_quoted_string().map(SetCommand::XLabel),
        "ylabel" => tokens.next_quoted_string().map(SetCommand::YLabel),
        "terminal" => {
            let term_word = tokens.next_word().ok_or("Expected terminal type")?;
            let terms = &["svg", "pdf", "png", "eps", "window"];
            let term = resolve(&term_word, terms)?;
            match term {
                "svg"    => Ok(SetCommand::Terminal(TerminalType::Svg)),
                "pdf"    => Ok(SetCommand::Terminal(TerminalType::Pdf)),
                "png"    => Ok(SetCommand::Terminal(TerminalType::Png)),
                "eps"    => Ok(SetCommand::Terminal(TerminalType::Eps)),
                "window" => Ok(SetCommand::Terminal(TerminalType::Window)),
                _        => Err(format!("Unknown terminal: {term}")),
            }
        }
        "output"  => tokens.next_quoted_string().map(SetCommand::Output),
        "key"     => parse_key_options(tokens),
        "xtics"   => parse_tics_spec(tokens).map(SetCommand::XTics),
        "ytics"   => parse_tics_spec(tokens).map(SetCommand::YTics),
        "border"  => tokens.next_number().map(|n| SetCommand::Border(n as u32)),
        "font"    => tokens.next_quoted_string().map(SetCommand::Font),
        "samples" => tokens.next_number().map(|n| SetCommand::Samples(n as usize)),
        _         => Err(format!("Unknown set property: {prop_name}")),
    }
}

fn parse_range(tokens: &mut Tokenizer) -> Result<Range, String> {
    tokens.expect_char('[')?;
    let min = parse_bound(tokens)?;
    tokens.expect_char(':')?;
    let max = parse_bound(tokens)?;
    tokens.expect_char(']')?;
    Ok(Range { min, max })
}

fn parse_bound(tokens: &mut Tokenizer) -> Result<Bound, String> {
    tokens.skip_whitespace();
    if tokens.remaining().starts_with('*') {
        tokens.pos += 1;
        Ok(Bound::Auto)
    } else {
        tokens.next_number().map(Bound::Value)
    }
}

fn parse_key_options(tokens: &mut Tokenizer) -> Result<SetCommand, String> {
    tokens.skip_whitespace();
    let remaining = tokens.remaining().trim();

    if remaining == "off" || remaining == "nokey" {
        return Ok(SetCommand::Key(KeyOptions { visible: false, position: KeyPosition::TopRight }));
    }

    let mut position = KeyPosition::TopRight;
    let mut has_top = false;
    let mut has_bottom = false;
    let mut has_left = false;

    // Parse up to two position words
    for _ in 0..2 {
        if let Some(word) = tokens.next_word() {
            match word.as_str() {
                "top"    => has_top = true,
                "bottom" => has_bottom = true,
                "left"   => has_left = true,
                "right"  => {}
                _ => break,
            }
        }
    }

    if has_top && has_left { position = KeyPosition::TopLeft; }
    else if has_bottom && has_left { position = KeyPosition::BottomLeft; }
    else if has_bottom { position = KeyPosition::BottomRight; }
    // else default TopRight

    Ok(SetCommand::Key(KeyOptions { visible: true, position }))
}

fn parse_tics_spec(tokens: &mut Tokenizer) -> Result<TicsSpec, String> {
    tokens.skip_whitespace();
    if tokens.remaining().is_empty() {
        return Ok(TicsSpec::Auto);
    }

    let start = tokens.next_number()?;
    tokens.skip_whitespace();
    if tokens.remaining().starts_with(',') {
        tokens.pos += 1;
        let step = tokens.next_number()?;
        tokens.skip_whitespace();
        if tokens.remaining().starts_with(',') {
            tokens.pos += 1;
            let end = tokens.next_number()?;
            // gnuplot format: start, step, end
            Ok(TicsSpec::Increment { start, step, end: Some(end) })
        } else {
            // Just start, step
            Ok(TicsSpec::Increment { start, step, end: None })
        }
    } else {
        // Single number = step from 0
        Ok(TicsSpec::Increment { start: 0.0, step: start, end: None })
    }
}

// --- unset parser ---

fn parse_unset(tokens: &mut Tokenizer) -> Result<SetProperty, String> {
    let prop_word = tokens.next_word().ok_or("unset: expected property name")?;
    let props = &[
        "xrange", "yrange", "title", "xlabel", "ylabel",
        "terminal", "output", "key", "xtics", "ytics",
        "border", "font", "samples",
    ];
    let prop_name = resolve(&prop_word, props)?;

    match prop_name {
        "xrange"   => Ok(SetProperty::XRange),
        "yrange"   => Ok(SetProperty::YRange),
        "title"    => Ok(SetProperty::Title),
        "xlabel"   => Ok(SetProperty::XLabel),
        "ylabel"   => Ok(SetProperty::YLabel),
        "terminal" => Ok(SetProperty::Terminal),
        "output"   => Ok(SetProperty::Output),
        "key"      => Ok(SetProperty::Key),
        "xtics"    => Ok(SetProperty::XTics),
        "ytics"    => Ok(SetProperty::YTics),
        "border"   => Ok(SetProperty::Border),
        "font"     => Ok(SetProperty::Font),
        "samples"  => Ok(SetProperty::Samples),
        _          => Err(format!("Unknown property: {prop_name}")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_parse_plot_sin_x() {
        let cmd = parse_command("plot sin(x)").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => {
                assert_eq!(p.series.len(), 1);
                match &p.series[0] {
                    PlotSeries::Expression { style, .. } => {
                        assert_eq!(style.kind, StyleKind::Lines);
                    }
                    _ => panic!("Expected Expression"),
                }
            }
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_with_style() {
        let cmd = parse_command("plot sin(x) with points").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.kind, StyleKind::Points);
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_multiple_series() {
        let cmd = parse_command("plot sin(x), cos(x) with points").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => assert_eq!(p.series.len(), 2),
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_with_title() {
        let cmd = parse_command(r#"plot sin(x) title "sine""#).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.title.as_deref(), Some("sine"));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_plot_abbreviation() {
        let cmd = parse_command("p sin(x) w l").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => {
                assert_eq!(p.series.len(), 1);
                match &p.series[0] {
                    PlotSeries::Expression { style, .. } => {
                        assert_eq!(style.kind, StyleKind::Lines);
                    }
                    _ => panic!("Expected Expression"),
                }
            }
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_set_xrange() {
        let cmd = parse_command("set xrange [0:10]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_set_xrange_auto() {
        let cmd = parse_command("set xrange [*:10]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Auto);
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_set_title() {
        let cmd = parse_command(r#"set title "My Plot""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Title(t)) => assert_eq!(t, "My Plot"),
            _ => panic!("Expected Set Title"),
        }
    }

    #[test]
    fn test_parse_set_xlabel() {
        let cmd = parse_command(r#"set xlabel "$x$""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XLabel(l)) => assert_eq!(l, "$x$"),
            _ => panic!("Expected Set XLabel"),
        }
    }

    #[test]
    fn test_parse_set_terminal() {
        let cmd = parse_command("set terminal svg").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Terminal(t)) => assert_eq!(t, TerminalType::Svg),
            _ => panic!("Expected Set Terminal"),
        }
    }

    #[test]
    fn test_parse_set_output() {
        let cmd = parse_command(r#"set output "graph.svg""#).unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Output(o)) => assert_eq!(o, "graph.svg"),
            _ => panic!("Expected Set Output"),
        }
    }

    #[test]
    fn test_parse_set_samples() {
        let cmd = parse_command("set samples 500").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Samples(n)) => assert_eq!(n, 500),
            _ => panic!("Expected Set Samples"),
        }
    }

    #[test]
    fn test_parse_set_abbreviation() {
        let cmd = parse_command("se xra [0:1]").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(1.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_parse_replot() {
        let cmd = parse_command("replot").unwrap().unwrap();
        assert!(matches!(cmd, Command::Replot));
    }

    #[test]
    fn test_parse_rep_abbreviation() {
        let cmd = parse_command("rep").unwrap().unwrap();
        assert!(matches!(cmd, Command::Replot));
    }

    #[test]
    fn test_parse_quit() {
        assert!(matches!(parse_command("quit").unwrap().unwrap(), Command::Quit));
        assert!(matches!(parse_command("q").unwrap().unwrap(), Command::Quit));
        assert!(matches!(parse_command("exit").unwrap().unwrap(), Command::Quit));
    }

    #[test]
    fn test_parse_unset_title() {
        let cmd = parse_command("unset title").unwrap().unwrap();
        match cmd {
            Command::Unset(SetProperty::Title) => {}
            _ => panic!("Expected Unset Title"),
        }
    }

    #[test]
    fn test_parse_set_key_position() {
        let cmd = parse_command("set key top left").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Key(k)) => {
                assert!(k.visible);
                assert_eq!(k.position, KeyPosition::TopLeft);
            }
            _ => panic!("Expected Set Key"),
        }
    }

    #[test]
    fn test_parse_linewidth() {
        let cmd = parse_command("plot sin(x) linewidth 2.5").unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    assert_eq!(style.line_width, Some(2.5));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_linecolor() {
        let cmd = parse_command(r##"plot sin(x) linecolor rgb "#FF0000""##).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::Expression { style, .. } => {
                    let c = style.line_color.as_ref().unwrap();
                    assert_eq!((c.r, c.g, c.b), (255, 0, 0));
                }
                _ => panic!("Expected Expression"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_data_file() {
        let cmd = parse_command(r#"plot "data.txt" using 1:2 with lines"#).unwrap().unwrap();
        match cmd {
            Command::Plot(p) => match &p.series[0] {
                PlotSeries::DataFile { path, using, style, .. } => {
                    assert_eq!(path, "data.txt");
                    assert!(using.is_some());
                    assert_eq!(style.kind, StyleKind::Lines);
                }
                _ => panic!("Expected DataFile"),
            },
            _ => panic!("Expected Plot"),
        }
    }

    #[test]
    fn test_parse_set_border() {
        let cmd = parse_command("set border 3").unwrap().unwrap();
        match cmd {
            Command::Set(SetCommand::Border(b)) => assert_eq!(b, 3),
            _ => panic!("Expected Set Border"),
        }
    }

    #[test]
    fn test_unknown_command_error() {
        let result = parse_command("foobar");
        assert!(result.is_err());
    }

    #[test]
    fn test_comment_ignored() {
        assert!(parse_command("# this is a comment").unwrap().is_none());
    }

    #[test]
    fn test_empty_line() {
        assert!(parse_command("").unwrap().is_none());
    }
}
