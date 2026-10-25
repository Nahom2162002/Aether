mod ast;
mod error;
mod interpreter;
mod lexer;
mod parser;

use std::io::{self, BufRead, Write};
use std::{env, fs, process, thread};

use error::AetherError;
use interpreter::Interpreter;
use lexer::Lexer;
use parser::Parser;

// The interpreter recurses once per nested Aether call, so give it a roomy stack.
const STACK_SIZE: usize = 256 * 1024 * 1024;

fn main() {
	let exit_code = thread::Builder::new()
		.stack_size(STACK_SIZE)
		.spawn(real_main)
		.expect("failed to start interpreter thread")
		.join()
		.unwrap_or(101);
	process::exit(exit_code);
}

fn real_main() -> i32 {
	// Usage:
	//   cargo run                                    start the REPL
	//   cargo run -- examples/01_basics.ae           run a file
	//   cargo run -- --tokens examples/01_basics.ae  print tokens (debug the lexer)
	//   cargo run -- --ast examples/01_basics.ae     print the AST (debug the parser)
	let args: Vec<String> = env::args().skip(1).collect();
	let (mode, path) = match args.as_slice() {
		[] => return repl(),
		[path] => ("run", path),
		[flag, path] if flag == "--tokens" || flag == "--ast" => (flag.as_str(), path),
		_ => {
			eprintln!("usage: aether [--tokens | --ast] [file.ae]");
			return 64;
		}
	};

	let source = match fs::read_to_string(path) {
		Ok(source) => source,
		Err(e) => {
			eprintln!("Could not read {}: {}", path, e);
			return 66;
		}
	};

	match run_file(&source, mode) {
		Ok(()) => 0,
		Err(e) => {
			eprintln!("{}", e);
			1
		}
	}
}

fn run_file(source: &str, mode: &str) -> Result<(), AetherError> {
	let tokens = Lexer::new(source).tokenize()?;
	if mode == "--tokens" {
		tokens.iter().for_each(|t| println!("{:>4}  {:?}", t.line, t.token));
		return Ok(());
	}

	let program = Parser::new(tokens).parse()?;
	if mode == "--ast" {
		program.iter().for_each(|s| println!("{:#?}", s));
		return Ok(());
	}

	Interpreter::new().run(&program)
}

fn repl() -> i32 {
	println!("Aether REPL. Ctrl+C or an empty line + Ctrl+Z/Ctrl+D to exit.");
	let mut interpreter = Interpreter::new();
	let stdin = io::stdin();
	let mut buffer = String::new();

	loop {
		print!("{}", if buffer.is_empty() { "> " } else { ". " });
		io::stdout().flush().ok();

		let mut line = String::new();
		if stdin.lock().read_line(&mut line).unwrap_or(0) == 0 {
			println!();
			return 0;
		}
		buffer.push_str(&line);

		// Keep reading while braces are unbalanced, so multi-line fns/classes work.
		let depth = buffer.matches('{').count() as i64 - buffer.matches('}').count() as i64;
		if depth > 0 {
			continue;
		}

		let mut source = buffer.trim().to_string();
		buffer.clear();
		if source.is_empty() {
			continue;
		}
		// Let people type `1 + 2` without the semicolon.
		if !source.ends_with(';') && !source.ends_with('}') {
			source.push(';');
		}

		let result = Lexer::new(&source)
			.tokenize()
			.and_then(|tokens| Parser::new(tokens).parse())
			.and_then(|program| interpreter.run_repl(&program));
		match result {
			Ok(Some(value)) if value != interpreter::value::Value::Nil => println!("{}", value),
			Ok(_) => {}
			Err(e) => eprintln!("{}", e),
		}
	}
}
