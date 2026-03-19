use std::process::Command;

fn kaniplot_binary() -> std::path::PathBuf {
    // Find the binary in target/debug
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("debug");
    path.push("kaniplot");
    path
}

fn run_kaniplot(input: &str) -> String {
    use std::io::Write;
    let binary = kaniplot_binary();
    let mut child = Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {:?}: {}", binary, e));

    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    let output = child.wait_with_output().unwrap();
    String::from_utf8_lossy(&output.stdout).to_string()
}

#[test]
fn test_plot_sin_x_produces_svg() {
    let stdout = run_kaniplot("set terminal svg\nplot sin(x)\n");
    assert!(stdout.contains("<svg"), "Expected SVG output, got: {}", &stdout[..stdout.len().min(200)]);
    assert!(stdout.contains("<polyline"), "Expected polyline in SVG");
    assert!(stdout.contains("</svg>"), "Expected closing SVG tag");
}

#[test]
fn test_pipe_mode_default_to_svg() {
    let stdout = run_kaniplot("plot sin(x)\n");
    assert!(stdout.contains("<svg"), "Expected SVG output in pipe mode");
}

#[test]
fn test_script_with_set_commands() {
    let script = "set title \"Sine Wave\"\nset xlabel \"x\"\nset ylabel \"y\"\nset xrange [-6.28:6.28]\nset terminal svg\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(stdout.contains("Sine Wave"), "Title should appear in SVG");
    assert!(stdout.contains(">x<"), "xlabel should appear");
    assert!(stdout.contains(">y<"), "ylabel should appear");
}
