# engine

A compiler for structured thinking. A `.mf` document is a set of typed declarations — facts,
concepts, propositions, hypotheses, questions, blind spots, values, warrants, laws, layers,
notes — and the compiler decides whether each one can be checked at all, what it still owes,
and what follows from it.

It does not judge truth. It judges whether a claim is checkable, and it prints the smallest
edit that would make it so.

## Run it

```powershell
cd rust/engine-core
cargo build --release
.\target\release\engine-core.exe check ..\..\..\metaform\obligo\hello.mf
```

The binary is 0.6 MB, links no runtime beyond the OS, and needs no interpreter:

```powershell
.\target\release\engine-core.exe --help
```

`check`, `engine`, `suggest` answer `--json` for machines; every other command prints text.

## Test it

```powershell
cargo test
```

Two suites: `tests/semantics.rs` states the properties (an attack defeats its target, an
ungrounded premise makes a conclusion conditional), and `tests/golden.rs` pins the exact text
of every command against six fixture documents. `UPDATE_GOLDEN=1 cargo test` rewrites the
snapshots; read the diff before committing.

## The documents

```
fact F_TRACE { claim: "the trace shows 14ms frames", evidence: tool("trace") }
concept C_FAST { name: "fast", grounded_by: [F_TRACE], against: [F_SLOW] }
warrant W_FAST { claim: "14ms is fast", from: [F_TRACE], to: P_FAST, basis: [V_BUDGET] }
```

Eleven kinds, each with a decidable obligation: a fact owes evidence someone else can check, a
concept owes a counterexample, a proposition owes a rule, a hypothesis owes a test or a rival, a
value owes an instrument, a warrant owes a basis.

`engine-core build` turns a checked document into `OBLIGATIONS.md` (what is still owed),
`LEARNED.md` (the clause each concept's examples support), `OPEN.md` (holes, questions, blind
spots), `NOTES.md`, and `predicates.py` (the learned clauses as runnable functions, verified
against every example).

## Layout

```
rust/engine-core/src     the compiler: one module per question it answers
rust/engine-core/tests   properties and golden snapshots
scripts/diff_text.py     the record of the port, against obligo v1
PROTOCOL.md              the contract: commands, JSON, directories, deliberate differences
```

`obligo` (tag `v1`) holds the frozen Python implementation and the research behind it. It is
history, not a dependency.
