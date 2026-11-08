//! Compiler for the Ekips Prime Pro robot language.
//!
//! Ekips Prime Pro uses a TypeScript-like syntax without type annotations,
//! with unit suffixes on numbers (`20cm`, `90deg`, `2s`, `50%`) and
//! `async`/`await` for running things at the same time. See
//! `docs/en/language.md` (English) or `docs/de/language.md` (German) for the
//! full reference.
//!
//! Pipeline: [`lexer`] → [`parser`] → [`sema`] (names, scopes, types).
//! The standard library is described in one data table, [`stdlib`].
//!
//! ```
//! let source = "const robot = new DriveBase(new Motor(Port.A), new Motor(Port.B), 56mm, 112mm);\n\
//!               async function main() {\n  await robot.drive(20cm);\n}\n";
//! let result = ekips_compiler::check(source);
//! assert!(result.diagnostics.is_empty());
//!
//! let result = ekips_compiler::check("async function main() { await robot.drvie(20cm); }");
//! assert!(result.has_errors()); // `robot` is unknown
//! ```

pub mod ast;
pub mod diagnostic;
pub mod keywords;
pub mod lexer;
pub mod parser;
pub mod sema;
pub mod span;
pub mod stdlib;
mod suggest;
pub mod types;

pub use diagnostic::{Diagnostic, Lang, Severity};

/// Result of [`parse`]: the syntax tree plus every problem found.
///
/// The tree is returned even when there are errors, so tools like the
/// language server can still offer completions for the parts that are fine.
#[derive(Debug, Clone)]
pub struct ParseResult {
    /// The syntax tree (possibly incomplete if there were errors)
    pub program: ast::Program,
    /// All problems, in source order
    pub diagnostics: Vec<Diagnostic>,
}

impl ParseResult {
    /// `true` if there is at least one error (warnings do not count).
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Parses an Ekips Prime Pro source file.
pub fn parse(source: &str) -> ParseResult {
    let (tokens, mut diagnostics) = lexer::lex(source);
    let (program, parse_diags) = parser::Parser::new(&tokens).parse_program();
    diagnostics.extend(parse_diags);
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    ParseResult {
        program,
        diagnostics,
    }
}

/// Result of [`check`]: syntax tree, analysis results and all problems.
#[derive(Debug, Clone)]
pub struct CheckResult {
    /// The syntax tree (possibly incomplete if there were syntax errors)
    pub program: ast::Program,
    /// Names, bindings and members resolved by semantic analysis
    pub analysis: sema::Analysis,
    /// All problems (errors and warnings), in source order
    pub diagnostics: Vec<Diagnostic>,
}

impl CheckResult {
    /// `true` if there is at least one error (warnings do not count).
    pub fn has_errors(&self) -> bool {
        self.diagnostics.iter().any(Diagnostic::is_error)
    }
}

/// Parses and analyzes a source file.
///
/// If the parser found syntax errors, only those are reported: a broken
/// line often hides a declaration, and the resulting "unknown name" errors
/// would only confuse. The analysis is still returned, so editors can use it.
pub fn check(source: &str) -> CheckResult {
    let parsed = parse(source);
    let analysis = sema::analyze(&parsed.program);
    let mut diagnostics = parsed.diagnostics;
    if !diagnostics.iter().any(Diagnostic::is_error) {
        diagnostics.extend(analysis.diagnostics.iter().cloned());
    }
    diagnostics.sort_by_key(|d| (d.span.start, d.span.end));
    diagnostics.dedup();
    CheckResult {
        program: parsed.program,
        analysis,
        diagnostics,
    }
}
