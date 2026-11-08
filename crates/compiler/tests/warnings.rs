//! Tests for the warning about Promises that are not awaited.

use ekips_compiler::diagnostic::{ErrorKind, Severity};
use ekips_compiler::{Lang, check};

const SETUP: &str = "\
const arm = new Motor(Port.C);
const robot = new DriveBase(new Motor(Port.A), new Motor(Port.B), 56mm, 112mm);
async function wave() { await arm.rotate(90deg); }
function plain() { return 1; }
";

fn program(body: &str) -> String {
    format!("{SETUP}async function main() {{\n{body}\n}}\n")
}

fn warnings(source: &str) -> Vec<ErrorKind> {
    check(source)
        .diagnostics
        .into_iter()
        .filter(|d| d.severity() == Severity::Warning)
        .map(|d| d.kind)
        .collect()
}

fn unused(name: Option<&str>, in_async: bool) -> ErrorKind {
    ErrorKind::UnusedPromise {
        name: name.map(Into::into),
        in_async,
    }
}

#[test]
fn forgotten_await_warns() {
    assert_eq!(
        warnings(&program("robot.drive(20cm);")),
        [unused(Some("robot.drive"), true)]
    );
    assert_eq!(
        warnings(&program("arm.rotate(90deg);\nhub.beep();\nwait(1s);")),
        [
            unused(Some("arm.rotate"), true),
            unused(Some("hub.beep"), true),
            unused(Some("wait"), true)
        ]
    );
    // Own async functions return a Promise, too.
    assert_eq!(warnings(&program("wave();")), [unused(Some("wave"), true)]);
    // The combined Promise must be awaited as well.
    assert_eq!(
        warnings(&program("Promise.all([robot.drive(1cm), wave()]);")),
        [unused(Some("Promise.all"), true)]
    );
    // Method of a value of unknown type that returns a Promise everywhere
    assert_eq!(
        warnings(&format!(
            "{SETUP}function lift(m) {{ m.rotate(90deg); }}\nasync function main() {{}}"
        )),
        [unused(Some("m.rotate"), false)]
    );
}

#[test]
fn awaited_stored_or_combined_promises_do_not_warn() {
    let source = program(
        "await robot.drive(20cm);\n\
         const lifting = arm.rotate(90deg);\n\
         let later;\nlater = wave();\n\
         await Promise.all([robot.drive(1cm), lifting, later]);\n\
         await Promise.race([robot.drive(1m), waitUntil(() => false)]);\n\
         const go = () => robot.drive(5cm);\n\
         await go();",
    )
    .replace("1m", "100cm");
    let result = check(&source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
}

#[test]
fn commands_without_promise_do_not_warn() {
    assert!(
        warnings(&program(
            "robot.stop();\narm.run(50%);\nhub.show(1);\nplain();"
        ))
        .is_empty()
    );
}

#[test]
fn outside_async_functions_the_hint_differs() {
    let source = format!(
        "{SETUP}function f() {{ robot.drive(20cm); }}\nrobot.turn(90deg);\nasync function main() {{}}"
    );
    assert_eq!(
        warnings(&source),
        [
            unused(Some("robot.drive"), false),
            unused(Some("robot.turn"), false)
        ]
    );
    let d = &check(&source).diagnostics[0];
    assert!(
        d.hint(Lang::En)
            .unwrap()
            .contains("Make the function `async`")
    );
    assert!(
        d.hint(Lang::De)
            .unwrap()
            .contains("Mache die Funktion `async`")
    );
}

#[test]
fn warnings_are_not_errors() {
    let source = program("robot.drive(20cm);");
    let result = check(&source);
    assert!(!result.has_errors());
    assert_eq!(result.diagnostics.len(), 1);
    let en = result.diagnostics[0].render(&source, "t.ekips", Lang::En);
    assert!(
        en.starts_with("warning: The program does not wait until `robot.drive(…)` is finished."),
        "{en}"
    );
    assert!(en.contains("help: Write `await` in front"), "{en}");
    let de = result.diagnostics[0].render(&source, "t.ekips", Lang::De);
    assert!(
        de.starts_with("Warnung: Das Programm wartet nicht, bis `robot.drive(…)` fertig ist."),
        "{de}"
    );
}
