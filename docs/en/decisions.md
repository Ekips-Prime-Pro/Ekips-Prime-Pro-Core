# Architecture decisions

Short records of decisions that shape the project. Newest last. Each entry
says what was decided and why.

## D1 – Documentation lives in `docs/`, in English and German

- **Decision:** The wiki is a set of Markdown files in `docs/en` and
  `docs/de` with identical structure. `docs/README.md` links both.
- **Why:** Students on the team read German, contributors and AI assistants
  mostly English. Keeping the docs next to the code means they are versioned
  and tagged together with the compiler they describe.
