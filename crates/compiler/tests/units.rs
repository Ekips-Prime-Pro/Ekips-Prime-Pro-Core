//! Tests for unit checking (`20cm + 3s`, `robot.drive(90deg)`, …).

use ekips_compiler::diagnostic::ErrorKind;
use ekips_compiler::types::Dim::{self, Angle, Length, Number, Percent, Time};
use ekips_compiler::{Lang, check};

const SETUP: &str = "\
const arm = new Motor(Port.C);
const eye = new ColorSensor(Port.D);
const sonar = new DistanceSensor(Port.E);
const robot = new DriveBase(new Motor(Port.A), new Motor(Port.B), 56mm, 112mm);
";

fn program(body: &str) -> String {
    format!("{SETUP}async function main() {{\n{body}\n}}\n")
}

fn problems(body: &str) -> Vec<ErrorKind> {
    check(&program(body))
        .diagnostics
        .into_iter()
        .map(|d| d.kind)
        .collect()
}

fn assert_clean(body: &str) {
    let source = program(body);
    let result = check(&source);
    let rendered: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.render(&source, "test.ekips", Lang::En))
        .collect();
    assert!(rendered.is_empty(), "unexpected:\n{}", rendered.join("\n"));
}

fn mismatch(op: &'static str, left: Dim, right: Dim) -> ErrorKind {
    ErrorKind::UnitMismatch { op, left, right }
}

#[test]
fn calculating_with_units() {
    assert_clean(
        "const a = 20cm + 5mm;\nconst b = 2 * 90deg;\nconst c = 90deg * 2;\n\
         const d = 20cm / 2;\nconst ratio = 20cm / 5cm;\nconst e = -3s;\n\
         const f = 1rot - 90deg;\nconst g = (20cm + 1cm) * 3 - 2mm;\n\
         let speed = 100%;\nspeed -= 20%;\nspeed *= 2;\nconst h = 10 % 3;",
    );
    // The result keeps the unit, so this is checked further:
    assert_eq!(
        problems("const len = 20cm * 2;\nconst x = len + 1s;"),
        [mismatch("+", Length, Time)]
    );
    // length / length is a plain number
    assert_eq!(
        problems("const r = 20cm / 5cm;\nconst x = r + 1cm;"),
        [mismatch("+", Number, Length)]
    );
}

#[test]
fn mixing_kinds_is_an_error() {
    assert_eq!(
        problems("const x = 20cm + 3s;"),
        [mismatch("+", Length, Time)]
    );
    assert_eq!(
        problems("const x = 90deg - 1cm;"),
        [mismatch("-", Angle, Length)]
    );
    assert_eq!(
        problems("const x = 20cm + 5;"),
        [mismatch("+", Length, Number)]
    );
    assert_eq!(
        problems("if (sonar.distance < 10) {}"),
        [mismatch("<", Length, Number)]
    );
    assert_eq!(
        problems("if (hub.heading == 50%) {}"),
        [mismatch("==", Angle, Percent)]
    );
}

#[test]
fn zero_fits_every_kind() {
    assert_clean(
        "if (hub.heading > 0) {}\nconst x = 0 + 5cm;\nlet total = 0;\n\
         total += 5cm;\ntotal = total + 1cm;\nawait robot.drive(0);",
    );
}

#[test]
fn multiplying_and_dividing() {
    assert_eq!(
        problems("const x = 20cm * 3cm;"),
        [ErrorKind::UnitProduct {
            left: Length,
            right: Length
        }]
    );
    assert_eq!(
        problems("const x = 3 / 2s;"),
        [ErrorKind::UnitQuotient {
            left: Number,
            right: Time
        }]
    );
    assert_eq!(
        problems("const speed = 20cm / 2s;"),
        [ErrorKind::UnitQuotient {
            left: Length,
            right: Time
        }]
    );
}

#[test]
fn variables_keep_their_kind() {
    assert_eq!(
        problems("let d = 5cm;\nd = 2s;"),
        [ErrorKind::UnitAssign {
            name: Some("d".into()),
            old: Length,
            new: Time
        }]
    );
    assert_eq!(
        problems("let d = 5cm;\nd += 2s;"),
        [mismatch("+", Length, Time)]
    );
    assert_eq!(
        problems("let d = 5cm;\nd /= 1cm;"),
        [ErrorKind::UnitAssign {
            name: Some("d".into()),
            old: Length,
            new: Number
        }]
    );
    assert_eq!(
        problems("let d = 5cm;\nd++;"),
        [ErrorKind::UnitIncrement(Length)]
    );
    assert_clean("let d = 5cm;\nd = 7mm;\nd = 0;\nlet i = 1;\ni++;");
}

#[test]
fn wrong_kind_of_argument() {
    let wrong =
        |name: &str, param, expected, found, alternative: Option<&str>| ErrorKind::WrongArgUnit {
            name: name.into(),
            param,
            expected,
            found,
            alternative: alternative.map(Into::into),
        };
    assert_eq!(
        problems("await robot.drive(90deg);"),
        [wrong(
            "robot.drive",
            "distance",
            Length,
            Angle,
            Some("robot.turn")
        )]
    );
    assert_eq!(
        problems("await robot.turn(20cm);"),
        [wrong(
            "robot.turn",
            "angle",
            Angle,
            Length,
            Some("robot.drive")
        )]
    );
    assert_eq!(
        problems("await robot.drive(20);"),
        [wrong("robot.drive", "distance", Length, Number, None)]
    );
    assert_eq!(
        problems("await robot.drive(20cm, 2s);"),
        [wrong("robot.drive", "speed", Percent, Time, None)]
    );
    assert_eq!(
        problems("await arm.rotate(20cm);"),
        [wrong("arm.rotate", "angle", Angle, Length, None)]
    );
    assert_eq!(
        problems("await wait(500);"),
        [wrong("wait", "time", Time, Number, None)]
    );
    assert_eq!(
        problems("const r = new DriveBase(arm, arm, 56, 112mm);"),
        [wrong("DriveBase", "wheelDiameter", Length, Number, None)]
    );
    assert_eq!(
        problems("const m = Math.min(1cm, 2s);"),
        [wrong("Math.min", "values", Length, Time, None)]
    );
    assert_eq!(
        problems("const s = Math.sqrt(4cm);"),
        [wrong("Math.sqrt", "x", Number, Length, None)]
    );
}

#[test]
fn builtins_return_the_right_kind() {
    // Math keeps the kind of its input; properties have kinds.
    assert_clean(
        "await robot.drive(Math.abs(sonar.distance - 30cm));\n\
         await robot.turn(-hub.heading);\n\
         await arm.rotate(Math.max(arm.angle, 10deg), Math.round(hub.battery));\n\
         await wait(hub.timer);\nconst r = eye.reflection < 20%;",
    );
    assert_eq!(
        problems("await robot.drive(hub.heading);"),
        [ErrorKind::WrongArgUnit {
            name: "robot.drive".into(),
            param: "distance",
            expected: Length,
            found: Angle,
            alternative: Some("robot.turn".into()),
        }]
    );
}

#[test]
fn unknown_values_are_never_unit_errors() {
    assert_clean(
        "async function go(d) { await robot.drive(d); await robot.turn(d); }\n\
         await go(20cm);\nconst list = [1cm, 2s];\nconst x = list[0] + 3s;",
    );
}

#[test]
fn unit_messages_in_both_languages() {
    let source = program(
        "const a = 20cm + 3s;\nawait robot.drive(90deg);\nconst b = 2cm * 2cm;\n\
         const c = 1 / 2s;\nlet d = 1cm;\nd = 1s;\nd++;\nawait robot.drive(5);",
    );
    let result = check(&source);
    assert_eq!(result.diagnostics.len(), 7, "{:?}", result.diagnostics);
    for d in &result.diagnostics {
        for lang in [Lang::En, Lang::De] {
            assert!(d.hint(lang).is_some(), "{:?}", d.kind);
        }
    }
    let de = result.diagnostics[1].render(&source, "t.ekips", Lang::De);
    assert!(de.contains("Meintest du `robot.turn(…)`?"), "{de}");
    let en = result.diagnostics[0].render(&source, "t.ekips", Lang::En);
    assert!(
        en.contains("A length and a time can not be combined with `+`."),
        "{en}"
    );
}
