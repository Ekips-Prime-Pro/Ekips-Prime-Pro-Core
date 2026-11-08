# Architecture decisions

Short records of decisions that shape the project. Newest last. Each entry
says what was decided and why.

## D1 – Documentation lives in `docs/`, in English and German

- **Decision:** The wiki is a set of Markdown files in `docs/en` and
  `docs/de` with identical structure. `docs/README.md` links both.
- **Why:** Students on the team read German, contributors and AI assistants
  mostly English. Keeping the docs next to the code means they are versioned
  and tagged together with the compiler they describe.

## D2 – The standard library is one data table

- **Decision:** All built-ins are described in `GLOBALS` in
  `crates/compiler/src/stdlib.rs`, including parameter kinds, docs in both
  languages and aliases (German words). Analysis, unit checks, code
  generation, the generated reference (`docs/*/stdlib.md`) and the language
  server read this table.
- **Why:** One place to add a command. Docs, error messages and
  autocompletion can not drift apart. A test fails when the generated
  reference is outdated.

## D3 – Syntax errors hide semantic errors

- **Decision:** If the parser reports an error, `check` does not report
  semantic problems (the analysis still runs for editors).
- **Why:** A broken line often contains a declaration. Reporting "unknown
  name" for every later use would bury the one real mistake.

## D4 – Native function ids live in `ekips-bytecode`

- **Decision:** The `Native` enum (stable 16-bit ids of built-in functions)
  is part of the `no_std` bytecode crate. The stdlib table references these
  ids; a test checks that every id is used exactly once.
- **Why:** The ids are the ABI between compiler and VM. The VM must not
  depend on the compiler (which uses `std` and contains all the docs), but
  both must agree on the numbers.

## D5 – Scoping and hoisting like JavaScript

- **Decision:** `let`/`const` are block-scoped, functions are hoisted to the
  top of their block, and function bodies may refer to variables declared
  later in an outer block. Using a variable before its line in the *same*
  function (or top-level code) is an error.
- **Why:** Students who later learn JavaScript/TypeScript should not have to
  unlearn anything. The "used before created" error replaces the runtime
  error JavaScript would give.

## D6 – Units are checked at compile time, by kind

- **Decision:** The compiler infers a kind (`Length`, `Angle`, `Time`,
  `Percent`, `Number`) for every number and rejects mixing kinds. Units are
  converted to one base unit per kind at compile time (mm, degree, ms,
  percent), so `20cm + 5mm` is just `200 + 5`. Products and quotients of two
  units (`cm²`, `cm/s`) are not supported. A plain `0` fits every kind;
  values of unknown kind are never errors.
- **Why:** Most robot bugs in FLL are wrong numbers (driving 90 instead of
  turning 90). Catching them before the robot moves saves a lot of time.
  Leaving out compound units keeps the rules simple enough for grade 7.
