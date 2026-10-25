use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

use super::environment::Environment;
use crate::ast::FnDecl;
use crate::error::AetherError;

// Every runtime value in Aether. Cloning is cheap: the big things sit behind Rc.
#[derive(Clone)]
pub enum Value {
	Int(i64),
	Bool(bool),
	Str(Rc<str>),
	Nil,
	Function(Rc<Function>),
	Native(NativeFn),
	Class(Rc<Class>),
	// RefCell because instances are mutable (p.x = 5) and shared (two vars, one object).
	Instance(Rc<RefCell<Instance>>),
}

impl Value {
	// Aether's rule: only `false` and `nil` are falsey.
	pub fn is_truthy(&self) -> bool {
		!matches!(self, Value::Bool(false) | Value::Nil)
	}

	pub fn type_name(&self) -> String {
		match self {
			Value::Int(_) => "int".into(),
			Value::Bool(_) => "bool".into(),
			Value::Str(_) => "string".into(),
			Value::Nil => "nil".into(),
			Value::Function(_) | Value::Native(_) => "function".into(),
			Value::Class(_) => "class".into(),
			Value::Instance(i) => i.borrow().class.name.clone(),
		}
	}
}

// Primitives compare by value; objects, classes and functions by identity.
impl PartialEq for Value {
	fn eq(&self, other: &Self) -> bool {
		match (self, other) {
			(Value::Int(a), Value::Int(b)) => a == b,
			(Value::Bool(a), Value::Bool(b)) => a == b,
			(Value::Str(a), Value::Str(b)) => a == b,
			(Value::Nil, Value::Nil) => true,
			(Value::Function(a), Value::Function(b)) => Rc::ptr_eq(a, b),
			(Value::Native(a), Value::Native(b)) => a.name == b.name,
			(Value::Class(a), Value::Class(b)) => Rc::ptr_eq(a, b),
			(Value::Instance(a), Value::Instance(b)) => Rc::ptr_eq(a, b),
			_ => false,
		}
	}
}

// A built-in function written in Rust (print, len, ...).
#[derive(Clone)]
pub struct NativeFn {
	pub name: &'static str,
	pub arity: Option<usize>, // None = any number of arguments
	pub func: fn(&[Value]) -> Result<Value, AetherError>,
}

// A user-defined function. `closure` is the environment it was *defined* in,
// which is what lets inner functions see outer variables.
pub struct Function {
	pub decl: Rc<FnDecl>,
	pub closure: Rc<RefCell<Environment>>,
}

impl Function {
	// Turns `Point.len` into "len, but with `self` set to this particular point".
	// The new scope sits between the method's closure and its parameters, so the
	// body finds `self` one level up.
	pub fn bind(&self, instance: Rc<RefCell<Instance>>) -> Function {
		let env = Environment::new_child(self.closure.clone());
		env.borrow_mut().define("self", Value::Instance(instance));
		Function { decl: self.decl.clone(), closure: env }
	}
}

pub struct Class {
	pub name: String,
	pub superclass: Option<Rc<Class>>,
	pub methods: HashMap<String, Rc<Function>>,
}

impl Class {
	// Own methods win; otherwise walk up the inheritance chain.
	pub fn find_method(&self, name: &str) -> Option<Rc<Function>> {
		match self.methods.get(name) {
			Some(method) => Some(method.clone()),
			None => self.superclass.as_ref()?.find_method(name),
		}
	}
}

pub struct Instance {
	pub class: Rc<Class>,
	pub fields: HashMap<String, Value>,
}

impl fmt::Display for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		match self {
			Value::Int(n) => write!(f, "{}", n),
			Value::Bool(b) => write!(f, "{}", b),
			Value::Str(s) => write!(f, "{}", s),
			Value::Nil => write!(f, "nil"),
			Value::Function(func) => write!(f, "<fn {}>", func.decl.name),
			Value::Native(n) => write!(f, "<native fn {}>", n.name),
			Value::Class(c) => write!(f, "<class {}>", c.name),
			Value::Instance(i) => write!(f, "<{} instance>", i.borrow().class.name),
		}
	}
}

impl fmt::Debug for Value {
	fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
		write!(f, "{}", self)
	}
}
