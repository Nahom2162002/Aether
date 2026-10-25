# Aether Language (v0.2)

A small object-oriented scripting language with a tree-walking interpreter written in Rust.

```
class Point {
    fn init(x, y) { self.x = x; self.y = y; }
    fn len2() { return self.x * self.x + self.y * self.y; }
}

let p = Point(3, 4);
print("len2 =", p.len2());   // len2 = 25
```

## Running

```
cargo run                               # interactive REPL
cargo run -- examples/05_classes.ae     # run a file
cargo run -- --tokens file.ae           # show lexer output
cargo run -- --ast file.ae              # show parser output
cargo test                              # check all examples still produce their .out
```

## Types
- `int` (64-bit, overflow is an error)
- `bool`: `true`, `false`
- `string`: `"hello\n"` (escapes: `\n \t \" \\`)
- `nil`
- functions, classes, and instances

Only `false` and `nil` are falsey; everything else (including `0` and `""`) is truthy.

## Syntax

```
// comments
let x = 10;                 // declare
x = x + 1;                  // reassign (must already exist)

if x > 10 { ... } else if x == 10 { ... } else { ... }
while x > 0 { x = x - 1; }

fn add(a, b) { return a + b; }
let f = add;                // functions are values
fn counter() {              // closures capture variables
    let n = 0;
    fn next() { n = n + 1; return n; }
    return next;
}
```

## Operators
| Precedence (low → high) | Operators |
|---|---|
| assignment | `=` |
| or / and | `\|\|` `&&` (short-circuit, return the deciding operand) |
| equality | `==` `!=` |
| comparison | `<` `<=` `>` `>=` (ints and strings) |
| term | `+` `-` (`+` with a string on either side concatenates) |
| factor | `*` `/` `%` |
| unary | `!` `-` |
| call / property | `f()` `obj.field` |

## Classes
```
class Animal {
    fn init(name) { self.name = name; }    // constructor, runs on Animal("Rex")
    fn speak() { return self.name + " makes a sound"; }
}

class Dog < Animal {                        // inheritance
    fn speak() { return super.speak() + "!"; }
}
```
- Methods use `self` implicitly; don't list it as a parameter.
- Fields are created by assigning them: `self.x = 1;` or `p.x = 1;`.
- Instances are shared references: `let q = p;` gives two names for one object.

## Built-in functions
| Function | Does |
|---|---|
| `print(a, b, ...)` | prints values separated by spaces |
| `input()` / `input(prompt)` | reads a line from stdin |
| `len(s)` | length of a string |
| `str(x)` | converts anything to a string |
| `int(x)` | converts a string or bool to an int |
| `type(x)` | `"int"`, `"string"`, ..., or the class name for instances |
| `clock()` | milliseconds since 1970 |

To add one, write a Rust fn in [src/interpreter/natives.rs](src/interpreter/natives.rs) and register it there.

See [ROADMAP.md](ROADMAP.md) for how it works internally and ideas for what to build next.
