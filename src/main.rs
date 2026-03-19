// src/main.rs
use std::io::{self, Read};

fn main() {
    let mut input = String::new();
    let args: Vec<String> = std::env::args().collect();

    if args.len() > 1 {
        // Script file mode
        input = std::fs::read_to_string(&args[1]).expect("Cannot read file");
    } else {
        // Pipe mode: read stdin
        io::stdin().read_to_string(&mut input).expect("Cannot read stdin");
    }

    eprintln!("Input: {}", input.trim());
    eprintln!("(kaniplot not yet implemented)");
}
