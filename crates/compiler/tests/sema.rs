//! Tests for semantic analysis: scopes, names, built-ins and arguments.

use ekips_compiler::diagnostic::{ErrorKind, MemberOwner};
use ekips_compiler::sema::{BindingKind, Storage};
use ekips_compiler::types::Ty;
use ekips_compiler::{Lang, check};

const SETUP: &str = "\
const leftWheel = new Motor(Port.A);
const rightWheel = new Motor(Port.B);
const arm = new Motor(Port.C);
const eye = new ColorSensor(Port.D);
const robot = new DriveBase(leftWheel, rightWheel, 56mm, 112mm);
";

/// Setup + `body` inside `async function main`.
fn program(body: &str) -> String {
    format!("{SETUP}async function main() {{\n{body}\n}}\n")
}

/// Kinds of all problems in a full program.
fn problems_in(source: &str) -> Vec<ErrorKind> {
    check(source)
        .diagnostics
        .into_iter()
        .map(|d| d.kind)
        .collect()
}

/// Kinds of all problems when `body` is put into `main`.
fn problems(body: &str) -> Vec<ErrorKind> {
    problems_in(&program(body))
}

fn assert_clean(source: &str) {
    let result = check(source);
    let rendered: Vec<String> = result
        .diagnostics
        .iter()
        .map(|d| d.render(source, "test.ekips", Lang::En))
        .collect();
    assert!(
        rendered.is_empty(),
        "unexpected problems:\n{}",
        rendered.join("\n")
    );
}

fn unknown_name(name: &str, suggestion: Option<&str>) -> ErrorKind {
    ErrorKind::UnknownName {
        name: name.into(),
        suggestion: suggestion.map(Into::into),
    }
}

#[test]
fn example_programs_check_cleanly() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    let mut count = 0;
    for entry in std::fs::read_dir(dir).expect("examples directory") {
        let path = entry.expect("entry").path();
        if path.extension().is_some_and(|e| e == "ekips") {
            assert_clean(&std::fs::read_to_string(&path).expect("readable"));
            count += 1;
        }
    }
    assert!(count >= 4);
}

#[test]
fn language_reference_first_program_checks_cleanly() {
    assert_clean(
        "const leftWheel = new Motor(Port.A);
const rightWheel = new Motor(Port.B);
const robot = new DriveBase(leftWheel, rightWheel, 56mm, 112mm);
async function square(side) {
  for (let i = 0; i < 4; i++) {
    await robot.drive(side);
    await robot.turn(90deg);
  }
}
async function main() {
  await square(20cm);
  await hub.beep();
}",
    );
}

// ----------------------------------------------------------------------
// Names
// ----------------------------------------------------------------------

#[test]
fn unknown_names_get_suggestions() {
    assert_eq!(
        problems("await wiat(1s);"),
        [unknown_name("wiat", Some("wait"))]
    );
    assert_eq!(
        problems("const SPEED = 50%;\nawait robot.drive(10cm, spead);"),
        [unknown_name("spead", Some("SPEED"))]
    );
    // German word for a built-in
    assert_eq!(
        problems("await warte(1s);"),
        [unknown_name("warte", Some("wait"))]
    );
    assert_eq!(problems("x = 1;"), [unknown_name("x", None)]);
}

#[test]
fn unknown_members_get_suggestions() {
    let member = |owner: &str, member: &str, suggestion: Option<&str>| ErrorKind::UnknownMember {
        owner: MemberOwner::Name(owner.into()),
        member: member.into(),
        suggestion: suggestion.map(Into::into),
    };
    assert_eq!(
        problems("await robot.drvie(20cm);"),
        [member("DriveBase", "drvie", Some("drive"))]
    );
    assert_eq!(
        problems("await robot.fahre(20cm);"),
        [member("DriveBase", "fahre", Some("drive"))]
    );
    assert_eq!(
        problems("await arm.turn(90deg);"),
        [member("Motor", "turn", Some("rotate"))]
    );
    assert_eq!(
        problems("await hub.bep();"),
        [member("hub", "bep", Some("beep"))]
    );
    assert_eq!(
        problems("const c = Color.Rot;"),
        [member("Color", "Rot", Some("Red"))]
    );
    assert_eq!(problems("const p = Port.G;"), [member("Port", "G", None)]);
}

#[test]
fn members_of_unknown_values_are_checked_by_name() {
    // `motor` is a parameter, so its type is unknown – but `rotate` exists.
    assert_clean(&program(
        "async function lift(motor) { await motor.rotate(90deg); }\nawait lift(arm);",
    ));
    assert_eq!(
        problems("async function lift(motor) { await motor.rotat(90deg); }\nawait lift(arm);"),
        [ErrorKind::UnknownMember {
            owner: MemberOwner::Value(Ty::Any),
            member: "rotat".into(),
            suggestion: Some("rotate".into()),
        }]
    );
}

#[test]
fn block_scoping() {
    assert_eq!(
        problems("if (true) { let inside = 1; }\nconsole.log(inside);"),
        [unknown_name("inside", None)]
    );
    assert_eq!(
        problems("for (let i = 0; i < 3; i++) {}\nconsole.log(i);"),
        [unknown_name("i", None)]
    );
    assert_eq!(
        problems("for (const m of [arm]) {}\nconsole.log(m);"),
        [unknown_name("m", None)]
    );
    // The same name in an inner block is a new variable: fine.
    assert_clean(&program(
        "let x = 1;\nif (true) { let x = 2; console.log(x); }",
    ));
    // Parameters are visible in the function body only.
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f(a) {{ return a; }}\nasync function main() {{ console.log(a); }}"
        )),
        [unknown_name("a", None)]
    );
}

#[test]
fn duplicate_declarations() {
    let dup = |n: &str| ErrorKind::DuplicateDeclaration(n.into());
    assert_eq!(problems("let x = 1;\nlet x = 2;"), [dup("x")]);
    assert_eq!(problems("let x = 1;\nconst x = 2;"), [dup("x")]);
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f() {{}}\nfunction f() {{}}\nasync function main() {{}}"
        )),
        [dup("f")]
    );
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f(a, a) {{}}\nasync function main() {{}}"
        )),
        [dup("a")]
    );
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f(a) {{ let a = 1; }}\nasync function main() {{}}"
        )),
        [dup("a")]
    );
    assert_eq!(
        problems_in(&format!(
            "{SETUP}const arm = 1;\nasync function main() {{}}"
        )),
        [dup("arm")]
    );

    // The note points at the first declaration.
    let source = program("let x = 1;\nlet x = 2;");
    let d = &check(&source).diagnostics[0];
    let first = source.find("let x = 1").unwrap() + 4;
    assert_eq!(d.related.map(|s| s.start), Some(first));
    assert!(
        d.render(&source, "t.ekips", Lang::En)
            .contains("note: see t.ekips:7:5")
    );
}

#[test]
fn builtin_names_can_not_be_reused() {
    assert_eq!(
        problems("const hub = 1;"),
        [ErrorKind::ShadowsBuiltin("hub".into())]
    );
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function wait() {{}}\nasync function main() {{}}"
        )),
        [ErrorKind::ShadowsBuiltin("wait".into())]
    );
}

#[test]
fn use_before_declaration() {
    assert_eq!(
        problems("console.log(later);\nconst later = 1;"),
        [ErrorKind::UsedBeforeDeclaration("later".into())]
    );
    assert_eq!(
        problems("const x = x + 1;"),
        [ErrorKind::UsedBeforeDeclaration("x".into())]
    );
    // Functions are hoisted, and function bodies run later:
    // both may refer to things declared further down.
    assert_clean(&format!(
        "async function main() {{ await helper(); }}\n\
         async function helper() {{ await robot.drive(LENGTH); }}\n\
         const LENGTH = 20cm;\n{SETUP}"
    ));
    assert_clean(&program(
        "const check = () => limit > 3;\nconst limit = 5;\nconsole.log(check());",
    ));
}

// ----------------------------------------------------------------------
// Assignments
// ----------------------------------------------------------------------

#[test]
fn constants_can_not_be_changed() {
    let c = |n: &str| ErrorKind::AssignToConst(n.into());
    assert_eq!(problems("const SPEED = 50%;\nSPEED = 20%;"), [c("SPEED")]);
    assert_eq!(problems("const n = 1;\nn += 1;"), [c("n")]);
    assert_eq!(problems("const n = 1;\nn++;"), [c("n")]);
    assert_eq!(problems("for (const m of [1]) { m = 2; }"), [c("m")]);
    assert_eq!(problems("arm = 1;"), [c("arm")]);
    assert_clean(&program("let n = 1;\nn = 2;\nn += 1;\nn++;\n--n;"));
    assert_clean(&program("for (let m of [1]) { m = 2; }"));
    // Changing the contents of a const list is fine.
    assert_clean(&program("const list = [1, 2];\nlist[0] = 5;"));
}

#[test]
fn functions_and_builtins_can_not_be_changed() {
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f() {{}}\nasync function main() {{ f = 1; }}"
        )),
        [ErrorKind::AssignToFunction("f".into())]
    );
    assert_eq!(
        problems("hub.heading = 0deg;"),
        [ErrorKind::ReadOnly("hub.heading".into())]
    );
    assert_eq!(
        problems("arm.angle = 0deg;"),
        [ErrorKind::ReadOnly("arm.angle".into())]
    );
    assert_eq!(problems("Math = 1;"), [ErrorKind::ReadOnly("Math".into())]);
}

// ----------------------------------------------------------------------
// Using built-ins
// ----------------------------------------------------------------------

#[test]
fn builtins_must_be_used_correctly() {
    assert_eq!(
        problems("robot.stop;"),
        [ErrorKind::MustCall("robot.stop".into())]
    );
    assert_eq!(
        problems("const w = wait;"),
        [ErrorKind::MustCall("wait".into())]
    );
    assert_eq!(
        problems("const h = hub;"),
        [ErrorKind::NotAValue("hub".into())]
    );
    assert_eq!(
        problems("Motor.rotate(90deg);"),
        [ErrorKind::NotAValue("Motor".into())]
    );
    assert_eq!(
        problems("const m = Motor(Port.E);"),
        [ErrorKind::MissingNew("Motor".into())]
    );
    assert_eq!(
        problems("const w = new wait(1s);"),
        [ErrorKind::NotAClass(Some("wait".into()))]
    );
    assert_eq!(
        problems("const a = arm.angle();"),
        [ErrorKind::NotCallable(Some("arm.angle".into()))]
    );
    assert_eq!(
        problems("const n = 5;\nn();"),
        [ErrorKind::NotCallable(Some("n".into()))]
    );
}

#[test]
fn argument_counts() {
    let count = |name: &str, min, max, found| ErrorKind::WrongArgCount {
        name: name.into(),
        min,
        max,
        found,
    };
    assert_eq!(
        problems("robot.drive();"),
        [count("robot.drive", 1, Some(2), 0)]
    );
    assert_eq!(
        problems("robot.drive(1cm, 50%, 3);"),
        [count("robot.drive", 1, Some(2), 3)]
    );
    assert_eq!(
        problems("robot.stop(1);"),
        [count("robot.stop", 0, Some(0), 1)]
    );
    assert_eq!(
        problems("const m = new Motor();"),
        [count("Motor", 1, Some(1), 0)]
    );
    assert_eq!(
        problems("const x = Math.min(1);"),
        [count("Math.min", 2, None, 1)]
    );
    assert_clean(&program("console.log();\nconsole.log(1, 2, 3, 4);"));
    // User functions
    assert_eq!(
        problems_in(&format!(
            "{SETUP}function f(a, b) {{}}\nasync function main() {{ f(1); }}"
        )),
        [count("f", 2, Some(2), 1)]
    );
    assert_eq!(
        problems("const g = (x) => x;\ng(1, 2);"),
        [count("g", 1, Some(1), 2)]
    );
}

#[test]
fn argument_types() {
    assert!(matches!(
        problems("const m = new Motor(5);")[..],
        [ErrorKind::WrongArgType {
            param: "port",
            found: Ty::Num(_),
            ..
        }]
    ));
    assert!(matches!(
        problems("const s = eye.sees(Port.A);")[..],
        [ErrorKind::WrongArgType {
            param: "color",
            found: Ty::Port,
            ..
        }]
    ));
    assert!(matches!(
        problems("const r = new DriveBase(arm, eye, 56mm, 112mm);")[..],
        [ErrorKind::WrongArgType { param: "right", .. }]
    ));
    assert!(matches!(
        problems("await waitUntil(true);")[..],
        [ErrorKind::WrongArgType {
            param: "condition",
            found: Ty::Bool,
            ..
        }]
    ));
    assert!(matches!(
        problems("await Promise.all(robot.drive(1cm));")[..],
        [ErrorKind::WrongArgType {
            param: "promises",
            found: Ty::Promise,
            ..
        }]
    ));
    // Unknown types are never an error.
    assert_clean(&program(
        "function make(p) { return new Motor(p); }\nconst m = make(Port.E);",
    ));
}

// ----------------------------------------------------------------------
// Analysis results
// ----------------------------------------------------------------------

#[test]
fn bindings_and_captures_are_recorded() {
    let source = program(
        "let counter = 0;\nlet plain = 1;\nconsole.log(plain);\n\
         await waitUntil(() => { counter++; return counter > 3; });",
    );
    let result = check(&source);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let find = |name: &str| {
        result
            .analysis
            .bindings
            .values()
            .find(|b| b.name == name)
            .unwrap_or_else(|| panic!("binding {name}"))
    };
    assert!(find("counter").captured);
    assert!(!find("plain").captured);
    assert_eq!(find("robot").storage, Storage::Global);
    assert!(!find("robot").captured, "globals never need a cell");
    assert_eq!(
        find("robot").ty,
        Ty::Object(ekips_compiler::types::Class::DriveBase)
    );
    assert_eq!(find("main").kind, BindingKind::Function);
    assert!(matches!(find("counter").storage, Storage::Local(_)));
    // Every use of `robot` resolves to its declaration.
    let uses = result
        .analysis
        .resolutions
        .values()
        .filter(|r| matches!(r, ekips_compiler::sema::Resolution::Binding(s) if *s == find("robot").decl))
        .count();
    assert_eq!(uses, 0, "robot is not used in this program");
}

#[test]
fn syntax_errors_hide_semantic_errors() {
    // The broken first line would otherwise cause "unknown name `x`".
    let kinds = problems("let = 5;\nconsole.log(x);");
    assert!(
        kinds
            .iter()
            .all(|k| !matches!(k, ErrorKind::UnknownName { .. }))
    );
    assert!(!kinds.is_empty());
}

#[test]
fn every_semantic_message_has_text_and_hint_in_both_languages() {
    let source = program(
        "await wiat(1s);\nawait robot.drvie(2cm);\nlet x = 1;\nlet x = 2;\n\
         const hub = 1;\nconsole.log(y);\nlet y = 1;\nconst C = 1;\nC = 2;\n\
         arm.angle = 1deg;\nrobot.stop;\nconst h = Math;\nMotor(Port.A);\n\
         new wait();\narm.angle();\nrobot.drive();\nnew Motor(1);",
    );
    let result = check(&source);
    assert_eq!(result.diagnostics.len(), 14, "{:?}", result.diagnostics);
    for d in &result.diagnostics {
        for lang in [Lang::En, Lang::De] {
            assert!(!d.message(lang).is_empty());
            assert!(
                d.hint(lang).is_some(),
                "no hint for {:?} in {lang:?}",
                d.kind
            );
        }
        assert_ne!(d.message(Lang::En), d.message(Lang::De));
    }
}
