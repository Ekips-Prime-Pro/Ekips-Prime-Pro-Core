# Compiler

The compiler (`crates/compiler`) turns an `.ekips` file into bytecode in
several passes. Every pass reports *all* problems it finds, in English and
German, with a hint on how to fix them.

| Pass | Module | Finds |
|---|---|---|
| Lexer | `lexer.rs` | unknown characters, unknown units, unclosed texts/comments |
| Parser | `parser.rs` | syntax errors, misspelled keywords, `await` outside `async`, missing `main` |
| Semantic analysis | `sema.rs` | unknown names, scopes, `const`, wrong use of built-ins, units (this page) |
| Code generation | `codegen.rs` | size limits of the bytecode format |

API: `ekips_compiler::parse(source)` runs lexer and parser only,
`ekips_compiler::check(source)` also runs semantic analysis. If there are
syntax errors, `check` reports only those (see decision D3).

## Semantic analysis

### Scopes

Scoping works like in JavaScript:

- `let` and `const` are visible from their line to the end of their block
  (`{ … }`, the body of `if`/`while`/`for`, a function).
- A `for (let i = …)` or `for (const x of …)` variable is only visible in the loop.
- Function parameters are visible in the function body.
- `function` declarations are *hoisted*: they can be called above the line
  where they are written.
- Code inside a function runs later, so it may use variables that are
  declared further down in an outer block. Top-level code and code in the
  same function may not (`UsedBeforeDeclaration`).

### Checks

| Problem | Example | Message (English) |
|---|---|---|
| Unknown name | `await wiat(1s)` | I don't know the name `wiat`. *Did you mean `wait`?* |
| Unknown member | `robot.drvie(20cm)`, `robot.fahre(20cm)` | `DriveBase` has no `drvie`. *Did you mean `drive`?* |
| Declared twice in one block | `let x = 1; let x = 2;` | `x` already exists here. (+ note with the first line) |
| Hides a built-in | `const hub = 1` | `hub` is already a built-in name. |
| Used before created | `console.log(y); let y = 1;` | `y` is used before it is created. |
| Assignment to `const` | `const S = 1; S = 2;` (also `+=`, `++`) | `S` is a constant (`const`) and can not be changed. |
| Assignment to a function or built-in | `main = 1`, `hub.heading = 0deg` | … can not be changed / can only be read. |
| Command without `()` | `robot.stop;` | `robot.stop` is a command and must be called with brackets. |
| Built-in used on its own | `const h = hub` | `hub` can not be used on its own. |
| Missing `new` | `Motor(Port.A)` | A `Motor` is created with `new`. |
| `new` with a non-class | `new wait(1s)` | `wait` can not be created with `new`. |
| Calling a value | `arm.angle()` | `arm.angle` is not a command and can not be called. |
| Wrong number of arguments | `robot.drive()` | `robot.drive` needs 1 to 2 argument(s), but gets 0. |
| Wrong argument type | `new Motor(5)` | `Motor` needs a port like `Port.A` for `port`, but gets a plain number. |

"Did you mean …?" suggestions ignore upper/lower case (`spead` → `SPEED`),
know German and other words from the standard library table
(`warte` → `wait`, `fahre` → `drive`, `Color.Rot` → `Color.Red`) and never
guess for single letters.

### Types

There are no type annotations, but the compiler infers a rough type for
each expression: number of a kind (`Length`, `Angle`, `Time`, `Percent`,
`Number`), yes/no, text, list, function, Promise, port, color, or a device
(`Motor`, …). Things it can not know – parameters, list items – are
"unknown" and never cause an error. A variable gets the type of its
initial value; a plain `0` counts as unknown (counters often start at `0`).

For a value of unknown type, `.member` is checked by name across all
devices: `motor.rotate(…)` on a parameter `motor` is fine, `motor.rotat`
is an error.

### Results for later passes

`check` returns a `sema::Analysis` with

- `resolutions`: what each name refers to (declaration or built-in),
- `bindings`: every declaration with kind, type, storage (global or local
  of which function) and whether an inner function *captures* it,
- `members`: how each `.member` was resolved (namespace, known device, or
  dynamic).

Code generation and the language server use these instead of resolving
names again.

## Unit checking

Every number has a *kind*: `Length` (`mm`, `cm`), `Angle` (`deg`, `°`,
`rot`), `Time` (`s`, `ms`), `Percent` (`%`) or a plain `Number`. The
compiler derives the kind of every calculation and checks:

| Operation | Rule | Example OK | Error |
|---|---|---|---|
| `+` `-` `%` | same kind; result has that kind | `20cm + 5mm` | `20cm + 3s`, `20cm + 5` |
| `<` `<=` `>` `>=` `==` `!=` | same kind | `sonar.distance < 10cm` | `sonar.distance < 10` |
| `*` | at most one side has a unit | `2 * 90deg` | `20cm * 3cm` |
| `/` | by a plain number (keeps kind) or by the same kind (plain number) | `20cm / 2`, `20cm / 5cm` | `3 / 2s`, `20cm / 2s` |
| `=`, `+=`, … | a variable keeps its kind | `d = 7mm` | `let d = 5cm; d = 2s;` |
| `++` `--` | only plain numbers | `i++` | `d++` with `d = 5cm` |
| arguments | the kind from the standard library table | `robot.drive(20cm)` | `robot.drive(90deg)` |

- A plain `0` fits every kind: `hub.heading > 0`, `let total = 0; total += 5cm;`.
- Unknown values (parameters, list items) are never unit errors.
- For a wrong first argument, the compiler suggests a sibling command that
  takes that kind: `robot.drive(90deg)` → *Did you mean `robot.turn(…)`?*
- `Math.abs`, `Math.min`, `Math.max`, `Math.round`, `Math.floor`,
  `Math.ceil` keep the kind of their input; all inputs of `min`/`max` must
  have the same kind.

## Warnings

Warnings do not stop the program from being compiled, but it probably does
not do what was meant.

### Promise is not awaited

A command that takes time (`robot.drive`, `arm.rotate`, `hub.beep`,
`wait`, `waitUntil`, `Promise.all`/`race`, own `async` functions) returns a
Promise. If its Promise is **neither awaited, nor stored, nor passed on**
(for example to `Promise.all`), the compiler warns:

```text
warning: The program does not wait until `robot.drive(…)` is finished.
  = help: Write `await` in front to wait until it is finished. To do it at the
          same time as something else, store it (`const driving = …;`) or put
          it into `Promise.all([…])`.
```

| Code | Warning? |
|---|---|
| `robot.drive(20cm);` | yes |
| `await robot.drive(20cm);` | no |
| `const driving = robot.drive(20cm);` | no (stored) |
| `await Promise.all([robot.drive(20cm), wave()]);` | no (passed on) |
| `Promise.all([…]);` | yes (the combined Promise is not awaited) |
| `const go = () => robot.drive(5cm);` | no (returned by the arrow function) |

Outside `async` functions the hint says to make the function `async`.

## Code generation

`ekips_compiler::compile(source)` runs all passes and, if there are no
errors (warnings are fine), returns the encoded bytecode
(`CompileResult::bytecode`). `codegen.rs` uses the analysis results
instead of resolving names again:

- Directly declared top-level names become **globals**, everything else
  **local slots**. Variables captured by inner functions get a **cell**
  (`NewCell`); the cell is created when its block is entered, so each loop
  round has its own variable.
- Function declarations are created at the start of their block (hoisting).
- Built-ins become `CallNative` with the id from the standard library
  table; members of values of unknown type become `CallMethod` with a
  method set.
- Units are converted to base units (`20cm` → `200` mm).
- Function 0 runs the top-level code and then `await main()`.
- Size limits of the format (255 variables per function, 64 KiB code per
  function, 65535 constants) are reported as errors with a hint.

Show the bytecode of a program:

```sh
cargo run -p ekips-compiler --example check -- --asm examples/square.ekips
```

Format and instruction set: [bytecode.md](bytecode.md).

## Standard library table

All built-ins are described in **one data table**,
`crates/compiler/src/stdlib.rs` (`GLOBALS`): names, parameters with
expected kind, optional parameters, return type, the VM's native function,
documentation in both languages and aliases. The reference
[`stdlib.md`](stdlib.md) is generated from it:

```sh
cargo run -p ekips-compiler --example gen-docs
```

A test fails if the generated files are outdated.
