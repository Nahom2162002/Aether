use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use super::value::Value;
use crate::error::{runtime, AetherError};

// One scope of variables. Scopes form a chain via `parent`:
//   function body -> enclosing function -> ... -> globals
pub struct Environment {
	values: HashMap<String, Value>,
	parent: Option<Rc<RefCell<Environment>>>,
}

impl Environment {
	pub fn new_global() -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(Self { values: HashMap::new(), parent: None }))
	}

	pub fn new_child(parent: Rc<RefCell<Environment>>) -> Rc<RefCell<Self>> {
		Rc::new(RefCell::new(Self { values: HashMap::new(), parent: Some(parent) }))
	}

	// `let x = ...` always creates the variable in the *current* scope.
	pub fn define(&mut self, name: &str, value: Value) {
		self.values.insert(name.to_string(), value);
	}

	// Look in this scope, then walk up the parent chain.
	pub fn get(&self, name: &str) -> Result<Value, AetherError> {
		if let Some(value) = self.values.get(name) {
			return Ok(value.clone());
		}
		match &self.parent {
			Some(parent) => parent.borrow().get(name),
			None => Err(runtime(format!("Undefined variable '{}'", name))),
		}
	}

	// `x = ...` updates an *existing* variable wherever it lives in the chain.
	pub fn assign(&mut self, name: &str, value: Value) -> Result<(), AetherError> {
		if let Some(slot) = self.values.get_mut(name) {
			*slot = value;
			return Ok(());
		}
		match &self.parent {
			Some(parent) => parent.borrow_mut().assign(name, value),
			None => Err(runtime(format!(
				"Undefined variable '{}' (use 'let {} = ...' to create it)",
				name, name
			))),
		}
	}
}
