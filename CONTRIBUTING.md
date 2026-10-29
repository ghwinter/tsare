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

## How the branches move

Three tiers, and only three:

| branch | holds | written by | moves |
| --- | --- | --- | --- |
| `main` | what is stable and absorbed | a person merges `dev`; nothing else | slowest: only after `dev` has held |
| `dev` | integration: several stable changes together | a merge from `nightly` when green | when a change is wanted |
| `nightly` | every experiment and every new line | direct commits | may be reset from `dev` at any time |

A change starts on `nightly`. When it is green and wanted it merges to `dev`. When `dev` has
held and still passes, a person merges it to `main`. Nothing is committed to `main` directly,
and `dev` takes nothing that has not been on `nightly`. `nightly` is expected to break: a red
run there is information, not an incident.

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

## What never enters the tree

The repository is read on machines it has never seen, so a fact about one machine is wrong
everywhere else and cannot be checked by anyone who was not there. The tracked set is
default-deny: a file is admitted only when the build, the tests or the published package need it.
Nothing else is committed, in particular:

- **a local fact** — a drive letter, an absolute path, a home directory, a hostname, a private IP,
  a port on one machine, a username, a temp directory, a line number from one checkout; and a
  sentence whose meaning lives only outside the repository (a private plan, a chat, a machine
  state), because a reader cannot check what they cannot see;
- **a secret, or anything shaped like one** — keys, tokens, passwords, private-key blocks,
  connection strings, `.env` files;
- **personal data** — an email address, a phone number, a name that is not already the public
  author of the project;
- **a record of a run** — `*.jsonl`, `*.log`, dumps, matrices, machine goldens, anything large
  enough to be an artifact rather than a source;
- **non-core code or a script that served one investigation** — it belongs in the message of the
  commit that used it, or in `experiments/` (untracked), not in the tree.

A path in a document is relative to the repository root. If a measurement needs the machine that
took it, the number goes in the document and the machine goes in the commit message.

Three gates enforce this in `.github/workflows/ci.yml`: `no machine paths`, `no secrets`, and
`no large tracked files` (256 KiB). They exist because `.gitignore` hides a file instead of
reporting it, and because a forced add or a merge from an old branch bypasses the ignore rule.

## Commit messages

The history is a document, and a message that takes three paragraphs to say "fixed the cache" is a
document nobody reads. One header, an optional body of at most ten lines, nothing else:

```
type(scope): subject
```

`type` is one of `feat`, `fix`, `docs`, `style`, `test`, `refactor`, `perf`, `build`, `ci`, `chore`,
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
