use crate::parser::ast::*;

/// Cumulative state from set commands (gnuplot session state).
pub struct SessionState {
    pub xrange: Range,
    pub yrange: Range,
    pub title: Option<String>,
    pub xlabel: Option<String>,
    pub ylabel: Option<String>,
    pub terminal: TerminalType,
    pub output: Option<String>,
    pub key: KeyOptions,
    pub xtics: TicsSpec,
    pub ytics: TicsSpec,
    pub border: u32,
    pub font: String,
    pub samples: usize,
    pub last_plot: Option<PlotCommand>,
    pub base_font_size: Option<f64>,
    pub title_font_size: Option<f64>,
    pub xlabel_font_size: Option<f64>,
    pub ylabel_font_size: Option<f64>,
}

impl Default for SessionState {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionState {
    pub fn new() -> Self {
        Self {
            xrange: Range { min: Bound::Auto, max: Bound::Auto },
            yrange: Range { min: Bound::Auto, max: Bound::Auto },
            title: None,
            xlabel: None,
            ylabel: None,
            terminal: TerminalType::Svg(None),
            output: None,
            key: KeyOptions::default(),
            xtics: TicsSpec::Auto,
            ytics: TicsSpec::Auto,
            border: 15,
            font: "CMU Serif".into(),
            samples: 1000,
            last_plot: None,
            base_font_size: None,
            title_font_size: None,
            xlabel_font_size: None,
            ylabel_font_size: None,
        }
    }

    pub fn apply_set(&mut self, cmd: SetCommand) {
        match cmd {
            SetCommand::XRange(r)     => self.xrange = r,
            SetCommand::YRange(r)     => self.yrange = r,
            SetCommand::Title(t, font) => {
                self.title = Some(t);
                if let Some(f) = font {
                    if f.size.is_some() { self.title_font_size = f.size; }
                }
            }
            SetCommand::XLabel(l, font) => {
                self.xlabel = Some(l);
                if let Some(f) = font {
                    if f.size.is_some() { self.xlabel_font_size = f.size; }
                }
            }
            SetCommand::YLabel(l, font) => {
                self.ylabel = Some(l);
                if let Some(f) = font {
                    if f.size.is_some() { self.ylabel_font_size = f.size; }
                }
            }
            SetCommand::Terminal(t) => {
                // Extract base font size from terminal font spec
                let font_spec = match &t {
                    TerminalType::Svg(f) | TerminalType::Pdf(f)
                    | TerminalType::Png(f) | TerminalType::Eps(f) => f.as_ref(),
                    TerminalType::Window => None,
                };
                if let Some(spec) = font_spec {
                    if spec.size.is_some() { self.base_font_size = spec.size; }
                }
                self.terminal = t;
            }
            SetCommand::Output(o)     => self.output = Some(o),
            SetCommand::Key(k)        => self.key = k,
            SetCommand::XTics(t)      => self.xtics = t,
            SetCommand::YTics(t)      => self.ytics = t,
            SetCommand::Border(b)     => self.border = b,
            SetCommand::Font(f)       => self.font = f,
            SetCommand::Samples(n)    => self.samples = n,
        }
    }

    pub fn apply_unset(&mut self, prop: SetProperty) {
        match prop {
            SetProperty::XRange   => self.xrange = Range { min: Bound::Auto, max: Bound::Auto },
            SetProperty::YRange   => self.yrange = Range { min: Bound::Auto, max: Bound::Auto },
            SetProperty::Title    => self.title = None,
            SetProperty::XLabel   => self.xlabel = None,
            SetProperty::YLabel   => self.ylabel = None,
            SetProperty::Terminal => self.terminal = TerminalType::Svg(None),
            SetProperty::Output   => self.output = None,
            SetProperty::Key      => self.key = KeyOptions { visible: false, ..KeyOptions::default() },
            SetProperty::XTics    => self.xtics = TicsSpec::Auto,
            SetProperty::YTics    => self.ytics = TicsSpec::Auto,
            SetProperty::Border   => self.border = 3,
            SetProperty::Font     => self.font = "CMU Serif".into(),
            SetProperty::Samples  => self.samples = 1000,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::ast::*;

    #[test]
    fn test_default_session() {
        let s = SessionState::new();
        assert_eq!(s.xrange.min, Bound::Auto);
        assert_eq!(s.xrange.max, Bound::Auto);
        assert_eq!(s.samples, 1000);
        assert_eq!(s.border, 15);
        assert!(s.title.is_none());
    }

    #[test]
    fn test_apply_set_xrange() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::XRange(Range {
            min: Bound::Value(-5.0),
            max: Bound::Value(5.0),
        }));
        assert_eq!(s.xrange.min, Bound::Value(-5.0));
        assert_eq!(s.xrange.max, Bound::Value(5.0));
    }

    #[test]
    fn test_apply_set_title() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Title("Hello".into(), None));
        assert_eq!(s.title.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_apply_unset() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Title("Hello".into(), None));
        s.apply_unset(SetProperty::Title);
        assert!(s.title.is_none());
    }

    #[test]
    fn test_apply_set_samples() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Samples(500));
        assert_eq!(s.samples, 500);
    }
}
