//! Compiler for the Ekips Prime Pro robot language.
//!
//! Ekips Prime Pro uses a TypeScript-like syntax without type annotations,
//! with unit suffixes on numbers (`20cm`, `90deg`, `2s`, `50%`) and
//! `async`/`await` for running things at the same time. See
//! `docs/language.md` for the full reference.
//!
//! **Status:** lexer and parser are done. Name resolution, unit checking,
//! code generation and the VM follow.
//!
//! ```
//! let source = "async function main() {\n  await robot.drive(20cm);\n}\n";
//! let result = ekips_compiler::parse(source);
//! assert!(result.diagnostics.is_empty());
//! assert_eq!(result.program.body.len(), 1);
//! ```

pub mod ast;
pub mod diagnostic;
pub mod keywords;
pub mod lexer;
pub mod parser;
pub mod span;
mod suggest;

pub use diagnostic::{Diagnostic, Lang};

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
    /// `true` if there is at least one problem.
    pub fn has_errors(&self) -> bool {
        !self.diagnostics.is_empty()
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
