# Contributing

## What this is

A deterministic checker: same input, same output, no model inside. Anything that needs a model
stays outside the binary.

## Before a pull request

```powershell
cargo fmt --all --check
cargo clippy --all-targets -- -D warnings
cargo test --release
cargo run --release --quiet -- check SELF.mf docs/REPO.mf
```

## Rules the code keeps

1. No model call, no network, no clock, no randomness in `src/`. A check that cannot be decided
   says so instead of guessing.
2. Every diagnostic code has an entry in `src/explain.rs`; `tests/semantics.rs` fails if one
   does not.
3. A new check needs the document that would have caught it: add it to `tests/fixtures/` or
   `docs/rounds/`.
4. Output changes are golden changes. Run `UPDATE_GOLDEN=1 cargo test`, then read the diff
   before committing it.
5. Negative results stay in the repository. If the compiler refuted a hypothesis, the
   refutation is part of the result.
