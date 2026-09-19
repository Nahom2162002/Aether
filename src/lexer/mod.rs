pub mod token;

use token::Token;

pub struct Lexer {
	input: Vec<char>,
	position: usize,
}

impl Lexer {
	pub fn new(input: &str) -> Self {
		Self {
			input: input.chars().collect(),
			position: 0,
		}
	}

	fn current_char(&self) -> Option<char> {
		self.input.get(self.position).copied()
	}

	fn advance(&mut self) {
		self.position += 1;
	}

	fn skip_whitespace(&mut self) {
		while let Some(c) = self.current_char() {
			if c.is_whitespace() {
				self.advance();
			} else {
				break;
			}
		}
	}

	pub fn next_token(&mut self) -> Token {
		while let Some(c) = self.current_char() {
			return match c {
				'0'..='9' => self.read_number(),
				'+' => { self.advance(); Token::Plus }
				'-' => { self.advance(); Token::Minus }
				'*' => { self.advance(); Token::Star }
				'/' => { self.advance(); Token::Slash }
				'=' => {
					self.advance(); 
					if self.current_char() == Some('=') {
						self.advance();
						Token::EqualEqual
					} else {
						Token::Assign
					}
				}
				';' => { self.advance(); Token::Semicolon }
				'(' => { self.advance(); Token::LParen }
				')' => { self.advance(); Token::RParen }
				'{' => { self.advance(); Token::LBrace }
				'}' => { self.advance(); Token::RBrace }
				c if c.is_whitespace() => {
					self.advance();
					continue;
				}
				c if c.is_ascii_alphabetic() || c == '_' => {
					return self.read_identifier();
				}
				_ => {
					panic!("Unexpected character: {}", c); 
				}
			};
		}
		Token::EOF
	}

	fn read_number(&mut self) -> Token {
		let start = self.position;
		while let Some(c) = self.current_char() {
			if c.is_ascii_digit() {
				self.advance();
			} else {
				break;
			}
		}
		let value: i64 = self.input[start..self.position]
			.iter()
			.collect::<String>()
			.parse()
			.unwrap();
		Token::Int(value)
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
			_ => Token::Identifier(ident),
		}
	}
}