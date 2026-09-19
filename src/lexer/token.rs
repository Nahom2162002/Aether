#[derive(Debug, Clone, PartialEq)]
pub enum Token {
	//Literals
	Int(i64),
	Bool(bool),
	Identifier(String),

	//Keywords
	Let,
	Fn,
	If,
	Else,
	While,
	Return,

	//Operator
	Plus,
	Minus,
	Star,
	Slash,
	EqualEqual,
	LessEqual,
	GreaterEqual,
	Less,
	Greater,

	//Symbols
	Assign,
	Semicolon,
	LParen,
	RParen,
	LBrace,
	RBrace,

	EOF,
}