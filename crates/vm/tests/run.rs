//! End-to-end tests: compile programs, run them on the mock hub and check
//! the recorded hardware calls.

use ekips_hub_api::{Color, DeviceKind, Port};
use ekips_vm::mock::Change;
use ekips_vm::{ErrorKind, Lang, MockHub, RuntimeError, Status, Vm};

const SETUP: &str = "\
const arm = new Motor(Port.C);
const eye = new ColorSensor(Port.D);
const bumper = new ForceSensor(Port.E);
const robot = new DriveBase(new Motor(Port.A), new Motor(Port.B), 56mm, 112mm);
";

fn compile(source: &str) -> Vec<u8> {
    let result = ekips_compiler::compile(source);
    let errors: Vec<String> = result
        .diagnostics
        .iter()
        .filter(|d| d.is_error())
        .map(|d| d.render(source, "test.ekips", ekips_compiler::Lang::En))
        .collect();
    assert!(errors.is_empty(), "compile errors:\n{}", errors.join("\n"));
    result.bytecode.expect("bytecode")
}

/// Runs until the program is finished; returns the log without the setup lines.
fn run_on(source: &str, hub: &mut MockHub) -> Result<Vec<String>, RuntimeError> {
    let mut vm = Vm::load(&compile(source)).expect("valid program");
    loop {
        match vm.tick(hub)? {
            Status::Finished => break,
            Status::Running => {
                assert!(hub.now() < 600_000, "program did not finish");
                hub.advance(10);
            }
        }
    }
    Ok(hub
        .log_lines()
        .into_iter()
        .filter(|l| !l.contains("configure"))
        .collect())
}

fn run(body: &str) -> Vec<String> {
    run_with(body, &mut MockHub::new())
}

fn run_with(body: &str, hub: &mut MockHub) -> Vec<String> {
    let source = format!("{SETUP}async function main() {{\n{body}\n}}\n");
    run_on(&source, hub).unwrap_or_else(|e| panic!("runtime error: {e}"))
}

/// Only the `console.log` output.
fn output(body: &str) -> Vec<String> {
    run(body)
        .into_iter()
        .filter_map(|l| l.split_once("console: ").map(|(_, t)| t.to_string()))
        .collect()
}

#[test]
fn example_programs_run_to_the_end() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "ekips") {
            let source = std::fs::read_to_string(&path).unwrap();
            let mut hub = MockHub::new();
            // Let waitUntil conditions of the examples become true.
            hub.script(1000, Change::Color(Port::C, Color::Black));
            hub.script(1000, Change::Color(Port::E, Color::Green));
            hub.script(1000, Change::Distance(Port::D, Some(50.0)));
            let log = run_on(&source, &mut hub).unwrap();
            assert!(!log.is_empty(), "{} did nothing", path.display());
        }
    }
}

#[test]
fn square_drives_and_turns_in_order() {
    let log = run_on(
        &std::fs::read_to_string(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../examples/square.ekips"
        ))
        .unwrap(),
        &mut MockHub::new(),
    )
    .unwrap();
    assert_eq!(
        log,
        [
            "[  0.000s] hub: show \"Hi\"",
            "[  0.000s] drive: straight 20cm at 50%",
            "[  0.820s] drive: turn 90deg at 50%",
            "[  1.180s] drive: straight 20cm at 50%",
            "[  2.000s] drive: turn 90deg at 50%",
            "[  2.360s] drive: straight 20cm at 50%",
            "[  3.180s] drive: turn 90deg at 50%",
            "[  3.540s] drive: straight 20cm at 50%",
            "[  4.360s] drive: turn 90deg at 50%",
            "[  4.720s] hub: beep 500 Hz for 200ms",
        ]
    );
}

#[test]
fn promise_all_runs_things_at_the_same_time() {
    let log = run(
        "await Promise.all([robot.drive(20cm), arm.rotate(90deg)]);\nconsole.log(\"both done\");",
    );
    assert_eq!(
        log,
        [
            "[  0.000s] drive: straight 20cm at 50%",
            "[  0.000s] motor C: rotate 90deg at 50%",
            // the drive takes 820ms, the arm 180ms: Promise.all waits for both
            "[  0.820s] console: both done",
        ]
    );
}

#[test]
fn promise_race_finishes_with_the_first() {
    let mut hub = MockHub::new();
    hub.script(300, Change::Pressed(Port::E, true));
    let log = run_with(
        "await Promise.race([robot.drive(100cm), waitUntil(() => bumper.pressed)]);\n\
         robot.stop();\nconsole.log(\"bump\");",
        &mut hub,
    );
    assert_eq!(log[0], "[  0.000s] drive: straight 100cm at 50%");
    assert_eq!(log[1], "[  0.300s] drive: stop");
    assert_eq!(log.last().unwrap(), "[  0.300s] console: bump");
}

#[test]
fn stored_promises_run_in_the_background() {
    let log = run(
        "const lifting = arm.rotate(360deg);\nawait robot.drive(10cm);\n\
         console.log(\"driven\");\nawait lifting;\nconsole.log(\"lifted\");",
    );
    assert_eq!(
        log,
        [
            "[  0.000s] motor C: rotate 360deg at 50%",
            "[  0.000s] drive: straight 10cm at 50%",
            "[  0.410s] console: driven",
            "[  0.720s] console: lifted",
        ]
    );
}

#[test]
fn async_functions_start_right_away_like_javascript() {
    let source = format!(
        "{SETUP}async function job(name) {{ console.log(name + \" start\"); await wait(100ms); \
         console.log(name + \" end\"); }}\n\
         async function main() {{ const a = job(\"a\"); const b = job(\"b\"); \
         console.log(\"main\"); await Promise.all([a, b]); console.log(\"all\"); }}"
    );
    let out: Vec<String> = run_on(&source, &mut MockHub::new())
        .unwrap()
        .into_iter()
        .map(|l| l.split_once("console: ").unwrap().1.to_string())
        .collect();
    assert_eq!(out, ["a start", "b start", "main", "a end", "b end", "all"]);
}

#[test]
fn async_function_results() {
    assert_eq!(
        output(
            "async function twice(x) { await wait(10ms); return x * 2; }\n\
             const r = await twice(21);\nconsole.log(r);\n\
             const all = await Promise.all([twice(1), twice(2), 7]);\nconsole.log(all);\n\
             const first = await Promise.race([wait(1s), twice(5)]);\nconsole.log(first);"
        ),
        ["42", "[2, 4, 7]", "10"]
    );
}

#[test]
fn values_and_units_are_printed_nicely() {
    assert_eq!(
        output(
            "console.log(20cm + 5mm, 1.5rot, 2500ms, 50%, 10 % 3, 7 / 2);\n\
             console.log(\"a\" + 1, true, null, undefined, [1, \"x\"], Color.Red, Port.B);\n\
             console.log(20cm / 5cm, 2 * 90deg, -3s, Math.abs(-3cm), Math.max(1cm, 3cm, 2cm));"
        ),
        [
            "20.5cm 540deg 2.5s 50% 1 3.5",
            "a1 true null undefined [1, \"x\"] Color.Red Port.B",
            "4 180deg -3s 3cm 3cm",
        ]
    );
}

#[test]
fn control_flow() {
    assert_eq!(
        output(
            "let sum = 0;\nfor (let i = 0; i < 10; i++) { if (i == 2) continue; if (i == 5) break; sum += i; }\n\
             console.log(sum);\nlet n = 0;\nwhile (true) { n++; if (n >= 3) { break; } }\nconsole.log(n);\n\
             const list = [3, 4];\nlet total = 0;\nfor (const x of list) { total = total + x; }\nconsole.log(total);\n\
             console.log(n > 2 ? \"big\" : \"small\", 0 || \"default\", 1 && 2, !0);"
        ),
        ["8", "3", "7", "big default 2 true"]
    );
}

#[test]
fn lists_and_texts() {
    assert_eq!(
        output(
            "const list = [1, 2];\nlist[2] = 3;\nlist[0] += 10;\nconst old = list[1]++;\n\
             console.log(list, list.length, old, list[9]);\nconst t = \"Hallo\";\n\
             console.log(t.length, t[1]);"
        ),
        ["[11, 3, 3] 3 2 undefined", "5 a"]
    );
}

#[test]
fn functions_closures_and_recursion() {
    assert_eq!(
        output(
            "function fact(n) { return n <= 1 ? 1 : n * fact(n - 1); }\nconsole.log(fact(5));\n\
             let counter = 0;\nconst inc = () => { counter++; return counter; };\ninc();\ninc();\n\
             console.log(counter);\n\
             const fns = [];\nfor (const i of [1, 2, 3]) { fns[fns.length] = () => i * 10; }\n\
             console.log(fns[0](), fns[2]());\n\
             function makeAdder(a) { return (b) => a + b; }\nconsole.log(makeAdder(2)(3));\n\
             function outer() { let x = 1; function inner() { return () => x + 1; } x = 5; return inner()(); }\n\
             console.log(outer());"
        ),
        ["120", "2", "10 30", "5", "6"]
    );
}

#[test]
fn wait_until_with_a_captured_counter() {
    assert_eq!(
        output(
            "let ticks = 0;\nawait waitUntil(() => { ticks++; return ticks >= 5; });\nconsole.log(ticks);"
        ),
        ["5"]
    );
}

#[test]
fn devices_as_parameters_use_dynamic_calls() {
    let log = run(
        "async function lift(motor, amount) { await motor.rotate(amount); console.log(motor.angle); }\n\
         await lift(arm, 90deg);",
    );
    assert_eq!(
        log,
        [
            "[  0.000s] motor C: rotate 90deg at 50%",
            "[  0.180s] console: 90deg",
        ]
    );
}

#[test]
fn timer_and_heading() {
    let log = run(
        "await wait(1s);\nhub.resetTimer();\nawait robot.turn(90deg);\n\
         console.log(hub.heading, hub.timer > 0s);",
    );
    assert_eq!(log.last().unwrap(), "[  1.360s] console: 90deg true");
}

#[test]
fn endless_loops_do_not_block_other_tasks() {
    // `spin` never awaits; the scheduler still lets main continue.
    let source = format!(
        "{SETUP}let stop = false;\nasync function spin() {{ while (!stop) {{}} }}\n\
         async function main() {{ const s = spin(); await wait(50ms); stop = true; await s; \
         console.log(\"done\"); }}"
    );
    let log = run_on(&source, &mut MockHub::new()).unwrap();
    assert_eq!(log.len(), 1);
    assert!(log[0].ends_with("console: done"), "{log:?}");
}

#[test]
fn missing_device_is_a_runtime_error_with_line() {
    let mut hub = MockHub::with_devices(&[(Port::A, DeviceKind::Motor)]);
    let source =
        "const a = new Motor(Port.A);\nconst b = new Motor(Port.B);\nasync function main() {}";
    let err = run_on(source, &mut hub).unwrap_err();
    assert_eq!(
        err.kind,
        ErrorKind::Hardware(ekips_hub_api::HardwareError::NoDevice(Port::B))
    );
    assert_eq!(err.line, 2);
    assert_eq!(
        err.message(Lang::De),
        "An diesem Anschluss ist nichts eingesteckt."
    );
    assert_eq!(
        err.to_string(),
        "line 2: Nothing is plugged into this port."
    );
}

#[test]
fn runtime_errors() {
    let err = |body: &str| {
        let source = format!("{SETUP}async function main() {{\n{body}\n}}\n");
        run_on(&source, &mut MockHub::new()).unwrap_err()
    };
    let e = err("function f(n) { return f(n + 1); }\nf(1);");
    assert_eq!(e.kind, ErrorKind::TooDeep);
    let e = err("const list = [1];\nlist[5] = 2;");
    assert_eq!((e.kind, e.line), (ErrorKind::IndexOutOfRange, 7));
    let e = err("await waitUntil(async () => true);");
    assert_eq!(e.kind, ErrorKind::ConditionWaits);
    let e = err("await waitUntil(() => { while (true) {} });");
    assert_eq!(e.kind, ErrorKind::ConditionTooLong);
    let e = err("const x = [1] - 2;");
    assert_eq!(e.kind, ErrorKind::BadOperands);
}

#[test]
fn every_runtime_message_exists_in_both_languages() {
    use ekips_hub_api::HardwareError;
    let kinds = [
        ErrorKind::Hardware(HardwareError::NoDevice(Port::A)),
        ErrorKind::Hardware(HardwareError::WrongDevice {
            port: Port::A,
            expected: DeviceKind::Motor,
        }),
        ErrorKind::Hardware(HardwareError::NoDriveBase),
        ErrorKind::NotCallable,
        ErrorKind::NoSuchMember,
        ErrorKind::BadOperands,
        ErrorKind::BadArgument,
        ErrorKind::NotAList,
        ErrorKind::IndexOutOfRange,
        ErrorKind::ConditionWaits,
        ErrorKind::ConditionTooLong,
        ErrorKind::TooDeep,
        ErrorKind::BadProgram,
    ];
    for kind in kinds {
        let e = RuntimeError::new(kind, 1);
        assert_ne!(e.message(Lang::En), e.message(Lang::De));
    }
}

#[test]
fn broken_programs_are_rejected() {
    assert!(Vm::load(b"not a program").is_err());
    let mut bytes = compile("async function main() {}");
    bytes.truncate(bytes.len() - 3);
    assert!(Vm::load(&bytes).is_err());
}
