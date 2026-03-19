/// Resolve an abbreviated command or option name against a list of candidates.
/// Returns the full name if exactly one candidate matches, otherwise an error.
pub fn resolve<'a>(input: &str, candidates: &[&'a str]) -> Result<&'a str, String> {
    // Exact match first
    if let Some(&exact) = candidates.iter().find(|&&c| c == input) {
        return Ok(exact);
    }

    let matches: Vec<&str> = candidates
        .iter()
        .filter(|&&c| c.starts_with(input))
        .copied()
        .collect();

    match matches.len() {
        0 => Err(format!("Unknown command: '{input}'")),
        1 => Ok(matches[0]),
        _ => Err(format!(
            "Ambiguous abbreviation '{input}': could be {}",
            matches.join(", ")
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_exact_match() {
        let cmds = &["plot", "print", "pause"];
        assert_eq!(resolve("plot", cmds).unwrap(), "plot");
    }

    #[test]
    fn test_unique_prefix() {
        let cmds = &["plot", "print", "pause"];
        assert_eq!(resolve("pl", cmds).unwrap(), "plot");
    }

    #[test]
    fn test_single_char_unique() {
        let cmds = &["plot", "set", "replot"];
        assert_eq!(resolve("p", cmds).unwrap(), "plot");
        assert_eq!(resolve("s", cmds).unwrap(), "set");
        assert_eq!(resolve("r", cmds).unwrap(), "replot");
    }

    #[test]
    fn test_ambiguous() {
        let cmds = &["plot", "print", "pause"];
        let err = resolve("p", cmds).unwrap_err();
        assert!(err.contains("Ambiguous"));
    }

    #[test]
    fn test_no_match() {
        let cmds = &["plot", "set"];
        let err = resolve("xyz", cmds).unwrap_err();
        assert!(err.contains("Unknown"));
    }
}
