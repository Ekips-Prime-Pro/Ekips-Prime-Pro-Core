# Bytecode format and instruction set

The compiler produces one binary file per program. The hub (or the
simulator) loads it into the VM (`ekips-vm`). The format is defined in
`crates/bytecode` (`no_std`), which both sides use.

## File layout

All integers are little-endian.

| Part | Contents |
|---|---|
| Header (16 bytes) | magic `EKPP`, format version (u16, currently `1`), flags (u16, `0`), code length (u32), data length (u32) |
| Code section | the instructions of all functions, one after the other |
| Data section | globals count, function table, constant pool |

The firmware checks magic and version first. A program for another format
version is rejected with a clear message instead of crashing.

### Data section

```text
u16 globals                    number of global variables
u16 function_count
function_count × function:
    str  name                  u16 length + UTF-8
    u32  offset, u32 length    position in the code section
    u8   params, u8 locals     locals include the parameters
    u8   is_async
    u8   capture_count
    capture_count × (u8 kind, u8 index)   kind 0 = local slot, 1 = upvalue
u16 constant_count
constant_count × constant:
    0: f32 value, u8 kind      number (kind: 0 Number, 1 Length, 2 Angle, 3 Time, 4 Percent)
    1: str                     text
    2: u8 port                 0 = A … 5 = F
    3: u8 color                order of `Color::ALL`
    4: u8 n, n × u16 native    method set (for `CallMethod`)
```

**Function 0** is the entry point. It runs the top-level code (setup) and
then `await main()`. The program is finished when function 0 returns.

## Values and units

Numbers are `f32` (the hub's FPU is single precision) and carry their
*kind*. Units are converted by the compiler to one base unit per kind:

| Kind | Base unit | `20cm` is stored as |
|---|---|---|
| Length | mm | `200` |
| Angle | degree | `1.5rot` → `540` |
| Time | ms | `2s` → `2000` |
| Percent | % | `50%` → `50` |

The kind is only used for printing (`console.log(20cm)` prints `20cm`) and
for the kind of a result (`2 * 90deg` is an angle). All unit *checks*
happen in the compiler.

## Instructions

The VM is a **stack machine**. Each instruction is one opcode byte plus
operands. Jump targets are offsets relative to the start of the current
function (u16, so a function can have at most 64 KiB of code).

| Group | Instructions |
|---|---|
| Constants | `Const idx`, `Undefined`, `Null`, `True`, `False` |
| Stack | `Pop`, `Dup`, `Dup2` |
| Variables | `LoadLocal n`, `StoreLocal n`, `LoadGlobal n`, `StoreGlobal n` |
| Captured variables | `NewCell n`, `LoadCell n`, `StoreCell n`, `LoadUpval n`, `StoreUpval n` |
| Arithmetic | `Add`, `Sub`, `Mul`, `Div`, `Rem`, `Neg`, `ToNumber` |
| Logic / comparison | `Not`, `Eq`, `NotEq`, `Lt`, `Le`, `Gt`, `Ge` |
| Jumps | `Jump t`, `JumpIfFalse t`, `JumpIfTrueKeep t` (for `\|\|`), `JumpIfFalseKeep t` (for `&&`) |
| Functions | `Closure fn`, `Call argc`, `CallNative id argc`, `CallMethod set argc`, `Return` |
| Lists | `MakeList n`, `GetIndex`, `SetIndex` |
| Async | `Await` |
| Debug | `Line n` (start of source line `n`, for runtime errors) |

The exact encoding of every opcode is in `crates/bytecode/src/opcode.rs`.
`Program::disassemble()` prints a readable listing.

### Built-in functions

`CallNative id argc` runs a built-in function. The ids are the `Native`
enum in `crates/bytecode/src/native.rs` (part of the ABI). Methods get
their object as the first argument (`robot.drive(20cm)` → `robot, 200mm`).
Missing optional arguments are simply not passed.

When the compiler does not know the type of a value (a parameter
`motor` in `function lift(motor) { motor.rotate(90deg) }`), it emits
`CallMethod set argc`. The constant `set` lists all natives with that
name (`MotorRotate`); the VM picks the one whose receiver kind
(`Native::receiver()`) fits the value at runtime.

### Variables

- **Globals**: `const`/`let`/`function` directly at the top level.
- **Locals**: everything else, in numbered slots of the function's frame
  (parameters first).
- **Captured variables**: a local used by an inner function (arrow
  function, nested function) lives in a heap *cell*. The declaring
  function creates the cell with `NewCell` when the block is entered and
  uses `LoadCell`/`StoreCell`; the closure stores a reference to the cell
  and uses `LoadUpval`/`StoreUpval`. Because the cell is created on every
  entry of the block, each loop round gets its own variable (like `let` in
  JavaScript).

### Async functions and Promises

`Call` on an `async` function does not run it in the current task. It
creates a **new task**, runs it right away until its first `await` that
has to wait (like JavaScript), and pushes a **Promise** for its result.
`Await` on a pending Promise suspends the current task; the scheduler
resumes it when the Promise is done. Details: [vm.md](vm.md).
