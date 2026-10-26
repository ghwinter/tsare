# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- `ask`: the next question the document needs, from three families that are one defect in three
  places -- a list that does not cover its dimension, two facts that record the same evidence and
  the same attributes, and a concept whose positive and negative examples look alike. Each
  question names the attribute that would decide it, or says that none can. `--one` gives the
  single question that settles the most; `--json` is the shape a loop consumes.
- A test that walks the `--help` screen and fails when a command is listed there but missing
  from the dispatcher. Without it the command word is taken as a path and the error is
  `cannot find the file ask`, which says nothing about what is actually wrong.
- `novel --attribute N [--holds IDS]`: whether a proposed attribute is a new distinction, a
  column that is already recorded, or a recombination of the recorded attributes. The decision
  is a partition comparison -- two facts with identical records that the proposal separates are
  the witness -- so it needs no model and no statistics.
- `imply [--json] [--max-antecedent N] [--min-support N]`: the laws the attribute table already
  holds -- `items=1 -> urgent=0`, with the number of facts behind it and a fact that makes the
  converse fail. The search is bounded on purpose (antecedents up to N attributes, at least M
  facts) and says so in its own output instead of claiming to be the canonical basis.
- `ask`, `novel` and `imply` are MCP tools, so a model can ask for a judgement without a shell.
  The command line and the server print the same text: both call one renderer per command, and
  a test walks `tools/list` to check every listed tool is dispatched.

### Changed

- The text of `ask`, `novel` and `imply` is produced by one function each, used by both the
  command line and the MCP server, so the two cannot drift.

### Fixed

- The check cache and the case store live in `.tsare/`, not `.metaform/`. The old name was the
  abandoned project's, and it was being created inside other people's repositories.
- `cargo package` carries no stray README from an unlisted directory.

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


