//! Tests for code generation (the behaviour is tested end-to-end in `ekips-vm`).

use ekips_bytecode::{Capture, Constant, Dim, Native, Op, Program};
use ekips_compiler::compile;

fn program(source: &str) -> Program {
    let result = compile(source);
    let bytes = result
        .bytecode
        .unwrap_or_else(|| panic!("errors: {:?}", result.diagnostics));
    Program::decode(&bytes).expect("valid bytecode")
}

#[test]
fn example_programs_compile_to_valid_bytecode() {
    let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/../../examples");
    for entry in std::fs::read_dir(dir).unwrap() {
        let path = entry.unwrap().path();
        if path.extension().is_some_and(|e| e == "ekips") {
            let p = program(&std::fs::read_to_string(&path).unwrap());
            assert_eq!(p.functions[0].name, "<top>");
            assert!(p.functions.iter().any(|f| f.name == "main" && f.is_async));
        }
    }
}

#[test]
fn programs_with_errors_produce_no_bytecode() {
    let result = compile("async function main() { await robot.drive(20cm); }");
    assert!(result.bytecode.is_none());
    assert!(!result.diagnostics.is_empty());
    // Warnings alone do not stop compilation.
    let result = compile("async function main() { wait(1s); }");
    assert!(result.bytecode.is_some());
    assert_eq!(result.diagnostics.len(), 1);
}

#[test]
fn units_are_converted_to_base_units() {
    let p = program("async function main() { console.log(20cm, 1.5rot, 2s, 50%); }");
    for c in [
        Constant::Num(200.0, Dim::Length),
        Constant::Num(540.0, Dim::Angle),
        Constant::Num(2000.0, Dim::Time),
        Constant::Num(50.0, Dim::Percent),
    ] {
        assert!(
            p.constants.contains(&c),
            "{c:?} missing in {:?}",
            p.constants
        );
    }
}

#[test]
fn captured_variables_live_in_cells() {
    let p = program(
        "async function main() { let n = 0; const inc = () => { n++; }; inc(); let plain = 1; }",
    );
    let arrow = p.functions.iter().find(|f| f.name == "<arrow>").unwrap();
    assert_eq!(arrow.captures, [Capture::Local(0)]);
    let text = p.disassemble();
    assert!(text.contains("NewCell 0"), "{text}");
    assert!(text.contains("LoadUpval 0"), "{text}");
    assert!(
        text.contains("StoreLocal 2"),
        "plain stays a normal local:\n{text}"
    );
}

#[test]
fn unknown_receivers_use_method_sets() {
    let p = program("function lift(m) { return m.rotate(90deg); }\nasync function main() {}");
    assert!(
        p.constants
            .contains(&Constant::MethodSet(vec![Native::MotorRotate]))
    );
    assert!(p.disassemble().contains(Op::CallMethod.name()));
}

#[test]
fn entry_function_runs_main() {
    let p = program("async function main() {}");
    let text = p.disassemble();
    let top = &text[text.find("fn 0 <top>").unwrap()..text.find("fn 1").unwrap()];
    assert!(top.contains("Call 0") && top.contains("Await"), "{top}");
}
