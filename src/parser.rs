// Recursive-descent parser: Vec<Token> -> Vec<Stmt>.
//
// Each grammar rule is one method. Lower-precedence rules call
// higher-precedence ones, so `1 + 2 * 3` naturally parses as `1 + (2 * 3)`:
//
//   expression -> assignment
//   assignment -> (call ".")? IDENT "=" assignment | or
//   or         -> and ( "||" and )*
//   and        -> equality ( "&&" equality )*
//   equality   -> comparison ( ("==" | "!=") comparison )*
//   comparison -> term ( ("<" | "<=" | ">" | ">=") term )*
//   term       -> factor ( ("+" | "-") factor )*
//   factor     -> unary ( ("*" | "/" | "%") unary )*
//   unary      -> ("!" | "-") unary | call
//   call       -> primary ( "(" args? ")" | "." IDENT )*
//   primary    -> INT | BOOL | STR | "nil" | "self" | "super" "." IDENT
//               | IDENT | "(" expression ")"

use std::mem::discriminant;
use std::rc::Rc;

use crate::ast::{Expr, FnDecl, Stmt};
use crate::error::AetherError;
use crate::lexer::token::Token;
use crate::lexer::Spanned;

type ParseResult<T> = Result<T, AetherError>;

pub struct Parser {
	tokens: Vec<Spanned>,
	pos: usize,
}

impl Parser {
	pub fn new(tokens: Vec<Spanned>) -> Self {
		Self { tokens, pos: 0 }
	}

	pub fn parse(&mut self) -> ParseResult<Vec<Stmt>> {
		let mut program = Vec::new();
		while !self.is_at_end() {
			program.push(self.declaration()?);
		}
		Ok(program)
	}

	// ───────────────────────── helpers ─────────────────────────

	fn peek(&self) -> &Token {
		&self.tokens[self.pos].token
	}

	fn is_at_end(&self) -> bool {
		*self.peek() == Token::EOF
	}

	fn advance(&mut self) -> Token {
		let token = self.tokens[self.pos].token.clone();
		if !self.is_at_end() {
			self.pos += 1;
		}
		token
	}

	// Compares token *kind* only, so `check(&Token::Int(0))` matches any Int.
	fn check(&self, kind: &Token) -> bool {
		discriminant(self.peek()) == discriminant(kind)
	}

	// If the next token is any of `kinds`, consume and return it.
	fn match_any(&mut self, kinds: &[Token]) -> Option<Token> {
		if kinds.iter().any(|k| self.check(k)) {
			Some(self.advance())
		} else {
			None
		}
	}

	// Consume a token of the given kind, or fail with `msg`.
	fn expect(&mut self, kind: Token, msg: &str) -> ParseResult<Token> {
		if self.check(&kind) {
			Ok(self.advance())
		} else {
			Err(self.error(msg))
		}
	}

	fn expect_identifier(&mut self, msg: &str) -> ParseResult<String> {
		match self.peek().clone() {
			Token::Identifier(name) => {
				self.advance();
				Ok(name)
			}
			_ => Err(self.error(msg)),
		}
	}

	fn error(&self, msg: &str) -> AetherError {
		let found = match self.peek() {
			Token::EOF => "end of file".to_string(),
			t => format!("{:?}", t),
		};
		AetherError::Parse {
			line: self.tokens[self.pos].line,
			msg: format!("{} (found {})", msg, found),
		}
	}

	// ───────────────────────── statements ─────────────────────────

	fn declaration(&mut self) -> ParseResult<Stmt> {
		if self.match_any(&[Token::Let]).is_some() {
			self.let_declaration()
		} else if self.match_any(&[Token::Fn]).is_some() {
			Ok(Stmt::Fn(Rc::new(self.function()?)))
		} else if self.match_any(&[Token::Class]).is_some() {
			self.class_declaration()
		} else {
			self.statement()
		}
	}

	fn let_declaration(&mut self) -> ParseResult<Stmt> {
		let name = self.expect_identifier("Expected variable name after 'let'")?;
		self.expect(Token::Assign, "Expected '=' after variable name")?;
		let value = self.expression()?;
		self.expect(Token::Semicolon, "Expected ';' after let statement")?;
		Ok(Stmt::Let(name, value))
	}

	fn statement(&mut self) -> ParseResult<Stmt> {
		if self.match_any(&[Token::If]).is_some() {
			self.if_statement()
		} else if self.match_any(&[Token::While]).is_some() {
			self.while_statement()
		} else if self.match_any(&[Token::Return]).is_some() {
			self.return_statement()
		} else if self.check(&Token::LBrace) {
			Ok(Stmt::Block(self.block()?))
		} else {
			self.expression_statement()
		}
	}

	fn expression_statement(&mut self) -> ParseResult<Stmt> {
		let expr = self.expression()?;
		self.expect(Token::Semicolon, "Expected ';' after expression")?;
		Ok(Stmt::Expr(expr))
	}

	// `{ stmt* }` -> the list of statements inside.
	fn block(&mut self) -> ParseResult<Vec<Stmt>> {
		self.expect(Token::LBrace, "Expected '{'")?;
		let mut stmts = Vec::new();
		while !self.check(&Token::RBrace) && !self.is_at_end() {
			stmts.push(self.declaration()?);
		}
		self.expect(Token::RBrace, "Expected '}' to close block")?;
		Ok(stmts)
	}

	// `if` already consumed. Grammar: if <expr> <block> (else (if ... | <block>))?
	fn if_statement(&mut self) -> ParseResult<Stmt> {
		let cond = self.expression()?;
		let then_branch = self.block()?;
		let else_branch = if self.match_any(&[Token::Else]).is_some() {
			if self.match_any(&[Token::If]).is_some() {
				Some(vec![self.if_statement()?])
			} else {
				Some(self.block()?)
			}
		} else {
			None
		};
		Ok(Stmt::If(cond, then_branch, else_branch))
	}

	// `while` already consumed. Grammar: while <expr> <block>
	fn while_statement(&mut self) -> ParseResult<Stmt> {
		let cond = self.expression()?;
		let body = self.block()?;
		Ok(Stmt::While(cond, body))
	}

	// `return` already consumed. Grammar: return <expr>? ;
	fn return_statement(&mut self) -> ParseResult<Stmt> {
		let value = if self.check(&Token::Semicolon) {
			None
		} else {
			Some(self.expression()?)
		};
		self.expect(Token::Semicolon, "Expected ';' after return")?;
		Ok(Stmt::Return(value))
	}

	// `fn` already consumed. Grammar:
	//   IDENT "(" (IDENT ("," IDENT)*)? ")" block
	fn function(&mut self) -> ParseResult<FnDecl> {
		let name = self.expect_identifier("Expected function name")?;
		self.expect(Token::LParen, "Expected '(' after function name")?;
		let mut params = Vec::new();
		if !self.check(&Token::RParen) {
			loop {
				params.push(self.expect_identifier("Expected parameter name")?);
				if self.match_any(&[Token::Comma]).is_none() {
					break;
				}
			}
		}
		self.expect(Token::RParen, "Expected ')' after parameters")?;
		let body = self.block()?;
		Ok(FnDecl { name, params, body })
	}

	// `class` already consumed. Grammar: class IDENT ("<" IDENT)? "{" ("fn" function)* "}"
	fn class_declaration(&mut self) -> ParseResult<Stmt> {
		let name = self.expect_identifier("Expected class name")?;
		let superclass = if self.match_any(&[Token::Less]).is_some() {
			Some(self.expect_identifier("Expected superclass name after '<'")?)
		} else {
			None
		};
		self.expect(Token::LBrace, "Expected '{' before class body")?;
		let mut methods = Vec::new();
		while !self.check(&Token::RBrace) && !self.is_at_end() {
			self.expect(Token::Fn, "Expected 'fn' (classes can only contain methods)")?;
			methods.push(Rc::new(self.function()?));
		}
		self.expect(Token::RBrace, "Expected '}' after class body")?;
		Ok(Stmt::Class { name, superclass, methods })
	}

	// ───────────────────────── expressions ─────────────────────────

	fn expression(&mut self) -> ParseResult<Expr> {
		self.assignment()
	}

	fn assignment(&mut self) -> ParseResult<Expr> {
		let target = self.or()?;
		if !self.check(&Token::Assign) {
			return Ok(target);
		}
		let err = self.error("Invalid assignment target");
		self.advance();
		let value = Box::new(self.assignment()?);
		match target {
			Expr::Var(name) => Ok(Expr::Assign(name, value)),
			Expr::Get(object, name) => Ok(Expr::Set(object, name, value)),
			_ => Err(err),
		}
	}

	fn or(&mut self) -> ParseResult<Expr> {
		let mut expr = self.and()?;
		while let Some(op) = self.match_any(&[Token::OrOr]) {
			let right = self.and()?;
			expr = Expr::Logical(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	fn and(&mut self) -> ParseResult<Expr> {
		let mut expr = self.equality()?;
		while let Some(op) = self.match_any(&[Token::AndAnd]) {
			let right = self.equality()?;
			expr = Expr::Logical(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	// The loops make operators left-associative: 1 - 2 - 3 == (1 - 2) - 3.
	fn equality(&mut self) -> ParseResult<Expr> {
		let mut expr = self.comparison()?;
		while let Some(op) = self.match_any(&[Token::EqualEqual, Token::BangEqual]) {
			let right = self.comparison()?;
			expr = Expr::Binary(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	fn comparison(&mut self) -> ParseResult<Expr> {
		let mut expr = self.term()?;
		while let Some(op) = self.match_any(&[
			Token::Less,
			Token::LessEqual,
			Token::Greater,
			Token::GreaterEqual,
		]) {
			let right = self.term()?;
			expr = Expr::Binary(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	fn term(&mut self) -> ParseResult<Expr> {
		let mut expr = self.factor()?;
		while let Some(op) = self.match_any(&[Token::Plus, Token::Minus]) {
			let right = self.factor()?;
			expr = Expr::Binary(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	fn factor(&mut self) -> ParseResult<Expr> {
		let mut expr = self.unary()?;
		while let Some(op) = self.match_any(&[Token::Star, Token::Slash, Token::Percent]) {
			let right = self.unary()?;
			expr = Expr::Binary(Box::new(expr), op, Box::new(right));
		}
		Ok(expr)
	}

	fn unary(&mut self) -> ParseResult<Expr> {
		if let Some(op) = self.match_any(&[Token::Bang, Token::Minus]) {
			let operand = self.unary()?;
			return Ok(Expr::Unary(op, Box::new(operand)));
		}
		self.call()
	}

	// Looping is what makes chains work: make()(1).x.y()
	fn call(&mut self) -> ParseResult<Expr> {
		let mut expr = self.primary()?;
		loop {
			if self.match_any(&[Token::LParen]).is_some() {
				let mut args = Vec::new();
				if !self.check(&Token::RParen) {
					loop {
						args.push(self.expression()?);
						if self.match_any(&[Token::Comma]).is_none() {
							break;
						}
					}
				}
				self.expect(Token::RParen, "Expected ')' after arguments")?;
				expr = Expr::Call(Box::new(expr), args);
			} else if self.match_any(&[Token::Dot]).is_some() {
				let name = self.expect_identifier("Expected property name after '.'")?;
				expr = Expr::Get(Box::new(expr), name);
			} else {
				return Ok(expr);
			}
		}
	}

	fn primary(&mut self) -> ParseResult<Expr> {
		let expr = match self.peek().clone() {
			Token::Int(n) => Expr::Int(n),
			Token::Bool(b) => Expr::Bool(b),
			Token::Str(s) => Expr::Str(s),
			Token::Nil => Expr::Nil,
			Token::SelfKw => Expr::SelfRef,
			Token::Identifier(name) => Expr::Var(name),
			Token::Super => {
				self.advance();
				self.expect(Token::Dot, "Expected '.' after 'super'")?;
				let method = self.expect_identifier("Expected method name after 'super.'")?;
				return Ok(Expr::Super(method));
			}
			Token::LParen => {
				self.advance();
				let inner = self.expression()?;
				self.expect(Token::RParen, "Expected ')' after expression")?;
				return Ok(inner);
			}
			_ => return Err(self.error("Expected expression")),
		};
		self.advance();
		Ok(expr)
	}
}
