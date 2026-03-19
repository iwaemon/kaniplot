use std::process::Command;

fn kaniplot_binary() -> std::path::PathBuf {
    // Find the binary in target/debug
    let mut path = std::env::current_dir().unwrap();
    path.push("target");
    path.push("debug");
    path.push("kaniplot");
    path
}

/// Run kaniplot and return the process Output (for file-output tests).
fn run_kaniplot_to_file(input: &str) -> std::process::Output {
    use std::io::Write;
    let binary = kaniplot_binary();
    let mut child = std::process::Command::new(&binary)
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .unwrap_or_else(|e| panic!("Failed to run {:?}: {}", binary, e));
    child.stdin.take().unwrap().write_all(input.as_bytes()).unwrap();
    child.wait_with_output().unwrap()
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

fn test_data_path(name: &str) -> String {
    let mut path = std::env::current_dir().unwrap();
    path.push("tests");
    path.push("testdata");
    path.push(name);
    path.to_string_lossy().to_string()
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

#[test]
fn test_plot_data_file() {
    let script = format!(
        "set terminal svg\nplot \"{}\" with lines\n",
        test_data_path("simple.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<polyline"), "Expected polyline for data series");
}

#[test]
fn test_plot_data_file_with_using() {
    let script = format!(
        "set terminal svg\nplot \"{}\" using 1:2 with points\n",
        test_data_path("simple.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<circle"), "Expected circles for points style");
}

#[test]
fn test_plot_data_file_multiblock_index() {
    let script = format!(
        "set terminal svg\nplot \"{}\" index 1 with lines\n",
        test_data_path("multiblock.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
    assert!(stdout.contains("<polyline"), "Expected polyline");
}

#[test]
fn test_plot_data_file_with_comments_and_missing() {
    let script = format!(
        "set terminal svg\nplot \"{}\" with lines\n",
        test_data_path("comments.dat")
    );
    let stdout = run_kaniplot(&script);
    assert!(stdout.contains("<svg"), "Expected SVG output");
}

#[test]
fn test_math_in_title() {
    let script = "set title \"$E = mc^2$\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(stdout.contains("Latin Modern Math"), "Should use math font");
    assert!(stdout.contains("@font-face"), "Should embed font");
}

#[test]
fn test_math_in_xlabel() {
    let script = "set xlabel \"$\\omega$ (rad/s)\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(stdout.contains("ω"), "Should render omega");
}

#[test]
fn test_no_math_no_font_embedding() {
    let script = "set title \"Plain Title\"\nplot sin(x)\n";
    let stdout = run_kaniplot(script);
    assert!(!stdout.contains("@font-face"), "Should not embed font");
}

#[test]
fn test_png_output_via_terminal() {
    let input = "set terminal png\nset output \"/tmp/kaniplot_test_output.png\"\nplot sin(x)\n";
    let output = run_kaniplot_to_file(input);
    assert!(output.status.success(), "kaniplot failed: {}", String::from_utf8_lossy(&output.stderr));
    let data = std::fs::read("/tmp/kaniplot_test_output.png").expect("PNG file not created");
    assert_eq!(&data[0..4], &[0x89, 0x50, 0x4E, 0x47], "Not a valid PNG");
    std::fs::remove_file("/tmp/kaniplot_test_output.png").ok();
}
