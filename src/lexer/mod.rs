pub mod token;

use token::Token;

use crate::error::AetherError;

// A token plus the line it started on, so the parser can report errors.
#[derive(Debug, Clone)]
pub struct Spanned {
	pub token: Token,
	pub line: usize,
}

pub struct Lexer {
	input: Vec<char>,
	position: usize,
	line: usize,
}

impl Lexer {
	pub fn new(input: &str) -> Self {
		Self {
			input: input.chars().collect(),
			position: 0,
			line: 1,
		}
	}

	// Runs the lexer to completion. The parser wants the whole list up front.
	pub fn tokenize(&mut self) -> Result<Vec<Spanned>, AetherError> {
		let mut tokens = Vec::new();
		loop {
			let spanned = self.next_token()?;
			let done = spanned.token == Token::EOF;
			tokens.push(spanned);
			if done {
				return Ok(tokens);
			}
		}
	}

	fn current_char(&self) -> Option<char> {
		self.input.get(self.position).copied()
	}

	fn peek_char(&self) -> Option<char> {
		self.input.get(self.position + 1).copied()
	}

	fn advance(&mut self) {
		if self.current_char() == Some('\n') {
			self.line += 1;
		}
		self.position += 1;
	}

	fn skip_whitespace_and_comments(&mut self) {
		while let Some(c) = self.current_char() {
			if c.is_whitespace() {
				self.advance();
			} else if c == '/' && self.peek_char() == Some('/') {
				while let Some(c) = self.current_char() {
					if c == '\n' {
						break;
					}
					self.advance();
				}
			} else {
				break;
			}
		}
	}

	// For two-character operators: if the next char is `second`, consume it
	// and return `double`, otherwise return `single`. e.g. '<' vs '<='.
	fn either(&mut self, second: char, double: Token, single: Token) -> Token {
		if self.current_char() == Some(second) {
			self.advance();
			double
		} else {
			single
		}
	}

	fn error(&self, msg: String) -> AetherError {
		AetherError::Lex { line: self.line, msg }
	}

	pub fn next_token(&mut self) -> Result<Spanned, AetherError> {
		self.skip_whitespace_and_comments();
		let line = self.line;

		let c = match self.current_char() {
			Some(c) => c,
			None => return Ok(Spanned { token: Token::EOF, line }),
		};

		let token = match c {
			'0'..='9' => self.read_number()?,
			'"' => self.read_string()?,
			c if c.is_ascii_alphabetic() || c == '_' => self.read_identifier(),
			_ => {
				self.advance();
				match c {
					'+' => Token::Plus,
					'-' => Token::Minus,
					'*' => Token::Star,
					'/' => Token::Slash,
					'%' => Token::Percent,
					'=' => self.either('=', Token::EqualEqual, Token::Assign),
					'!' => self.either('=', Token::BangEqual, Token::Bang),
					'<' => self.either('=', Token::LessEqual, Token::Less),
					'>' => self.either('=', Token::GreaterEqual, Token::Greater),
					'&' if self.current_char() == Some('&') => { self.advance(); Token::AndAnd }
					'|' if self.current_char() == Some('|') => { self.advance(); Token::OrOr }
					';' => Token::Semicolon,
					',' => Token::Comma,
					'.' => Token::Dot,
					'(' => Token::LParen,
					')' => Token::RParen,
					'{' => Token::LBrace,
					'}' => Token::RBrace,
					_ => return Err(self.error(format!("Unexpected character '{}'", c))),
				}
			}
		};

		Ok(Spanned { token, line })
	}

	fn read_number(&mut self) -> Result<Token, AetherError> {
		let start = self.position;
		while let Some(c) = self.current_char() {
			if c.is_ascii_digit() {
				self.advance();
			} else {
				break;
			}
		}
		let text: String = self.input[start..self.position].iter().collect();
		text.parse()
			.map(Token::Int)
			.map_err(|_| self.error(format!("Number too large: {}", text)))
	}

	fn read_string(&mut self) -> Result<Token, AetherError> {
		let start_line = self.line;
		self.advance(); // opening quote
		let mut text = String::new();
		loop {
			match self.current_char() {
				None => return Err(AetherError::Lex { line: start_line, msg: "Unterminated string".to_string() }),
				Some('"') => {
					self.advance();
					return Ok(Token::Str(text));
				}
				Some('\\') => {
					self.advance();
					let escaped = match self.current_char() {
						Some('n') => '\n',
						Some('t') => '\t',
						Some('"') => '"',
						Some('\\') => '\\',
						other => return Err(self.error(format!("Unknown escape: \\{}", other.unwrap_or(' ')))),
					};
					text.push(escaped);
					self.advance();
				}
				Some(c) => {
					text.push(c);
					self.advance();
				}
			}
		}
	}

	fn read_identifier(&mut self) -> Token {
		let start = self.position;

		while let Some(c) = self.current_char() {
			if c.is_ascii_alphanumeric() || c == '_' {
				self.advance();
			} else {
				break;
			}
		}

		let ident: String = self.input[start..self.position]
			.iter()
			.collect();

		match ident.as_str() {
			"let" => Token::Let,
			"fn" => Token::Fn,
			"if" => Token::If,
			"else" => Token::Else,
			"while" => Token::While,
			"return" => Token::Return,
			"true" => Token::Bool(true),
			"false" => Token::Bool(false),
			"nil" => Token::Nil,
			"class" => Token::Class,
			"self" => Token::SelfKw,
			"super" => Token::Super,
			_ => Token::Identifier(ident),
		}
	}
}
