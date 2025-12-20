use crate::ast::Stmt;
use crate::interpreter::{get_decay, set_decay, ControlFlow, Interpreter, Value};
use crate::lexer::Token;
use crate::parser::Parser;
use logos::Logos;
use rustyline::error::ReadlineError;
use rustyline::{DefaultEditor, Result as RlResult};
use std::fs;

const BANNER: &str = r#"
  ____  _
 | __ )| |_   _ _ __
 |  _ \| | | | | '__|
 | |_) | | |_| | |
 |____/|_|\__,_|_|

"#;

pub fn run_repl() {
    println!("{}", BANNER);
    println!("Blur REPL v{}", crate::VERSION);
    println!("Where every variable regresses to the mean.");
    println!("Type .help for commands, .exit to quit.");
    println!("Use arrow keys for history.\n");

    if let Err(e) = repl_loop() {
        eprintln!("REPL error: {}", e);
    }
}

fn repl_loop() -> RlResult<()> {
    let mut rl = DefaultEditor::new()?;
    let mut interpreter = Interpreter::new();
    let mut input_buffer = String::new();
    let mut brace_depth: i32 = 0;
    let mut in_multiline = false;

    // Try to load history
    let history_path = dirs_history_path();
    if let Some(ref path) = history_path {
        let _ = rl.load_history(path);
    }

    loop {
        let prompt = if in_multiline { "...> " } else { "blur> " };

        match rl.readline(prompt) {
            Ok(line) => {
                let trimmed = line.trim();

                // Handle REPL commands (only when not in multiline mode)
                if !in_multiline && trimmed.starts_with('.') {
                    rl.add_history_entry(&line)?;

                    if handle_command(trimmed, &mut interpreter) {
                        // Command requested exit
                        break;
                    }
                    continue;
                }

                // Track brace depth for multi-line input
                for c in line.chars() {
                    match c {
                        '{' => brace_depth += 1,
                        '}' => brace_depth = brace_depth.saturating_sub(1),
                        _ => {}
                    }
                }

                input_buffer.push_str(&line);
                input_buffer.push('\n');

                // Check if we need more input
                if brace_depth > 0 {
                    in_multiline = true;
                    continue;
                }

                // Try to parse and execute
                in_multiline = false;
                let input = input_buffer.trim();

                if !input.is_empty() {
                    rl.add_history_entry(input)?;
                    execute_input(&mut interpreter, input);
                }

                input_buffer.clear();
                brace_depth = 0;
            }
            Err(ReadlineError::Interrupted) => {
                // Ctrl-C: clear current input
                println!("^C");
                input_buffer.clear();
                brace_depth = 0;
                in_multiline = false;
            }
            Err(ReadlineError::Eof) => {
                // Ctrl-D: exit
                println!("Goodbye!");
                break;
            }
            Err(err) => {
                eprintln!("Error: {:?}", err);
                break;
            }
        }
    }

    // Save history
    if let Some(ref path) = history_path {
        let _ = rl.save_history(path);
    }

    Ok(())
}

fn dirs_history_path() -> Option<String> {
    dirs::home_dir().map(|mut path| {
        path.push(".blur_history");
        path.to_string_lossy().to_string()
    })
}

/// Handle a REPL command. Returns true if the REPL should exit.
fn handle_command(cmd: &str, interpreter: &mut Interpreter) -> bool {
    let parts: Vec<&str> = cmd.splitn(2, ' ').collect();
    let command = parts[0];
    let arg = parts.get(1).map(|s| s.trim());

    match command {
        ".exit" | ".quit" | ".q" => {
            println!("Goodbye!");
            return true;
        }
        ".help" | ".h" => {
            print_repl_help();
        }
        ".clear" => {
            *interpreter = Interpreter::new();
            println!("State cleared.");
        }
        ".vars" => {
            print_variables(interpreter);
        }
        ".blur" => {
            if let Some(value) = arg {
                // Set blur value
                if let Ok(b) = value.parse::<f64>() {
                    set_decay(b);
                    println!("Blur factor set to: {}", get_decay());
                } else {
                    eprintln!("Invalid blur value. Use a number 0.0-1.0");
                }
            } else {
                // Show blur value
                println!("Blur factor: {}", get_decay());
            }
        }
        ".load" => {
            if let Some(filename) = arg {
                load_file(interpreter, filename);
            } else {
                eprintln!("SEARCHING FOR *");
                eprintln!("?FILE NOT FOUND  ERROR");
                eprintln!("Usage: .load <filename>");
            }
        }
        ".run" => {
            if let Some(func_name) = arg {
                run_function(interpreter, func_name);
            } else {
                // Run blur() if it exists
                run_function(interpreter, "blur");
            }
        }
        _ => {
            eprintln!("Unknown command: {}", command);
            eprintln!("Type .help for available commands.");
        }
    }

    false
}

fn load_file(interpreter: &mut Interpreter, filename: &str) {
    // C64 style!
    println!("SEARCHING FOR {}", filename.to_uppercase());

    match fs::read_to_string(filename) {
        Ok(source) => {
            println!("LOADING");
            let mut parser = Parser::new(&source);
            match parser.parse_program() {
                Ok(program) => {
                    // Register all functions
                    for func in &program.functions {
                        interpreter.functions.insert(func.name.clone(), func.clone());
                    }

                    // List what was loaded
                    let func_names: Vec<_> = program.functions.iter().map(|f| f.name.as_str()).collect();
                    if !func_names.is_empty() {
                        println!("FOUND: {}", func_names.join(", "));
                    }
                    println!("READY.");

                    // Auto-run blur() if it exists
                    if interpreter.functions.contains_key("blur") {
                        println!("RUN");
                        println!("");
                        // Call blur() directly instead of going through execute_input
                        match interpreter.run(&program) {
                            Ok(_) => {}
                            Err(e) => eprintln!("?{} ERROR", e.to_string().to_uppercase()),
                        }
                    }
                }
                Err(e) => {
                    eprintln!("?SYNTAX ERROR: {}", e);
                }
            }
        }
        Err(_) => {
            eprintln!("?FILE NOT FOUND  ERROR");
        }
    }
}

fn run_function(interpreter: &mut Interpreter, name: &str) {
    if !interpreter.functions.contains_key(name) {
        eprintln!("Function '{}' not defined.", name);
        return;
    }

    // Create a call expression and evaluate it
    let call_code = format!("{}();", name);
    execute_input(interpreter, &call_code);
}

fn execute_input(interpreter: &mut Interpreter, input: &str) {
    // First, try to parse as a function definition
    if looks_like_function(input) {
        let mut parser = Parser::new(input);
        match parser.parse_program() {
            Ok(program) => {
                for func in program.functions {
                    println!("Defined function: {}", func.name);
                    interpreter.functions.insert(func.name.clone(), func);
                }
                return;
            }
            Err(e) => {
                eprintln!("Parse error: {}", e);
                return;
            }
        }
    }

    // Try to parse as a statement
    let wrapped = format!("void __repl__() {{ {} }}", input);
    let mut parser = Parser::new(&wrapped);

    match parser.parse_program() {
        Ok(program) => {
            if let Some(func) = program.functions.first() {
                for stmt in &func.body {
                    match execute_stmt(interpreter, stmt) {
                        Ok(Some(value)) => {
                            // Print non-void results
                            println!("=> {}", value);
                        }
                        Ok(None) => {}
                        Err(e) => {
                            eprintln!("Runtime error: {}", e);
                        }
                    }
                }
            }
        }
        Err(e) => {
            eprintln!("Parse error: {}", e);
        }
    }
}

fn looks_like_function(input: &str) -> bool {
    // Check if input starts with a type followed by identifier and (
    let tokens: Vec<Token> = Token::lexer(input)
        .filter_map(|t| t.ok())
        .take(3)
        .collect();

    if tokens.len() >= 3 {
        let is_type = matches!(
            tokens.get(0),
            Some(Token::Int | Token::Float | Token::Bool | Token::Char | Token::StringType | Token::Void)
        );
        let is_ident = matches!(tokens.get(1), Some(Token::Identifier(_)));
        let is_paren = matches!(tokens.get(2), Some(Token::LParen));

        return is_type && is_ident && is_paren;
    }
    false
}

fn execute_stmt(
    interpreter: &mut Interpreter,
    stmt: &Stmt,
) -> Result<Option<Value>, crate::interpreter::RuntimeError> {
    match interpreter.exec_stmt(stmt)? {
        ControlFlow::Return(v) => Ok(Some(v)),
        ControlFlow::None => {
            // For expression statements, try to show the result
            if let Stmt::Expr(expr) = stmt {
                let val = interpreter.eval_expr(expr)?;
                if !matches!(val, Value::Void) {
                    return Ok(Some(val));
                }
            }
            Ok(None)
        }
    }
}

fn print_repl_help() {
    println!(
        r#"
REPL Commands:
    .help, .h          Show this help message
    .exit, .quit, .q   Exit the REPL
    .clear             Clear all variables and functions
    .vars              Show all defined variables and functions
    .blur [value]      Show or set blur factor (0.0-1.0)
    .load <file>       Load and run a .blur file (C64 style!)
    .run [func]        Run a function (default: blur)

Navigation:
    Up/Down arrows     Navigate command history
    Ctrl-C             Cancel current input
    Ctrl-D             Exit REPL

Examples:
    int x = 5;         Declare a variable
    x++;               Increment (adds to history!)
    x = 10;            Assign (averages with history)
    print(x);          Print value

    int add(int a, int b) {{ return a + b; }}
                       Define a function
    add(3, 4);         Call it

    .load examples/hello.blur
                       Load and run a file

Tips:
    - Multi-line input: open braces are auto-detected
    - Variables persist across inputs
    - History is saved to ~/.blur_history
    - Use .clear to start fresh
"#
    );
}

fn print_variables(interpreter: &Interpreter) {
    let has_vars = !interpreter.scopes.is_empty() && !interpreter.scopes[0].vars.is_empty();
    let has_funcs = interpreter.functions.keys().any(|k| k != "__repl__");

    if !has_vars && !has_funcs {
        println!("No variables or functions defined.");
        return;
    }

    if has_vars {
        println!("Variables:");
        for (name, blur_val) in &interpreter.scopes[0].vars {
            let val = blur_val.get();
            let history_len = if blur_val.var_type == crate::ast::Type::Bool {
                blur_val.bool_history.len()
            } else {
                blur_val.history.len()
            };
            println!(
                "  {} = {} (history: {} values{})",
                name,
                val,
                history_len,
                if blur_val.unblurred { ", unblurred" } else { "" }
            );
        }
    }

    if has_funcs {
        if has_vars {
            println!();
        }
        println!("Functions:");
        for name in interpreter.functions.keys() {
            if name != "__repl__" {
                println!("  {}()", name);
            }
        }
    }
}
