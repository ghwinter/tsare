# engine

**An external reasoning engine for AI agents.** It turns a claim into a small program and
answers two questions a model cannot answer about itself: *can this be checked at all*, and
*what is the smallest edit that would make it so*.

A `.mf` document is a set of typed declarations — facts,
concepts, propositions, hypotheses, questions, blind spots, values, warrants, laws, layers,
partitions, notes — and the compiler decides whether each one can be checked at all, what it
still owes, and what follows from it.

It does not judge truth. It judges whether a claim is checkable, and prints the smallest edit
that would make it so.

```powershell
$ tsare check SELF.mf

SELF.mf:13:13: warning[W-UNCONFIRMED]: fact F_ENUM_FORM was supplied by a model, not by the user or a tool
 13 |   evidence: model
    |             ^^^^^
```

## Use it from an agent

The point is not that a human types `.mf` files. It is that an agent, before it asserts
something, can write the claim as code and have a deterministic checker say whether the claim
is anchored, whether a rule connects it to its support, and what is missing.

`agent/AGENTS.snippet.md` is a block to paste into your agent instructions. The loop it
installs is one line long: write the claim, compile it, report only what compiles, and report
the refusals with the corrected version.

```powershell
tsare check claim.mf        # what is wrong with it
tsare fix claim.mf          # the smallest edits that would fix it
tsare --explain E-VACUOUS   # what a diagnostic means
```

Nothing model-shaped is inside the binary: proposal is the agent's job, verification is the
binary's, and the two meet at `propose` (three verdicts, three policies) and at a human who
confirms.

## Install

```powershell
cargo install --path .
tsare --help
```

One binary, no dependencies, no runtime beyond the OS. `tsare --version` reports the
release; `--explain CODE` explains any diagnostic it prints.

## First document

```mf
fact F_TRACE {
  claim: "the trace shows 14ms frames",
  evidence: tool("trace")
}

value V_BUDGET {
  metric: "frame_ms", direction: max, unit: "ms", threshold: 16, measured_by: "trace"
}

proposition P_FAST {
  statement: "frames land inside the budget",
  from: [F_TRACE]
}

warrant W_FAST {
  claim: "14ms is inside the 16ms budget",
  from: [F_TRACE],
  to: P_FAST,
  basis: [V_BUDGET]
}
```

```powershell
tsare check examples\hello.mf      # 0 error(s), 0 warning(s)
tsare engine examples\inquiry.mf   # held, conditional, blocked
Get-Content SELF.mf | tsare check -   # a document may arrive on stdin
```

`check`, `engine` and `suggest` answer `--json` for machines; every other command prints text.
Exit codes: `0` no errors, `1` errors found or a proposal refused, `2` a usage or file problem.

## What it can answer

| question | command |
| --- | --- |
| is every declaration well formed, and does every reference resolve? | `check` |
| which conclusions survive attack, and which reach a fact? | `argue`, `derive`, `engine` |
| which facts gate a conclusion, and how much external work does the document demand? | `debt`, `cost` |
| what does a concept's examples share, and what do the attributes imply? | `fca` |
| which diagnostics have a machine repair, and which need a value only the author can choose? | `fix` |
| are two documents the same argument? | `map` |
| what does the checked document oblige, and what did it learn? | `build` |

Twelve kinds, each with a decidable obligation: a fact owes evidence someone else can check, a
concept owes a counterexample, a proposition owes a rule, a hypothesis owes a test or a rival,
a value owes an instrument, a warrant owes a basis, a partition owes a dimension and a stated
closure.

## What it is not

Not classical inductive logic programming. The learner searches attribute literals and edge
paths and stops at a three-literal conjunction, so a rule like
`grandparent(X,Z) :- parent(X,Y), parent(Y,Z)` cannot be written: there are no variables to
unify and no Horn clauses to resolve. What it does have is ILP's control loop — background,
positive and negative examples, a biased search, coverage — with the schema playing the part of
the mode and type declarations an ILP system must be given.

Not a theorem prover, and not a judge of truth. It decides whether a claim is checkable and
what would be needed to check it.

## Tests

```powershell
cargo test --release
```

`tests/semantics.rs` states the properties (an attack defeats its target, a number cannot
exceed its support, a closed list is contradicted by its own attributes). `tests/golden.rs`
pins the exact text of every command against six fixture documents; `UPDATE_GOLDEN=1` rewrites
them, and the diff is the review. `SELF.mf` is the repository describing itself, compiled by
the compiler it argues about; a test fails if it stops compiling cleanly.

## Layout

```
src/            the compiler: syntax, checks, reasons, analyses, repairs, output
tests/          properties and golden snapshots
docs/           PROTOCOL.md (the contract) and the ten rounds that built it
examples/       documents to run first
experiments/    the hole-filling harness and its recorded results
scripts/        the differential against the frozen reference implementation
SELF.mf         the repository, written as declarations
```

## License

MIT.





