//! Checks `.ekips` files and prints all problems.
//!
//! ```text
//! cargo run -p ekips-compiler --example check -- [--de] [--ast] FILE...
//! ```
//!
//! This is a temporary helper until the real CLI lives in the Tools repo.

use std::process::ExitCode;

use ekips_compiler::{Lang, parse};

fn main() -> ExitCode {
    let mut lang = Lang::En;
    let mut show_ast = false;
    let mut files = Vec::new();
    for arg in std::env::args().skip(1) {
        match arg.as_str() {
            "--de" => lang = Lang::De,
            "--ast" => show_ast = true,
            _ => files.push(arg),
        }
    }
    if files.is_empty() {
        eprintln!("usage: check [--de] [--ast] FILE...");
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
        let result = parse(&source);
        if show_ast {
            println!("{:#?}", result.program);
        }
        for diag in &result.diagnostics {
            eprintln!("{}", diag.render(&source, path, lang));
        }
        if result.has_errors() {
            failed = true;
            let n = result.diagnostics.len();
            match lang {
                Lang::En => eprintln!("{path}: {n} problem(s)"),
                Lang::De => eprintln!("{path}: {n} Problem(e)"),
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
