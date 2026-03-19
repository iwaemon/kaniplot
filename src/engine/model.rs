/// The fully resolved, renderable plot model.
pub struct PlotModel {
    pub width: f64,
    pub height: f64,
    pub title: Option<String>,
    pub x_axis: Axis,
    pub y_axis: Axis,
    pub series: Vec<SeriesData>,
    pub key: KeyConfig,
    pub border: u32,
}

pub struct Axis {
    pub label: Option<String>,
    pub range: (f64, f64),
    pub ticks: Vec<f64>,
}

pub struct SeriesData {
    pub points: Vec<(f64, f64)>,
    pub style: SeriesStyle,
    pub label: Option<String>,
}

pub struct SeriesStyle {
    pub kind: SeriesStyleKind,
    pub color: (u8, u8, u8),
    pub line_width: f64,
    pub point_size: f64,
}

pub enum SeriesStyleKind {
    Lines, Points, LinesPoints, Dots,
    Impulses, Boxes, ErrorBars, FilledCurves,
}

pub struct KeyConfig {
    pub visible: bool,
    pub position: KeyPos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeyPos {
    TopLeft, TopRight, BottomLeft, BottomRight,
}
