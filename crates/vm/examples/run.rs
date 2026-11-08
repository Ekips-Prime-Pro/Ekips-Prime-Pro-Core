//! Compiles an `.ekips` program, runs it in the VM on a simulated hub and
//! prints every hardware call with its (simulated) time.
//!
//! ```text
//! cargo run -p ekips-vm --example run -- [--de] [--asm] FILE.ekips
//! ```
//!
//! Sensors of the simulated hub can be changed at a point in time:
//!
//! ```text
//! --at 1500 color C black      color sensor at port C sees black from 1.5 s
//! --at 800 distance D 5cm      distance sensor at port D measures 5 cm
//! --at 2000 press E            force sensor at port E is pressed
//! ```

use std::process::ExitCode;

use ekips_hub_api::{Color, Port};
use ekips_vm::mock::Change;
use ekips_vm::{Lang, MockHub, Status, Vm};

/// Simulated time per tick in ms.
const STEP_MS: u64 = 10;
/// Stop after this much simulated time.
const LIMIT_MS: u64 = 10 * 60 * 1000;

fn parse_change(args: &[String]) -> Option<(u64, Change)> {
    let time = args.first()?.parse().ok()?;
    let port = Port::from_char(args.get(2)?.chars().next()?)?;
    let change = match args.get(1)?.as_str() {
        "color" => Change::Color(port, Color::from_name(&args.get(3)?.to_lowercase())?),
        "distance" => {
            let text = args.get(3)?;
            let mm = match text.strip_suffix("cm") {
                Some(cm) => cm.parse::<f32>().ok()? * 10.0,
                None => text.strip_suffix("mm").unwrap_or(text).parse().ok()?,
            };
            Change::Distance(port, Some(mm))
        }
        "press" => Change::Pressed(port, true),
        "release" => Change::Pressed(port, false),
        _ => return None,
    };
    Some((time, change))
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let mut lang = Lang::En;
    let mut show_asm = false;
    let mut file = None;
    let mut hub = MockHub::new();
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--de" => lang = Lang::De,
            "--asm" => show_asm = true,
            "--at" => {
                let end = (i + 5).min(args.len());
                let Some((time, change)) = parse_change(&args[i + 1..end]) else {
                    eprintln!("bad --at, e.g. `--at 1500 color C black`");
                    return ExitCode::from(2);
                };
                hub.script(time, change);
                i += if matches!(
                    args.get(i + 2).map(String::as_str),
                    Some("press" | "release")
                ) {
                    3
                } else {
                    4
                };
            }
            other => file = Some(other.to_string()),
        }
        i += 1;
    }
    let Some(path) = file else {
        eprintln!("usage: run [--de] [--asm] [--at TIME SENSOR PORT VALUE]... FILE.ekips");
        return ExitCode::from(2);
    };
    let source = match std::fs::read_to_string(&path) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("{path}: {e}");
            return ExitCode::FAILURE;
        }
    };

    let compiler_lang = match lang {
        Lang::En => ekips_compiler::Lang::En,
        Lang::De => ekips_compiler::Lang::De,
    };
    let result = ekips_compiler::compile(&source);
    for d in &result.diagnostics {
        eprintln!("{}", d.render(&source, &path, compiler_lang));
    }
    let Some(bytes) = result.bytecode else {
        return ExitCode::FAILURE;
    };
    let mut vm = Vm::load(&bytes).expect("the compiler produces valid bytecode");
    if show_asm {
        println!(
            "{}",
            ekips_bytecode::Program::decode(&bytes)
                .expect("valid")
                .disassemble()
        );
    }

    let mut printed = 0;
    let outcome = loop {
        let status = vm.tick(&mut hub);
        for line in &hub.log_lines()[printed..] {
            println!("{line}");
        }
        printed = hub.log.len();
        match status {
            Ok(Status::Finished) => break Ok(()),
            Ok(Status::Running) if hub.now() >= LIMIT_MS => break Err(None),
            Ok(Status::Running) => hub.advance(STEP_MS),
            Err(e) => break Err(Some(e)),
        }
    };
    let seconds = hub.now() as f64 / 1000.0;
    match (outcome, lang) {
        (Ok(()), Lang::En) => println!("program finished after {seconds:.3}s"),
        (Ok(()), Lang::De) => println!("Programm nach {seconds:.3}s beendet"),
        (Err(None), Lang::En) => println!("stopped after {seconds:.0}s (time limit)"),
        (Err(None), Lang::De) => println!("nach {seconds:.0}s abgebrochen (Zeitlimit)"),
        (Err(Some(e)), _) => {
            let label = if lang == Lang::De {
                "Laufzeitfehler"
            } else {
                "runtime error"
            };
            let line = if lang == Lang::De { "Zeile" } else { "line" };
            eprintln!("{label} ({path}, {line} {}): {}", e.line, e.message(lang));
            return ExitCode::FAILURE;
        }
    }
    ExitCode::SUCCESS
}
