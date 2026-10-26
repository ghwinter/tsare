# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

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

- The crate lives at the repository root; `rust/engine-core` is gone.
- Documentation moved under `docs/`.

### Fixed

- A comma between fields is required; it had been optional, so a whole class of typos parsed.
- `E-COLLAPSE` no longer fires for a concept with no positive example.
- `at all` is not treated as a universal claim.
