//! Checks `.ekips` files and prints all problems.
//!
//! ```text
//! cargo run -p ekips-compiler --example check -- [--de] [--ast] [--asm] FILE...
//! ```
//!
//! This is a temporary helper until the real CLI lives in the Tools repo.

use std::process::ExitCode;

use ekips_compiler::{Lang, check, compile};

fn main() -> ExitCode {
    let mut lang = Lang::En;
    let mut show_ast = false;
    let mut show_asm = false;
    let mut files = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--de" => lang = Lang::De,
            "--ast" => show_ast = true,
            "--asm" => show_asm = true,
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        eprintln!("usage: check [--de] [--ast] [--asm] FILE...");
        return ExitCode::from(2);
    }

    let mut failed = false;
    for path in &files {
        let source = match std::fs::read_to_string(path) {
            Ok(s) => s,
            Err(e) => {
                eprintln!("{path}: {e}");
                failed = true;
                continue;
            }
        };
        let result = check(&source);
        if show_ast {
            println!("{:#?}", result.program);
        }
        if show_asm {
            if let Some(bytes) = compile(&source).bytecode {
                let program = ekips_bytecode::Program::decode(&bytes).expect("valid bytecode");
                println!("{}", program.disassemble());
            }
        }
        for diag in &result.diagnostics {
            eprintln!("{}", diag.render(&source, path, lang));
        }
        let errors = result.diagnostics.iter().filter(|d| d.is_error()).count();
        let warnings = result.diagnostics.len() - errors;
        if errors > 0 {
            failed = true;
            match lang {
                Lang::En => eprintln!("{path}: {errors} error(s), {warnings} warning(s)"),
                Lang::De => eprintln!("{path}: {errors} Fehler, {warnings} Warnung(en)"),
            }
        } else if warnings > 0 {
            match lang {
                Lang::En => println!("{path}: OK with {warnings} warning(s)"),
                Lang::De => println!("{path}: in Ordnung, mit {warnings} Warnung(en)"),
            }
        } else {
            match lang {
                Lang::En => println!("{path}: OK"),
                Lang::De => println!("{path}: alles in Ordnung"),
            }
        }
    }
    if failed {
        ExitCode::FAILURE
    } else {
        ExitCode::SUCCESS
    }
}
