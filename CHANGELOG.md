# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ask`: for an open list, the recorded attribute that would decide the elements its dimension
  covers and it does not. When no attribute separates a missing element from the listed ones,
  it says so and names them -- that is the answer that tells you to record something new.
- A test that walks the `--help` screen and fails when a command is listed there but missing
  from the dispatcher. Without it the command word is taken as a path and the error is
  `cannot find the file ask`, which says nothing about what is actually wrong.

## [0.3.0] - 2026-10-26

The version that learns to say what it cannot tell apart, and to say which format it speaks.

### Added

- `W-COLLIDE`: two facts that record the same evidence and the same attributes are reported,
  because nothing a machine can read tells them apart. A fact with no attributes never collides.
- A document may declare its format on the first line, `tsare 1`. The line is optional and its
  absence means format 1, so every existing document compiles unchanged. A document that
  declares a newer format stops with `E-VERSION` instead of producing parse noise.
- `--version` prints the crate version and the document format it speaks.
- A release workflow: a `v*` tag whose number differs from `Cargo.toml` fails before anything is
  published; `cargo package --locked` builds the exact package; publishing needs a token.

### Changed

- The tree tracks what the build, the tests or the package need, and nothing else: `src/`,
  `tests/`, four root documents, `docs/PROTOCOL.md`, `docs/REPO.mf` and `SELF.mf`. The rounds of
  documents, the examples, the experiment harness and the integration notes stay out of git.
- `cargo package` carries the same list, so the published crate contains no stray README.

## [0.2.0] - 2026-10-26

The version that can be published: one crate, no dependencies, MIT.

### Added

- `partition`: a list must name the concept its elements share, say whether it is closed, and
  give a reason when it is. A list with no dimension may be written only as `unknown`, names
  the question that would type it, and cannot discharge a premise.
- `fca`: the attribute pairs a concept's example share, and the facts those attributes imply.
- `propose`: gate values an outside model wants to fill into holes, with three verdicts
  (`admitted`, `review`, `rejected`) and three policies (`harmless`, `strict`, `improve`).
- `--explain CODE`: what a diagnostic means and the smallest fix; a test keeps the table
  complete for every code the compiler can print.
- `--version`, `--help` covering every command, and `PATH` may be `-` for standard input.
- Diagnostics carry an end position, are rendered with the source line and a caret, and a
  directory is rendered file by file.
- A `Session` owns ordering, counts, text, JSON and the exit code.
- Parser recovery: every syntax error in one run, as `E-SYNTAX` with a span.
- Checks derived from structure: list element types, `W-EXAMPLE-SHAPE`, `W-BY-MISMATCH`,
  `W-CONFIDENCE-INFLATED`, and the partition family.

### Changed

- The crate lives at the repository root; `rust/tsare` is gone.
- Documentation moved under `docs/`.

### Fixed

- A comma between fields is required; it had been optional, so a whole class of typos parsed.
- `E-COLLAPSE` no longer fires for a concept with no positive example.
- `at all` is not treated as a universal claim.


