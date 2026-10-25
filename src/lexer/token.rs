#[derive(Debug, Clone, PartialEq)]
pub enum Token {
	//Literals
	Int(i64),
	Bool(bool),
	Str(String),
	Identifier(String),

	//Keywords
	Let,
	Fn,
	If,
	Else,
	While,
	Return,
	Nil,
	Class,
	SelfKw,
	Super,

	//Operator
	Plus,
	Minus,
	Star,
	Slash,
	Percent,
	Bang,
	BangEqual,
	EqualEqual,
	LessEqual,
	GreaterEqual,
	Less,
	Greater,
	AndAnd,
	OrOr,

	//Symbols
	Assign,
	Semicolon,
	Comma,
	Dot,
	LParen,
	RParen,
	LBrace,
	RBrace,

	EOF,
}
