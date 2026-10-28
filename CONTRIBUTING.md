# Contributing

## What this is

A deterministic checker: same input, same output, no model inside. Anything that needs a model
stays outside the binary.

## Before a pull request

```powershell
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo run --release --quiet -- check SELF.tsare docs/REPO.tsare
```

## Rules the code keeps

1. No model call, no network, no clock, no randomness in `src/`. A check that cannot be decided
   says so instead of guessing.
2. Every diagnostic code has an entry in `src/explain.rs`; `tests/semantics.rs` fails if one
   does not.
3. A new check needs the document that would have caught it: add it to `tests/fixtures/`, the
   corpus every check runs against.
4. Output changes are golden changes. Run `UPDATE_GOLDEN=1 cargo test`, then read the diff
   before committing it.
5. Negative results stay in the repository. If the compiler refuted a hypothesis, the
   refutation is part of the result.

## Commit messages

The history is a document, and a message that takes three paragraphs to say "fixed the cache" is a
document nobody reads. One header, an optional body of at most ten lines, nothing else:

```
type(scope): subject
```

`type` is one of `feat`, `fix`, `docs`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`,
`revert`. `scope` is optional and names the module or command (`cache`, `dsh`, `shape`). The
header is at most 72 characters, starts lowercase, and carries no trailing period. The body says
why, in plain sentences, one point per line; it does not narrate the search, and it does not
repeat the diff.

Merge, revert, and `fixup!` commits are exempt. A hook enforces this at write time:

```powershell
git config core.hooksPath .githooks
```

The same check runs in CI on every pushed commit, so a machine without the hook still cannot land a
message that does not conform. The rules are deliberately mechanical: a format is only useful if it
does not need a judgement call.
