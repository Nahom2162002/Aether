# Aether internals & roadmap

## Pipeline

```
source ──▶ Lexer ──▶ Vec<Spanned> ──▶ Parser ──▶ Vec<Stmt> (AST) ──▶ Interpreter
src/lexer/                            src/parser.rs   src/ast.rs        src/interpreter/
```

| File | Role |
|---|---|
| `lexer/mod.rs` | characters → tokens, tracks line numbers |
| `parser.rs` | tokens → AST, one method per grammar rule (grammar at the top of the file) |
| `ast.rs` | `Expr` and `Stmt` enums |
| `interpreter/mod.rs` | walks the AST: `execute` for statements, `evaluate` for expressions |
| `interpreter/environment.rs` | variable scopes, chained to their parent |
| `interpreter/value.rs` | runtime values: `Value`, `Function`, `Class`, `Instance` |
| `interpreter/natives.rs` | built-in functions |

## How OOP works

- `class Point { ... }` evaluates to a `Value::Class` holding its methods.
- Calling a class, like `Point(3, 4)`, creates a `Value::Instance` and runs `init` if present.
- `p.x` → `Expr::Get`: check the instance's **fields** first, then the class's **methods**
  (walking up `superclass` if needed).
- When a method is fetched, it gets **bound** (`Function::bind`): wrapped in a new scope
  where `self` = that instance. That's why methods don't declare `self` as a parameter.
- `super` works the same way: a class with a parent gets one extra scope around its
  methods where `super` = the parent class.
- `p.x = 5` → `Expr::Set`: writes into the instance's field map.
- Instances are `Rc<RefCell<..>>`, so `let q = p;` makes two names for the same object.
- `return` is implemented by `execute` returning `Ok(Some(value))`, which bubbles up
  through blocks and loops until `call_function` catches it.

## Ideas for next features

Roughly easiest first:

1. **More built-ins** (`abs`, `min`, `max`, `sqrt`, `random`). Just add to `natives.rs`.
2. **`for` loops**: `for i in 0..10 { }`. Parse it, then *desugar* into a `while` in the parser,
   so the interpreter needs no changes.
3. **`break` / `continue`**: like `return`, but caught by the `while` loop. Change
   `ExecResult` to `Result<Flow, AetherError>` with `enum Flow { Normal, Return(Value), Break, Continue }`.
4. **Floats**: add `Token::Float(f64)` / `Value::Float(f64)` and the int/float mix rules in `binary`.
5. **Lists**: `[1, 2, 3]`, `xs[0]`, `xs[0] = 5`, `len(xs)`, `push(xs, 4)`.
   `Value::List(Rc<RefCell<Vec<Value>>>)` plus `Expr::List`, `Expr::Index`, `Expr::SetIndex`.
6. **Line numbers on runtime errors**: store a line on `Expr::Call`, `Expr::Binary`, `Expr::Get`
   and report it in `AetherError::Runtime`.
7. **Static checks before running**: a "resolver" pass over the AST that catches
   `return` outside functions, `self` outside methods, and unused variables.
8. **Speed**: compile the AST to bytecode and run it on a stack VM
   (*Crafting Interpreters* Part III). Roughly 10× faster than tree-walking.

Further reading: *Crafting Interpreters* (craftinginterpreters.com) follows this exact design.
