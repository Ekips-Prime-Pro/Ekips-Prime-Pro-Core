# Changelog

All notable changes to Ekips Prime Pro Core. Firmware and Tools pin Core by
Git tag (`vX.Y.Z`). The format follows [Keep a Changelog](https://keepachangelog.com/),
versions follow [Semantic Versioning](https://semver.org/) (before 1.0, a
minor version may break the API).

## [0.3.0] – 2026-11-08

### Added

- Bytecode instruction set (`ekips_bytecode::Op`) and program format
  (`Program` with function table, constant pool and code; `encode`,
  `decode`, `disassemble`). Format version stays `1` (first version with
  instructions). Design doc `docs/{en,de}/bytecode.md`.
- `Native::receiver()` and `Receiver` for calls on values of unknown type.
- Code generation (`ekips_compiler::codegen`) and
  `ekips_compiler::compile(source)` → `CompileResult { bytecode, diagnostics }`.
- New crate **`ekips-vm`** (`no_std` + `alloc`): interpreter with a
  cooperative scheduler for `async`/`await` (one task per async call, one
  time slice per task and tick), Promises for movements, waits,
  `waitUntil`, `Promise.all`/`race`; all hardware access through
  `ekips_hub_api::Hardware`; runtime errors with line and bilingual messages.
- `ekips_vm::MockHub`: simulated hub with clock, movement times, scripted
  sensors and a text log of all hardware calls.
- Example `cargo run -p ekips-vm --example run -- FILE.ekips` (prints the
  hardware calls), `--asm` option of the `check` example.
- Docs `docs/{en,de}/vm.md`, decisions D7–D10.

### Changed

- `ekips-bytecode` now uses `alloc` and depends on `ekips-hub-api`.
- CI builds `ekips-vm` for `thumbv7em-none-eabihf`.

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
