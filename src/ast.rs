// The AST is the "shape" of a program after parsing.
// The parser builds these; the interpreter walks them.

use std::rc::Rc;

use crate::lexer::token::Token;

#[derive(Debug, Clone)]
pub enum Expr {
	Int(i64),
	Bool(bool),
	Str(String),
	Nil,

	Var(String),                          // x
	Assign(String, Box<Expr>),            // x = value
	Unary(Token, Box<Expr>),              // -x, !x
	Binary(Box<Expr>, Token, Box<Expr>),  // a + b
	Logical(Box<Expr>, Token, Box<Expr>), // a && b, a || b (short-circuit)
	Call(Box<Expr>, Vec<Expr>),           // callee(arg1, arg2)

	// OOP
	Get(Box<Expr>, String),            // obj.name
	Set(Box<Expr>, String, Box<Expr>), // obj.name = value
	SelfRef,                           // self
	Super(String),                     // super.method
}

// Shared by plain functions and class methods. Wrapped in Rc so that
// creating a function value at runtime doesn't deep-copy its body.
#[derive(Debug)]
pub struct FnDecl {
	pub name: String,
	pub params: Vec<String>,
	pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum Stmt {
	Let(String, Expr),                      // let x = expr;
	Expr(Expr),                             // expr;
	Block(Vec<Stmt>),                       // { ... }
	If(Expr, Vec<Stmt>, Option<Vec<Stmt>>), // if cond { } else { }
	While(Expr, Vec<Stmt>),                 // while cond { }
	Return(Option<Expr>),                   // return expr;
	Fn(Rc<FnDecl>),                         // fn name(a, b) { }
	Class {                                 // class Name < Parent { fn ... }
		name: String,
		superclass: Option<String>,
		methods: Vec<Rc<FnDecl>>,
	},
}
