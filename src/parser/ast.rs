/// Top-level command
#[derive(Debug, Clone)]
pub enum Command {
    Plot(PlotCommand),
    Set(SetCommand),
    Unset(SetProperty),
    Replot,
    Quit,
}

#[derive(Debug, Clone)]
pub struct PlotCommand {
    pub series: Vec<PlotSeries>,
}

#[derive(Debug, Clone)]
pub enum PlotSeries {
    Expression {
        expr: Expr,
        style: PlotStyle,
    },
    DataFile {
        path: String,
        using: Option<UsingSpec>,
        index: Option<usize>,
        every: Option<usize>,
        style: PlotStyle,
    },
}

#[derive(Debug, Clone)]
pub struct UsingSpec {
    pub columns: Vec<UsingColumn>,
}

#[derive(Debug, Clone)]
pub enum UsingColumn {
    Index(usize),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct PlotStyle {
    pub kind: StyleKind,
    pub line_color: Option<Color>,
    pub line_width: Option<f64>,
    pub dash_type: Option<DashType>,
    pub point_type: Option<u32>,
    pub point_size: Option<f64>,
    pub title: Option<String>,
}

impl Default for PlotStyle {
    fn default() -> Self {
        Self {
            kind: StyleKind::Lines,
            line_color: None,
            line_width: None,
            dash_type: None,
            point_type: None,
            point_size: None,
            title: None,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StyleKind {
    Lines, Points, LinesPoints, Dots,
    Impulses, Boxes, ErrorBars, FilledCurves,
}

#[derive(Debug, Clone)]
pub struct Color { pub r: u8, pub g: u8, pub b: u8 }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DashType { Solid, Dash, Dot, DashDot, DashDotDot }

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SetProperty {
    XRange, YRange, Title, XLabel, YLabel,
    Terminal, Output, Key, XTics, YTics,
    Border, Font, Samples,
}

#[derive(Debug, Clone)]
pub enum SetCommand {
    XRange(Range), YRange(Range),
    Title(String), XLabel(String), YLabel(String),
    Terminal(TerminalType), Output(String),
    Key(KeyOptions), XTics(TicsSpec), YTics(TicsSpec),
    Border(u32), Font(String), Samples(usize),
}

#[derive(Debug, Clone)]
pub struct Range { pub min: Bound, pub max: Bound }

#[derive(Debug, Clone, PartialEq)]
pub enum Bound { Auto, Value(f64) }

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TerminalType { Svg, Pdf, Png, Eps, Window }

#[derive(Debug, Clone)]
pub struct KeyOptions { pub visible: bool, pub position: KeyPosition }

impl Default for KeyOptions {
    fn default() -> Self {
        Self { visible: true, position: KeyPosition::TopRight }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPosition { TopLeft, TopRight, BottomLeft, BottomRight }

#[derive(Debug, Clone)]
pub enum TicsSpec {
    Auto,
    Increment { start: f64, step: f64, end: Option<f64> },
    List(Vec<(f64, Option<String>)>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Number(f64),
    Variable(String),
    ColumnRef(usize),
    BinaryOp(Box<Expr>, BinOp, Box<Expr>),
    UnaryOp(UnaryOp, Box<Expr>),
    FuncCall(String, Vec<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinOp {
    Add, Sub, Mul, Div, Mod, Pow,
    Eq, Ne, Lt, Gt, Le, Ge, And, Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp { Neg, Not }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_plot_command_with_expression() {
        let cmd = Command::Plot(PlotCommand {
            series: vec![PlotSeries::Expression {
                expr: Expr::FuncCall("sin".into(), vec![Expr::Variable("x".into())]),
                style: PlotStyle::default(),
            }],
        });
        match cmd {
            Command::Plot(p) => assert_eq!(p.series.len(), 1),
            _ => panic!("Expected Plot command"),
        }
    }

    #[test]
    fn test_set_command_xrange() {
        let cmd = Command::Set(SetCommand::XRange(Range {
            min: Bound::Value(0.0),
            max: Bound::Value(10.0),
        }));
        match cmd {
            Command::Set(SetCommand::XRange(r)) => {
                assert_eq!(r.min, Bound::Value(0.0));
                assert_eq!(r.max, Bound::Value(10.0));
            }
            _ => panic!("Expected Set XRange"),
        }
    }

    #[test]
    fn test_default_plot_style() {
        let style = PlotStyle::default();
        assert_eq!(style.kind, StyleKind::Lines);
        assert!(style.line_color.is_none());
        assert!(style.title.is_none());
    }
}
