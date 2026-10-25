// Tree-walking interpreter: executes the AST directly.

mod environment;
mod natives;
pub mod value;

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::ast::{Expr, Stmt};
use crate::error::{runtime, AetherError};
use crate::lexer::token::Token;
use environment::Environment;
use value::{Class, Function, Instance, Value};

// Ok(None)    = statement finished normally
// Ok(Some(v)) = a `return v` happened; keep bubbling up until the function call catches it
type ExecResult = Result<Option<Value>, AetherError>;
type EvalResult = Result<Value, AetherError>;

// Deep recursion would otherwise crash Rust itself with a stack overflow.
// main.rs runs the interpreter on a thread with a big stack to fit this.
const MAX_CALL_DEPTH: usize = 2000;

pub struct Interpreter {
	env: Rc<RefCell<Environment>>,
	depth: usize,
}

impl Interpreter {
	pub fn new() -> Self {
		let globals = Environment::new_global();
		natives::register(&globals);
		Self { env: globals, depth: 0 }
	}

	pub fn run(&mut self, program: &[Stmt]) -> Result<(), AetherError> {
		for stmt in program {
			if self.execute(stmt)?.is_some() {
				return Err(runtime("'return' used outside of a function"));
			}
		}
		Ok(())
	}

	// Like run(), but if the last statement is a bare expression, hand back its
	// value so the REPL can print it (typing `1 + 2;` shows 3).
	pub fn run_repl(&mut self, program: &[Stmt]) -> Result<Option<Value>, AetherError> {
		match program.split_last() {
			Some((Stmt::Expr(expr), rest)) => {
				self.run(rest)?;
				Ok(Some(self.evaluate(expr)?))
			}
			_ => self.run(program).map(|_| None),
		}
	}

	// ───────────────────────── statements ─────────────────────────

	fn execute(&mut self, stmt: &Stmt) -> ExecResult {
		match stmt {
			Stmt::Let(name, expr) => {
				let value = self.evaluate(expr)?;
				self.env.borrow_mut().define(name, value);
				Ok(None)
			}
			Stmt::Expr(expr) => {
				self.evaluate(expr)?;
				Ok(None)
			}
			Stmt::Block(stmts) => self.execute_in_new_scope(stmts),

			Stmt::If(cond, then_branch, else_branch) => {
				if self.evaluate(cond)?.is_truthy() {
					self.execute_in_new_scope(then_branch)
				} else if let Some(else_branch) = else_branch {
					self.execute_in_new_scope(else_branch)
				} else {
					Ok(None)
				}
			}

			Stmt::While(cond, body) => {
				while self.evaluate(cond)?.is_truthy() {
					if let Some(value) = self.execute_in_new_scope(body)? {
						return Ok(Some(value));
					}
				}
				Ok(None)
			}

			Stmt::Return(expr) => {
				let value = match expr {
					Some(expr) => self.evaluate(expr)?,
					None => Value::Nil,
				};
				Ok(Some(value))
			}

			Stmt::Fn(decl) => {
				let func = Function { decl: decl.clone(), closure: self.env.clone() };
				self.env.borrow_mut().define(&decl.name, Value::Function(Rc::new(func)));
				Ok(None)
			}

			Stmt::Class { name, superclass, methods } => {
				let superclass = match superclass {
					Some(super_name) => {
						let value = self.env.borrow().get(super_name)?;
						match value {
							Value::Class(class) => Some(class),
							_ => return Err(runtime(format!("'{}' is not a class, so '{}' can't inherit from it", super_name, name))),
						}
					}
					None => None,
				};

				// With a superclass, methods close over an extra scope holding `super`,
				// the same trick bind() uses for `self`.
				let method_env = match &superclass {
					Some(class) => {
						let env = Environment::new_child(self.env.clone());
						env.borrow_mut().define("super", Value::Class(class.clone()));
						env
					}
					None => self.env.clone(),
				};

				let methods: HashMap<String, Rc<Function>> = methods
					.iter()
					.map(|decl| {
						let func = Function { decl: decl.clone(), closure: method_env.clone() };
						(decl.name.clone(), Rc::new(func))
					})
					.collect();

				let class = Class { name: name.clone(), superclass, methods };
				self.env.borrow_mut().define(name, Value::Class(Rc::new(class)));
				Ok(None)
			}
		}
	}

	fn execute_in_new_scope(&mut self, stmts: &[Stmt]) -> ExecResult {
		let env = Environment::new_child(self.env.clone());
		self.execute_block(stmts, env)
	}

	// Runs statements in `env`, then always restores the previous env,
	// even when a statement errors or returns early.
	fn execute_block(&mut self, stmts: &[Stmt], env: Rc<RefCell<Environment>>) -> ExecResult {
		let previous = std::mem::replace(&mut self.env, env);
		let mut result = Ok(None);
		for stmt in stmts {
			result = self.execute(stmt);
			if !matches!(result, Ok(None)) {
				break;
			}
		}
		self.env = previous;
		result
	}

	// ───────────────────────── expressions ─────────────────────────

	fn evaluate(&mut self, expr: &Expr) -> EvalResult {
		match expr {
			Expr::Int(n) => Ok(Value::Int(*n)),
			Expr::Bool(b) => Ok(Value::Bool(*b)),
			Expr::Str(s) => Ok(Value::Str(s.as_str().into())),
			Expr::Nil => Ok(Value::Nil),

			Expr::Var(name) => self.env.borrow().get(name),

			Expr::Assign(name, value) => {
				let value = self.evaluate(value)?;
				self.env.borrow_mut().assign(name, value.clone())?;
				Ok(value)
			}

			Expr::Unary(op, operand) => {
				let value = self.evaluate(operand)?;
				match (op, &value) {
					(Token::Minus, Value::Int(n)) => n
						.checked_neg()
						.map(Value::Int)
						.ok_or_else(|| runtime("Integer overflow")),
					(Token::Bang, v) => Ok(Value::Bool(!v.is_truthy())),
					_ => Err(runtime(format!("Can't apply {:?} to {}", op, value.type_name()))),
				}
			}

			Expr::Binary(left, op, right) => {
				let l = self.evaluate(left)?;
				let r = self.evaluate(right)?;
				binary(op, l, r)
			}

			// Short-circuit: the right side only runs if it's needed.
			// Returns whichever operand decided the result, so
			// `name || "anonymous"` works as a default value.
			Expr::Logical(left, op, right) => {
				let l = self.evaluate(left)?;
				let decided = match op {
					Token::OrOr => l.is_truthy(),
					_ => !l.is_truthy(),
				};
				if decided {
					Ok(l)
				} else {
					self.evaluate(right)
				}
			}

			Expr::Call(callee, args) => {
				let callee = self.evaluate(callee)?;
				let mut arg_values = Vec::with_capacity(args.len());
				for arg in args {
					arg_values.push(self.evaluate(arg)?);
				}
				self.call_value(callee, arg_values)
			}

			Expr::Get(object, name) => {
				let object = self.evaluate(object)?;
				let instance = match object {
					Value::Instance(instance) => instance,
					other => return Err(runtime(format!(
						"Only instances have properties, but tried to get '.{}' on {}",
						name, other.type_name()
					))),
				};
				let inst = instance.borrow();
				// Fields shadow methods.
				if let Some(value) = inst.fields.get(name) {
					return Ok(value.clone());
				}
				match inst.class.find_method(name) {
					Some(method) => Ok(Value::Function(Rc::new(method.bind(instance.clone())))),
					None => Err(runtime(format!("{} has no property '{}'", inst.class.name, name))),
				}
			}

			Expr::Set(object, name, value) => {
				let object = self.evaluate(object)?;
				let instance = match object {
					Value::Instance(instance) => instance,
					other => return Err(runtime(format!(
						"Only instances have fields, but tried to set '.{}' on {}",
						name, other.type_name()
					))),
				};
				// Evaluate before borrowing: `self.x = self.x + 1` reads the same instance.
				let value = self.evaluate(value)?;
				instance.borrow_mut().fields.insert(name.clone(), value.clone());
				Ok(value)
			}

			Expr::SelfRef => self
				.env
				.borrow()
				.get("self")
				.map_err(|_| runtime("'self' can only be used inside a method")),

			Expr::Super(method) => {
				let superclass = self.env.borrow().get("super");
				let this = self.env.borrow().get("self");
				match (superclass, this) {
					(Ok(Value::Class(superclass)), Ok(Value::Instance(instance))) => {
						match superclass.find_method(method) {
							Some(m) => Ok(Value::Function(Rc::new(m.bind(instance)))),
							None => Err(runtime(format!("{} has no method '{}'", superclass.name, method))),
						}
					}
					_ => Err(runtime("'super' can only be used inside a method of a class that has a superclass")),
				}
			}
		}
	}

	fn call_value(&mut self, callee: Value, args: Vec<Value>) -> EvalResult {
		match callee {
			Value::Native(native) => {
				if let Some(arity) = native.arity {
					check_arity(native.name, arity, args.len())?;
				}
				(native.func)(&args)
			}

			Value::Function(func) => self.call_function(&func, args),

			// Calling a class constructs an instance, then runs `init` on it.
			Value::Class(class) => {
				let instance = Rc::new(RefCell::new(Instance {
					class: class.clone(),
					fields: HashMap::new(),
				}));
				match class.find_method("init") {
					Some(init) => {
						self.call_function(&init.bind(instance.clone()), args)?;
					}
					None => check_arity(&class.name, 0, args.len())?,
				}
				Ok(Value::Instance(instance))
			}

			other => Err(runtime(format!(
				"Can only call functions and classes, not {}",
				other.type_name()
			))),
		}
	}

	fn call_function(&mut self, func: &Function, args: Vec<Value>) -> EvalResult {
		let params = &func.decl.params;
		check_arity(&func.decl.name, params.len(), args.len())?;

		if self.depth >= MAX_CALL_DEPTH {
			return Err(runtime(format!(
				"Stack overflow: more than {} nested calls (infinite recursion in '{}'?)",
				MAX_CALL_DEPTH, func.decl.name
			)));
		}

		// New scope whose parent is where the function was *defined*, not
		// where it's being called. That's lexical scoping, and it's what makes
		// closures work.
		let env = Environment::new_child(func.closure.clone());
		for (param, arg) in params.iter().zip(args) {
			env.borrow_mut().define(param, arg);
		}

		self.depth += 1;
		let result = self.execute_block(&func.decl.body, env);
		self.depth -= 1;

		Ok(result?.unwrap_or(Value::Nil))
	}
}

fn check_arity(name: &str, expected: usize, got: usize) -> Result<(), AetherError> {
	if expected == got {
		Ok(())
	} else {
		let plural = if expected == 1 { "" } else { "s" };
		Err(runtime(format!(
			"{}() expected {} argument{} but got {}",
			name, expected, plural, got
		)))
	}
}

fn binary(op: &Token, l: Value, r: Value) -> EvalResult {
	use Value::*;
	let overflow = || runtime("Integer overflow");

	match (op, &l, &r) {
		(Token::EqualEqual, _, _) => Ok(Bool(l == r)),
		(Token::BangEqual, _, _) => Ok(Bool(l != r)),

		(Token::Plus, Int(a), Int(b)) => a.checked_add(*b).map(Int).ok_or_else(overflow),
		(Token::Minus, Int(a), Int(b)) => a.checked_sub(*b).map(Int).ok_or_else(overflow),
		(Token::Star, Int(a), Int(b)) => a.checked_mul(*b).map(Int).ok_or_else(overflow),
		(Token::Slash | Token::Percent, Int(_), Int(0)) => Err(runtime("Division by zero")),
		(Token::Slash, Int(a), Int(b)) => a.checked_div(*b).map(Int).ok_or_else(overflow),
		(Token::Percent, Int(a), Int(b)) => a.checked_rem(*b).map(Int).ok_or_else(overflow),

		(Token::Less, Int(a), Int(b)) => Ok(Bool(a < b)),
		(Token::LessEqual, Int(a), Int(b)) => Ok(Bool(a <= b)),
		(Token::Greater, Int(a), Int(b)) => Ok(Bool(a > b)),
		(Token::GreaterEqual, Int(a), Int(b)) => Ok(Bool(a >= b)),

		(Token::Less, Str(a), Str(b)) => Ok(Bool(a < b)),
		(Token::LessEqual, Str(a), Str(b)) => Ok(Bool(a <= b)),
		(Token::Greater, Str(a), Str(b)) => Ok(Bool(a > b)),
		(Token::GreaterEqual, Str(a), Str(b)) => Ok(Bool(a >= b)),

		// If either side is a string, `+` concatenates: "x = " + 5 -> "x = 5"
		(Token::Plus, Str(_), _) | (Token::Plus, _, Str(_)) => {
			Ok(Str(format!("{}{}", l, r).into()))
		}

		_ => Err(runtime(format!(
			"Can't use {:?} on {} and {}",
			op,
			l.type_name(),
			r.type_name()
		))),
	}
}
