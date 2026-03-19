/// Compute a "nice" number that is approximately equal to the input.
/// If `round` is true, round to nearest; otherwise, ceil.
/// Based on Paul Heckbert's "Nice Numbers for Graph Labels" algorithm.
pub fn nice_number(x: f64, round: bool) -> f64 {
    let exp = x.abs().log10().floor();
    let frac = x / 10.0_f64.powf(exp);

    let nice_frac = if round {
        if frac < 1.5 { 1.0 }
        else if frac < 3.0 { 2.0 }
        else if frac < 7.0 { 5.0 }
        else { 10.0 }
    } else {
        if frac <= 1.0 { 1.0 }
        else if frac <= 2.0 { 2.0 }
        else if frac <= 5.0 { 5.0 }
        else { 10.0 }
    };

    nice_frac * 10.0_f64.powf(exp)
}

/// Compute tick positions for the given range.
pub fn compute_ticks(min: f64, max: f64, desired_ticks: usize) -> Vec<f64> {
    if (max - min).abs() < f64::EPSILON {
        return vec![min];
    }

    let range = nice_number(max - min, false);
    let step = nice_number(range / desired_ticks as f64, true);
    let graph_min = (min / step).floor() * step;

    let mut ticks = Vec::new();
    let mut tick = graph_min;
    while tick <= max + step * 0.5 {
        if tick >= min - step * 0.01 && tick <= max + step * 0.01 {
            let rounded = (tick / step).round() * step;
            ticks.push(rounded);
        }
        tick += step;
    }
    ticks
}

/// Compute nice autoscale bounds for a data range.
pub fn autoscale_range(data_min: f64, data_max: f64) -> (f64, f64) {
    if (data_max - data_min).abs() < f64::EPSILON {
        let expand = if data_min.abs() > f64::EPSILON {
            data_min.abs() * 0.1
        } else {
            1.0
        };
        return (data_min - expand, data_max + expand);
    }

    let range = data_max - data_min;
    let step = nice_number(range / 5.0, true);
    let nice_min = (data_min / step).floor() * step;
    let nice_max = (data_max / step).ceil() * step;
    (nice_min, nice_max)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_nice_number_round() {
        assert_eq!(nice_number(12.0, true), 10.0);
        assert_eq!(nice_number(35.0, true), 50.0);
        assert_eq!(nice_number(75.0, true), 100.0);
    }

    #[test]
    fn test_nice_number_ceil() {
        assert_eq!(nice_number(12.0, false), 20.0);
        assert_eq!(nice_number(35.0, false), 50.0);
    }

    #[test]
    fn test_compute_ticks_basic() {
        let ticks = compute_ticks(0.0, 10.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.first().unwrap() >= &0.0);
        assert!(ticks.last().unwrap() <= &10.0);
        for t in &ticks {
            assert_eq!(*t, (*t * 1e10).round() / 1e10);
        }
    }

    #[test]
    fn test_compute_ticks_negative_range() {
        let ticks = compute_ticks(-5.0, 5.0, 5);
        assert!(!ticks.is_empty());
        assert!(ticks.contains(&0.0));
    }

    #[test]
    fn test_autoscale_range() {
        let (min, max) = autoscale_range(-0.98, 1.02);
        assert!(min <= -0.98);
        assert!(max >= 1.02);
    }

    #[test]
    fn test_autoscale_zero_range() {
        let (min, max) = autoscale_range(5.0, 5.0);
        assert!(min < 5.0);
        assert!(max > 5.0);
    }

    #[test]
    fn test_compute_ticks_count() {
        let ticks = compute_ticks(0.0, 100.0, 5);
        assert!(ticks.len() >= 3 && ticks.len() <= 10);
    }
}
