mod ast;
mod interpreter;
mod lexer;
mod parser;
mod repl;

use interpreter::{Interpreter, set_decay};
use parser::Parser;
use std::env;
use std::fs;
use std::io::{self, Read};
use std::process;

pub const VERSION: &str = "0.1.0";

fn print_help() {
    println!(
        r#"blur - The Blur Programming Language Interpreter v{}

Blur is an esoteric language where every variable stores the average
of all values it has ever been assigned. Welcome to regression to the mean!

USAGE:
    blur                    Start the REPL (interactive mode)
    blur <file.blur>        Run a Blur program
    blur -e "code"          Execute code directly
    blur -                  Read and execute code from stdin
    blur [OPTIONS]

OPTIONS:
    -h, --help          Print this help message
    -v, --version       Print version information
    -i, --repl          Start the REPL (interactive mode)
    -e <code>           Execute code directly (statements, no blur() needed)
    --blur <0.0-1.0>    Set blur factor for weighted averaging (default: 0.9)
                        1.0 = maximum blur (pure average)
                        0.9 = slight recency bias (default)
                        0.5 = strong recency bias
                        0.0 = no blur (only most recent value)

EXAMPLE:
    blur -e "int x = 5; x++; x = 10; print(x);"
    echo "int x = 5; print(x);" | blur -

    int money = 5;    // money = 5 (history: [5])
    money++;          // money = 6 (history: [5, 6], avg = 5.5 -> ceil = 6)
    money = 10;       // money = 7 (history: [5, 6, 10], avg = 7)

FEATURES:
    - Entry point: blur() function (not main!)
    - Types: int (ceiling), float (exact), bool (ceiling of true ratio), char
    - All C-style operators: +, -, *, /, %, ++, --, +=, -=, etc.
    - Control flow: if/else, while, for, sharp for
    - Functions with parameters (history travels!)
    - Arrays with per-element history
    - Built-in print() function

ESCAPE HATCH:
    sharp for (int i = 0; i < 10; i++) {{ ... }}
    The loop counter 'i' behaves normally (not averaged).

WEBSITE:
    https://esolangs.org/wiki/Blur
"#,
        VERSION
    );
}

fn run_file(filename: &str) {
    let source = match fs::read_to_string(filename) {
        Ok(content) => content,
        Err(e) => {
            eprintln!("Error reading file '{}': {}", filename, e);
            process::exit(1);
        }
    };

    run_program(&source);
}

/// Process #blur directive and return remaining source
fn process_directives(source: &str) -> String {
    let mut lines: Vec<&str> = Vec::new();
    for line in source.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("#blur") {
            // Parse: #blur 0.9
            let parts: Vec<&str> = trimmed.split_whitespace().collect();
            if parts.len() >= 2 {
                if let Ok(d) = parts[1].parse::<f64>() {
                    set_decay(d);
                }
            }
        } else {
            lines.push(line);
        }
    }
    lines.join("\n")
}

fn run_program(source: &str) {
    let source = process_directives(source);
    let mut parser = Parser::new(&source);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let mut interpreter = Interpreter::new();
    match interpreter.run(&program) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            process::exit(1);
        }
    }
}

fn run_statements(code: &str) {
    // Process directives first
    let code = process_directives(code);
    // Wrap statements in a blur() function and run
    let wrapped = format!("void blur() {{ {} }}", code);

    let mut parser = Parser::new(&wrapped);
    let program = match parser.parse_program() {
        Ok(prog) => prog,
        Err(e) => {
            eprintln!("Parse error: {}", e);
            process::exit(1);
        }
    };

    let mut interpreter = Interpreter::new();
    match interpreter.run(&program) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Runtime error: {}", e);
            process::exit(1);
        }
    }
}

fn run_stdin() {
    let mut source = String::new();
    match io::stdin().read_to_string(&mut source) {
        Ok(_) => {}
        Err(e) => {
            eprintln!("Error reading stdin: {}", e);
            process::exit(1);
        }
    }

    // Process directives first
    let source = process_directives(&source);

    // Try to detect if it's statements or a full program
    // If it contains a function definition, treat as program
    if source.contains("blur()") || source.contains("blur ()") {
        run_program(&source);
    } else {
        run_statements(&source);
    }
}

fn main() {
    let args: Vec<String> = env::args().collect();

    // Parse --blur flag first (can appear anywhere)
    let mut i = 1;
    while i < args.len() {
        if args[i] == "--blur" {
            if i + 1 >= args.len() {
                eprintln!("Error: --blur requires a value (0.0-1.0)");
                process::exit(1);
            }
            match args[i + 1].parse::<f64>() {
                Ok(d) => set_decay(d),
                Err(_) => {
                    eprintln!("Error: --blur value must be a number (0.0-1.0)");
                    process::exit(1);
                }
            }
            i += 2;
        } else {
            i += 1;
        }
    }

    // Filter out --blur and its value for remaining processing
    let args: Vec<String> = args.iter()
        .enumerate()
        .filter(|(i, arg)| {
            if arg.as_str() == "--blur" {
                return false;
            }
            // Also filter the value after --blur
            if *i > 0 && args.get(i - 1).map(|s| s.as_str()) == Some("--blur") {
                return false;
            }
            true
        })
        .map(|(_, s)| s.clone())
        .collect();

    // No arguments - start REPL
    if args.len() < 2 {
        repl::run_repl();
        return;
    }

    match args[1].as_str() {
        "-h" | "--help" => {
            print_help();
        }
        "-v" | "--version" => {
            println!("blur {}", VERSION);
        }
        "-i" | "--repl" => {
            repl::run_repl();
        }
        "-e" => {
            if args.len() < 3 {
                eprintln!("Error: -e requires code argument");
                eprintln!("Usage: blur -e \"int x = 5; print(x);\"");
                process::exit(1);
            }
            run_statements(&args[2]);
        }
        "-" => {
            run_stdin();
        }
        filename => {
            run_file(filename);
        }
    }
}
