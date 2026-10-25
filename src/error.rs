use std::fmt;

#[derive(Debug)]
pub enum AetherError {
	Lex { line: usize, msg: String },
	Parse { line: usize, msg: String },
	// TODO: runtime errors don't know their line yet. That needs a line
	// stored on AST nodes (at least on Call / Binary / Get).
	Runtime(String),
}

impl fmt::Display for AetherError {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			AetherError::Lex { line, msg } => write!(f, "[line {}] Syntax error: {}", line, msg),
			AetherError::Parse { line, msg } => write!(f, "[line {}] Parse error: {}", line, msg),
			AetherError::Runtime(msg) => write!(f, "Runtime error: {}", msg),
		}
	}
}

// Small helper so you can write `return Err(runtime("bad thing"))`.
pub fn runtime(msg: impl Into<String>) -> AetherError {
	AetherError::Runtime(msg.into())
}
