// Built-in functions. Adding a new one = write a fn + add one line to `register`.

use std::cell::RefCell;
use std::io::{self, BufRead, Write};
use std::rc::Rc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::environment::Environment;
use super::value::{NativeFn, Value};
use crate::error::{runtime, AetherError};

type NativeResult = Result<Value, AetherError>;

pub fn register(globals: &Rc<RefCell<Environment>>) {
	let natives = [
		NativeFn { name: "print", arity: None, func: print },
		NativeFn { name: "input", arity: None, func: input },
		NativeFn { name: "len", arity: Some(1), func: len },
		NativeFn { name: "str", arity: Some(1), func: str },
		NativeFn { name: "int", arity: Some(1), func: int },
		NativeFn { name: "type", arity: Some(1), func: type_of },
		NativeFn { name: "clock", arity: Some(0), func: clock },
	];
	let mut env = globals.borrow_mut();
	for native in natives {
		env.define(native.name, Value::Native(native));
	}
}

// print(a, b, c) prints the values separated by spaces.
fn print(args: &[Value]) -> NativeResult {
	let parts: Vec<String> = args.iter().map(|v| v.to_string()).collect();
	println!("{}", parts.join(" "));
	Ok(Value::Nil)
}

// input() or input("prompt: ") reads one line from stdin (without the newline).
fn input(args: &[Value]) -> NativeResult {
	if args.len() > 1 {
		return Err(runtime("input() takes at most 1 argument"));
	}
	if let Some(prompt) = args.first() {
		print!("{}", prompt);
		io::stdout().flush().ok();
	}
	let mut line = String::new();
	io::stdin()
		.lock()
		.read_line(&mut line)
		.map_err(|e| runtime(format!("input() failed: {}", e)))?;
	Ok(Value::Str(line.trim_end_matches(['\r', '\n']).into()))
}

fn len(args: &[Value]) -> NativeResult {
	match &args[0] {
		Value::Str(s) => Ok(Value::Int(s.chars().count() as i64)),
		other => Err(runtime(format!("len() expects a string, got {}", other.type_name()))),
	}
}

fn str(args: &[Value]) -> NativeResult {
	Ok(Value::Str(args[0].to_string().into()))
}

fn int(args: &[Value]) -> NativeResult {
	match &args[0] {
		Value::Int(n) => Ok(Value::Int(*n)),
		Value::Bool(b) => Ok(Value::Int(*b as i64)),
		Value::Str(s) => s
			.trim()
			.parse()
			.map(Value::Int)
			.map_err(|_| runtime(format!("int() can't convert \"{}\" to a number", s))),
		other => Err(runtime(format!("int() can't convert {}", other.type_name()))),
	}
}

// type(5) == "int", type(Point(1, 2)) == "Point"
fn type_of(args: &[Value]) -> NativeResult {
	Ok(Value::Str(args[0].type_name().into()))
}

// Milliseconds since 1970. Subtract two calls to time something.
fn clock(_args: &[Value]) -> NativeResult {
	let ms = SystemTime::now()
		.duration_since(UNIX_EPOCH)
		.map(|d| d.as_millis() as i64)
		.unwrap_or(0);
	Ok(Value::Int(ms))
}
