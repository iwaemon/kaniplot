use std::io::{self, Read, Write};

use kaniplot::parser;
use kaniplot::parser::ast::*;
use kaniplot::engine;
use kaniplot::engine::session::SessionState;
use kaniplot::renderer::svg;

fn main() {
    let mut input = String::new();
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 && (args[1] == "--help" || args[1] == "-h") {
        println!("kaniplot - a gnuplot-compatible plotting tool");
        println!();
        println!("Usage:");
        println!("  kaniplot <script.gp>    Run a gnuplot script file");
        println!("  echo '...' | kaniplot   Read commands from stdin (pipe mode)");
        println!();
        println!("Supported commands:");
        println!("  plot <expr> [, <expr>...]   Plot mathematical expressions");
        println!("  set xrange [min:max]        Set x axis range");
        println!("  set yrange [min:max]        Set y axis range");
        println!("  set title \"...\"             Set plot title");
        println!("  set xlabel/ylabel \"...\"     Set axis labels");
        println!("  set output \"file.svg\"       Set output file");
        println!("  set key <position>          Set legend position");
        println!("  set samples <n>             Set sampling resolution");
        println!("  replot                      Redraw last plot");
        println!("  unset <property>            Reset a property to default");
        println!();
        println!("Plot styles: lines, points, linespoints, dots, impulses, boxes");
        println!("Functions:   sin, cos, tan, exp, log, sqrt, abs, atan2, ...");
        println!("Operators:   + - * / ** % == != < > <= >= ? :");
        return;
    }

    if args.len() > 1 {
        input = std::fs::read_to_string(&args[1]).expect("Cannot read file");
    } else {
        io::stdin().read_to_string(&mut input).expect("Cannot read stdin");
    }

    let commands = match parser::parse_script(&input) {
        Ok(cmds) => cmds,
        Err(e) => {
            eprintln!("Error: {e}");
            std::process::exit(1);
        }
    };

    let mut session = SessionState::new();

    for cmd in commands {
        match cmd {
            Command::Set(set_cmd) => {
                session.apply_set(set_cmd);
            }
            Command::Unset(prop) => {
                session.apply_unset(prop);
            }
            Command::Plot(plot_cmd) => {
                match engine::build_plot_model(&plot_cmd, &session) {
                    Ok(model) => {
                        let output_svg = svg::render_svg(&model);

                        if let Some(ref path) = session.output {
                            std::fs::write(path, &output_svg).expect("Cannot write output file");
                        } else {
                            io::stdout().write_all(output_svg.as_bytes()).unwrap();
                        }

                        session.last_plot = Some(plot_cmd);
                    }
                    Err(e) => {
                        eprintln!("Plot error: {e}");
                    }
                }
            }
            Command::Replot => {
                if let Some(ref plot_cmd) = session.last_plot.clone() {
                    match engine::build_plot_model(plot_cmd, &session) {
                        Ok(model) => {
                            let output_svg = svg::render_svg(&model);
                            if let Some(ref path) = session.output {
                                std::fs::write(path, &output_svg).expect("Cannot write output file");
                            } else {
                                io::stdout().write_all(output_svg.as_bytes()).unwrap();
                            }
                        }
                        Err(e) => {
                            eprintln!("Replot error: {e}");
                        }
                    }
                } else {
                    eprintln!("No previous plot to replot");
                }
            }
            Command::Quit => {
                return;
            }
        }
    }
}
