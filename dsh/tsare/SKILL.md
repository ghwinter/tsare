---
name: tsare
description: An external reasoning engine for an LLM that has no reasoning structure of its own. A commitment is written as a .tsare document and a deterministic binary says whether it can be checked at all, what it still owes, and whether the recorded vocabulary can tell two objects apart. No model inside.
---

# tsare

A model predicts the next token; it has no structure of its own to hold a commitment in. tsare is
that structure, kept outside it, for long sessions and complex tasks where "sounds right" is the
failure mode. It does not make a claim true. It says whether the claim can be checked, and what it
still owes.

Use it when a claim, a number, a plan or a design decision is about to be asserted and a
reader will rely on it. Four cases, not every sentence:

1. you are about to conclude something;
2. you are about to give a number;
3. you are about to say "it works";
4. you are choosing between options.

## The loop

Write the claim as declarations in `.tsare/session.tsare` — the file the gate checks at turn
stopping. One line is enough while a claim is single:

    fact F_TRACE { claim: "the trace shows 14ms", evidence: tool("trace, 200 runs") }

A chain adds the rule that connects it:

    fact F_TRACE { claim: "the trace shows 14ms", evidence: tool("trace, 200 runs") }
    value V_BUDGET { metric: "frame_ms", direction: max, unit: "ms", threshold: 16, measured_by: "trace" }
    proposition P_FAST { statement: "frames land in budget", from: [F_TRACE] }
    warrant W_FAST { claim: "16ms is the budget", from: [F_TRACE], to: P_FAST, basis: [V_BUDGET] }

Then:

    tsare check .tsare/session.tsare        # errors block; warnings must be reported, not hidden
    tsare ask .tsare/session.tsare --one    # the next question the document needs
    tsare novel .tsare/session.tsare --attribute X --holds F_A,F_B   # is X new, redundant, or a recombination?
    tsare imply .tsare/session.tsare        # the laws the attribute table already holds
    tsare fix .tsare/session.tsare          # the smallest edits that would make it checkable
    tsare --explain CODE                    # what a diagnostic means

## Rules

- Report a claim only after it compiles with no error. Name the warnings that stand.
- If the checker refuses, report the refusal and the corrected version together. Never silently
  rewrite into something that passes.
- `evidence: model` without `confirmed_by` is a warning you must pass on: the claim came from a
  model, not from a tool or a person.
- Never invent a kind, a field or a code: `tsare kinds`, `tsare --explain`.
- A number needs an instrument, a universal claim needs a scope, a proposal needs evidence.
- If a claim cannot be checked, do not delete it to pass: record the gap and say `unchecked` with
  the reason in the answer. The gate reads the ledger, not the prose, and a thin ledger passes --
  the escape is honest, the silence is not.

## Where it is

Build it with `cargo build --release` and run `target/release/tsare` from the checkout, or install
it with `cargo install --path .` and call `tsare`. Every path in this repository is relative to its
own root: no document names a machine, a drive or a home directory.
