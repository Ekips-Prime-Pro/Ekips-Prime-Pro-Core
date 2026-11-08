# CLAUDE.md

Guidance for AI assistants (and humans) working on this repository.

## Project

**Ekips Prime Pro** is a custom programming language and firmware for the
LEGO® Education SPIKE™ Prime hub, built for a FIRST® LEGO® League team of
students in grades 7–10. The language looks like TypeScript without types,
with units on numbers (`20cm`, `90deg`, `2s`, `50%`) and `async`/`await`
for doing several things at the same time.

This repository (**Core**) contains the language, the compiler, the bytecode
format, the virtual machine and the hardware API. The other repositories
include Core via a **Git tag** (`v0.x.y`).

## Repositories

| Repository | Contents |
|---|---|
| **Ekips-Prime-Pro-Core** (this one) | Language, compiler, bytecode, VM, hardware API (`Hardware` trait) |
| Ekips-Prime-Pro-Firmware | Embassy firmware for the hub's STM32F413; implements `Hardware`, runs the VM |
| Ekips-Prime-Pro-Tools | CLI, simulator, language server (LSP) |
| Ekips-Prime-Pro-VSCode | VS Code extension (syntax highlighting, LSP client) |

## Layout of this repository

| Path | Purpose |
|---|---|
| `crates/hub-api` | `no_std`. `Hardware` trait, `Port`, `Color`, device kinds |
| `crates/bytecode` | `no_std`. Program container format, `Dim`, native function ids (instruction set planned) |
| `crates/compiler` | Lexer, parser, semantic analysis, standard library table (unit checking, code generation planned) |
| `crates/vm` | *(planned)* `no_std` + `alloc`. Bytecode interpreter with cooperative scheduler, mock hardware |
| `examples/*.ekips` | Example programs; all of them must compile without problems |
| `docs/en`, `docs/de` | Wiki/documentation, always in both languages |
| `CHANGELOG.md` | Changes per tagged version |

## Rules (apply to everything)

1. **English only** for code, identifiers, comments and commit messages.
2. **Document immediately.** Every feature and every architecture decision is
   documented in the wiki under `/docs` in the same change.
3. **Documentation is always bilingual:** `docs/en/...` and `docs/de/...`
   with the same content and the same file names. Link new pages from
   `docs/README.md`.
4. **Compiler messages** (errors and warnings) always exist in English **and**
   German, written for children, and say **how to fix** the problem (hint).
5. **Quality gate** – all of these must pass before every commit:
   ```sh
   cargo fmt --all --check
   cargo clippy --workspace --all-targets -- -D warnings
   cargo test --workspace
   cargo build --target thumbv7em-none-eabihf -p ekips-hub-api -p ekips-bytecode
   ```
   New features need tests.
6. Small, meaningful commits; push regularly.
7. **Releases:** Firmware and Tools pin Core by Git tag. For each release,
   update `CHANGELOG.md`, bump `workspace.package.version` and create an
   annotated tag (`git tag -a vX.Y.Z -m "..."`).

## Architecture notes

- The standard library is described in **one data table**
  (`crates/compiler/src/stdlib.rs`). Semantic analysis, code generation,
  generated docs (`docs/*/stdlib.md`) and later the LSP all read from it.
  Do not duplicate stdlib knowledge elsewhere. After changing it, run
  `cargo run -p ekips-compiler --example gen-docs`.
- Native function ids (the ABI between compiler and VM) live in
  `ekips-bytecode` (`Native`). A test checks that the table and the ids agree.

- The VM never talks to hardware directly, only through
  `ekips_hub_api::Hardware`.
- Decisions are recorded in `docs/*/decisions.md`.

## Useful commands

```sh
cargo test --workspace
cargo run -p ekips-compiler --example check -- examples/*.ekips
cargo run -p ekips-compiler --example check -- --de examples/square.ekips
cargo run -p ekips-compiler --example gen-docs     # regenerate docs/*/stdlib.md
```
