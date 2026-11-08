# Changelog

All notable changes to Ekips Prime Pro Core. Firmware and Tools pin Core by
Git tag (`vX.Y.Z`). The format follows [Keep a Changelog](https://keepachangelog.com/),
versions follow [Semantic Versioning](https://semver.org/) (before 1.0, a
minor version may break the API).

## [0.2.0] – 2026-11-08

### Added

- `ekips_compiler::check(source)`: parsing plus semantic analysis.
- Semantic analysis (`sema`): block scopes and hoisting like JavaScript,
  unknown names and members with case-insensitive "Did you mean …?"
  (including German aliases such as `fahre` → `drive`), duplicate
  declarations, names hiding built-ins, use before declaration, assignment
  to `const`, functions and built-in values, wrong use of built-ins,
  argument count and type checks.
- Unit checking: kinds `Length`/`Angle`/`Time`/`Percent`/`Number` for every
  expression; errors for `20cm + 3s`, `20cm * 3cm`, `3 / 2s`, changing the
  kind of a variable, `++` on values with units and wrong arguments
  (`robot.drive(90deg)` suggests `robot.turn`).
- Warning (`Severity::Warning`) for Promises that are neither awaited, nor
  stored, nor passed on.
- Standard library as one data table (`stdlib::GLOBALS`) with bilingual docs;
  generated reference `docs/{en,de}/stdlib.md`.
- `ekips_bytecode::Dim` and `ekips_bytecode::Native` (native function ids).
- `Diagnostic::related` (second position, e.g. first declaration),
  `Diagnostic::is_error`.
- Bilingual wiki in `docs/` and `CLAUDE.md`.

### Changed

- `docs/language.md` moved to `docs/en/language.md`; German translation in
  `docs/de/language.md`.
- `ParseResult::has_errors` ignores warnings.
- `suggest` ignores upper/lower case and no longer guesses for single letters.

## [0.1.0]

- Lexer and parser with error recovery and bilingual error messages.
- `ekips-hub-api` (`Hardware` trait) and the bytecode header.
