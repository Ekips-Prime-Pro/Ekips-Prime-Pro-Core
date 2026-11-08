# Virtual machine (`ekips-vm`)

The VM runs compiled programs. It is `no_std` with `alloc`, so the same
code runs in the hub firmware (STM32F413) and in the simulator on a PC. It
never talks to hardware directly: every motor, sensor and hub call goes
through the `Hardware` trait from `ekips-hub-api`.

## Using it

```rust
use ekips_vm::{MockHub, Status, Vm};

let bytes = ekips_compiler::compile(source).bytecode.unwrap();
let mut vm = Vm::load(&bytes)?;        // checks header and format version
let mut hub = MockHub::new();          // or the real hardware in the firmware
loop {
    match vm.tick(&mut hub)? {         // runs the program a little further
        Status::Finished => break,
        Status::Running => hub.advance(10), // firmware: wait ~1 ms
    }
}
```

`tick` never blocks. The firmware calls it in its main loop; between ticks
it can update motors, sensors and USB.

## Tasks and the cooperative scheduler

- Every running `async` function is a **task** with its own stack.
  Function 0 (setup + `await main()`) is the root task.
- Calling an `async` function creates a task and runs it **right away**
  until it has to wait – like JavaScript. The caller then gets a Promise.
- `await` on a Promise that is not done yet **suspends** the task.
- `tick` goes through all tasks once. A waiting task is woken when its
  Promise is done; a ready task runs until it waits or has used up its
  **time slice** (`DEFAULT_BUDGET` = 1000 instructions, `Vm::set_budget`).
  Because every task has its own slice, a task in an endless loop
  (`while (!done) {}`) can not stop the others.
- The program is finished when the root task returns, i.e. when `main` is
  done. Movements that are still running are not stopped by the VM; the
  firmware stops all motors after the program.

## Promises

| Created by | Done when |
|---|---|
| `motor.rotate(…)` | the motor reports `motor_done`, or the motor gets a new command |
| `robot.drive(…)`, `robot.turn(…)` | `drive_done`, or the drive base gets a new command (`stop`, `drive`, …) |
| `wait(t)`, `hub.beep(…)` | the time has passed |
| `waitUntil(f)` | `f()` returns a truthy value (checked once per tick while awaited) |
| `Promise.all([…])` | all are done; the result is the list of results |
| `Promise.race([…])` | the first is done; the others keep running |
| calling an `async` function | the function returns; the result is its return value |

The condition of `waitUntil` runs to the end on every check; it must not
use `await` and must finish within 10 000 instructions.

## Values

`undefined`, `null`, yes/no, numbers (`f32` with their kind), texts,
lists (shared references), functions (closures with captured cells),
Promises, ports, colors and devices. `console.log` prints numbers in the
units students write: `20cm`, `90deg`, `2.5s`, `50%`.

## Runtime errors

Problems the compiler can not see (a device is not plugged in, endless
recursion, writing far behind the end of a list, …) stop the program with
a `RuntimeError`: a kind, the source line, and a message in English and
German (`message(Lang::De)`).

## Mock hardware

`MockHub` is a simulated hub for tests, examples and the simulator:

- simulated clock: nothing moves until `advance(ms)`;
- movements take realistic time (100% = 1000 degrees per second at the
  motor or wheel);
- every call is recorded in `log` / `log_lines()`;
- sensor values can be set (`set`) or scheduled (`script(time, change)`);
- `MockHub::new()` accepts every device, `MockHub::with_devices(…)` only
  the listed ones (to test "nothing plugged in" errors).

## Example: run a program and print the hardware calls

```sh
cargo run -p ekips-vm --example run -- examples/square.ekips
cargo run -p ekips-vm --example run -- examples/drive_to_line.ekips --at 1500 color C black
```

```text
[  0.000s] drive: configure left A right B wheel 5.6cm axle 11.2cm
[  0.000s] drive: straight 100cm at 40%
[  1.500s] drive: stop
[  1.500s] motor A: stop
[  1.500s] motor B: stop
[  1.500s] console: Found the line!
program finished after 1.510s
```

Options: `--de` (German messages), `--asm` (show bytecode),
`--at TIME color|distance|press|release PORT [VALUE]` (change a sensor).
