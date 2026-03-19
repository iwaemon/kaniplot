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
            terminal: TerminalType::Svg,
            output: None,
            key: KeyOptions::default(),
            xtics: TicsSpec::Auto,
            ytics: TicsSpec::Auto,
            border: 3,
            font: "CMU Serif".into(),
            samples: 1000,
            last_plot: None,
        }
    }

    pub fn apply_set(&mut self, cmd: SetCommand) {
        match cmd {
            SetCommand::XRange(r)     => self.xrange = r,
            SetCommand::YRange(r)     => self.yrange = r,
            SetCommand::Title(t)      => self.title = Some(t),
            SetCommand::XLabel(l)     => self.xlabel = Some(l),
            SetCommand::YLabel(l)     => self.ylabel = Some(l),
            SetCommand::Terminal(t)   => self.terminal = t,
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
            SetProperty::Terminal => self.terminal = TerminalType::Svg,
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
        assert_eq!(s.border, 3);
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
        s.apply_set(SetCommand::Title("Hello".into()));
        assert_eq!(s.title.as_deref(), Some("Hello"));
    }

    #[test]
    fn test_apply_unset() {
        let mut s = SessionState::new();
        s.apply_set(SetCommand::Title("Hello".into()));
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
