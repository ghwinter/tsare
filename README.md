# tsare

**An assisted reasoning engine for LLMs: a commitment compiles, and its structure has a
deterministic answer.** Claims are written in a small typed language; the compiler says whether a
claim can be checked at all, what it still owes, and what to ask next.

It does not make a claim true, and it does not generate ideas. It holds the structure a model
cannot hold, and makes "this thought is not a hallucination" mean something mechanical: every
assertion has its ground on the page, every inference has a rule, every distinction has an
attribute that can tell the two things apart.

A `.tsare` document is a set of typed declarations — facts, concepts, propositions, hypotheses,
questions, blind spots, values, warrants, laws, layers, partitions, notes — and the compiler
decides whether each one can be checked at all, what it still owes, and what follows from it.

Truth is about the world; warrant is about the page. tsare checks the page. A false claim with a
traceable ground is an error you can find; a true claim with no ground is a hallucination you
cannot — that is the distinction it exists to make.

```powershell
$ tsare check SELF.tsare

SELF.tsare:13:13: warning[W-UNCONFIRMED]: fact F_ENUM_FORM was supplied by a model, not by the user or a tool
 13 |   evidence: model
    |             ^^^^^
```

## Why this exists

A model predicts the next token. It has no structure of its own to hold a commitment in: what it
asserted forty turns ago, which number it invented, whether "it works" was ever run. In a long
session that shows up as drift -- a definition that quietly changes, a plan that loses its
constraints, a status that was never checked.

tsare is the structure the model does not have, kept outside it. Commitments are written as
declarations, and a deterministic binary -- no model, no network, no clock -- says what each one
still owes. The model proposes, the binary verifies, the human confirms.

It is for the work where the model's own context cannot be trusted to hold: long-running sessions,
complex tasks, and anything creative enough that "sounds right" is the failure mode.

## The loop

tsare is not a linter at the end of a turn; it is the state a reasoning loop runs on. The unit is
not the answer, it is the claim:

```
claim -> check -> ask -> answer -> retire
```

- **claim**: write it where the gate reads it, `.tsare/session.tsare`, with its evidence and its
  scope; one line is enough while a claim is single.
- **check**: it is admitted only if it is anchored; errors stop it, warnings are named, not hidden.
- **ask**: `tsare ask --one` names the next question the document needs — the attribute that would
  settle an open pair, the instrument the claim is missing.
- **answer**: a measurement, a document, a person. The answer is another fact, with a grade.
- **retire**: `supersedes` withdraws what the answer overtook, and `because:` says what made the
  withdrawal right.

A turn that adds no checkable claim, answers no open question, and retires nothing is a no-op
turn. The ledger shows it; the model's prose does not.

## Use it from an agent

The point is not that a human types `.tsare` files. It is that an agent, before it asserts
something, writes the claim as code and has a deterministic checker say whether the claim is
anchored, whether a rule connects it to its support, and what is missing.

```powershell
tsare check .tsare/session.tsare   # what is wrong with it
tsare ask .tsare/session.tsare --one   # the next question the document needs
tsare fix .tsare/session.tsare     # the smallest edits that would fix it
tsare --explain E-VACUOUS          # what a diagnostic means
```

The binary is enough for any agent; the harness only decides when it is called. `dsh/tsare-gate`
installs three seams in DeepSeek Harness: a standing line with the next question, one reminder
after a verification command, and a turn-stopping gate that reads the ledger, not the prose. The
gate is a brake; `ask` is the engine.

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

```tsare
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
tsare check docs\REPO.tsare             # 0 error(s), 0 warning(s)
tsare engine docs\REPO.tsare            # held, conditional, blocked
Get-Content SELF.tsare | tsare check -  # a document may arrive on stdin
```

`check`, `engine` and `suggest` answer `--json` for machines; every other command prints text.
Exit codes: `0` no errors, `1` errors found (or a file that is not there), `2` a usage problem.

## What it can answer

| question | command |
| --- | --- |
| is every declaration well formed, and does every reference resolve? | `check` |
| which conclusions survive attack, and which reach a fact? | `argue`, `derive`, `engine` |
| which facts gate a conclusion, and how much external work does the document demand? | `debt`, `cost` |
| what does a concept's examples share, and what do the attributes imply? | `fca` |
| which attribute would decide the elements an open list does not cover? | `ask` |
| is this new predicate a distinction, or a recombination of what is already recorded? | `novel` |
| which laws does the attribute table already hold, and where does one run backwards? | `imply` |
| which diagnostics have a machine repair, and which need a value only the author can choose? | `fix` |
| are two documents the same argument? | `map` |
| did the last edit remove complaints, or trade them for others? | `delta` |
| what does the checked document oblige, and what did it learn? | `build` |

Twelve kinds, each with a decidable obligation: a fact owes evidence someone else can check, a
concept owes a counterexample, a proposition owes a rule, a hypothesis owes a test or a rival,
a value owes an instrument, a warrant owes a basis, a partition owes a dimension and a stated
closure.

## What it is not

Not classical inductive logic programming. The learner searches attribute literals and edge
paths, so a rule like
`grandparent(X,Z) :- parent(X,Y), parent(Y,Z)` cannot be written: there are no variables to
unify and no Horn clauses to resolve. What it does have is ILP's control loop — background,
positive and negative examples, a biased search, coverage — with the schema playing the part of
the mode and type declarations an ILP system must be given.

Not a theorem prover, and not a judge of truth. It decides whether a claim is checkable and
what would be needed to check it. A claim can pass every check and still be false; what it
cannot be is unsupported, unlicensed, indistinguishable, unclosed, or inconsistent without the
document saying so:

| failure | what the page shows |
| --- | --- |
| **unsupported** | a conclusion with no premise, a fact with no evidence, a number with no instrument |
| **unlicensed** | a model's word used as a fact, a self-verdict, a threshold with no measurement |
| **indistinguishable** | two things the recorded vocabulary cannot tell apart being treated as two |
| **unclosed** | a dangling reference, a cycle, a list that does not cover its extent |
| **inconsistent** | two claims that contradict, a layer that collapses, a tension with no priority |

## Tests

```powershell
cargo test --release
```

`tests/semantics.rs` states the properties (an attack defeats its target, a number cannot
exceed its support, a closed list is contradicted by its own attributes). `tests/golden.rs`
pins the exact text of every command against six fixture documents; `UPDATE_GOLDEN=1` rewrites
them, and the diff is the review. `SELF.tsare` is the repository describing itself, compiled by
the compiler it argues about; a test fails if it stops compiling cleanly.

## Layout

```
src/            the compiler: syntax, checks, reasons, analyses, repairs, output
tests/          properties, golden snapshots, fixtures
docs/           PROTOCOL.md (the machine contract) and REPO.tsare (the repository as declarations)
SELF.tsare         the same project described again, from a different angle; a test compiles it
```

Only what the build, the tests or the published package need is tracked. The rounds of
documents that built the language, the experiment harness and its recorded results, and the
agent integration notes are kept in the working copies, not in git.

## License

MIT.






