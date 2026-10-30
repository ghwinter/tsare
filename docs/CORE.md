# tsare — the core

A compiler keeps a small core: a term language, a type system, and a judgment that decides
whether a term is well typed. Everything in front of it is elaboration, and the core re-checks
what elaboration produced. Lean is the same shape at a larger scale: an elaborator that may be
wrong, and a kernel (`infer`, `whnf`, `def_eq`) that may not.

tsare has this shape to reach. This file states the core: one judgment, one former, six
questions, one order, one graph, one fixed point, one decision. Everything a command does that is
not in this file is a **view**: it may propose, it may not admit. The rules are the target; the
last section says which parts of the tree already keep them.

## Four words

| word | what it means | who answers it |
| --- | --- | --- |
| admissible | no rule reports an error | the checker |
| checkable | every debt names something that could discharge it | the checker |
| checked | every debt is discharged | the checker |
| true | the claim matches the world | not tsare |

`checkable` is the predicate the tool exists for. A document can be admissible and not checkable:
it compiles, and it still owes. `checked` is the stronger claim that the answers are in. Neither
is a claim about truth.

## The judgment

```
Γ ⊢ d : τ ! O
```

- `Γ` is the ledger: a sequence of scopes, each a set of declarations visible here.
- `d` is a declaration, the only term former.
- `τ` is its kind, checked against the kind's signature.
- `O` is its obligation row: what `d` still owes to someone who is not the model.

The judgment has three readings:

- `admit(Γ, d)` — may `d` enter `Γ`?
- `nf(Γ)` — the normal form of the ledger: grounded, defeated, retired.
- `decide(Γ)` — `closed`, `closable`, or `open`.

`admit` is not compositional. R5 and R6 read the whole ledger, so the judgment is a function of
`(Γ, d)`, not of `d` alone.

## One former, twelve shapes

A declaration is `claim + witness + scope`. The kind names the **shape of the witness**, not a
separate judgment. `note` is the unit: it has no witness and is not a commitment.

| kind | the claim is | the witness is | a witness is |
| --- | --- | --- | --- |
| fact | a state of affairs | evidence | a command, a document, or a person |
| concept | a distinction | an example pair | one positive and one negative |
| proposition | a conditional | a rule | premises, a conclusion, a warrant |
| hypothesis | a rival | a test | the observation that would refute it |
| question | an open choice | a split | two answers that separate two live hypotheses |
| blindspot | a known gap | a witness pair | the pair, the vocabulary, and the closing question |
| value | a quantity | an instrument | a command and a threshold |
| warrant | a rule | a basis | what makes the rule acceptable |
| law | a principle | a checker | the thing that would break if it failed |
| layer | an interface | a contract | what others may use, and nothing else |
| partition | a classification | a dimension | the concept the elements share, or a stated lack |
| note | — | — | nothing |

The table is the test of the claim: a kind whose row does not fit is a seventh question, not a
twelfth special case.

## The six questions

Every declaration is asked six questions against the ledger it enters. A "no" is an **error** (the
declaration may not enter) or a **debt** (it enters, and the ledger records what it owes). The
letter of a code says which.

1. **Shape.** A known kind; every required field present; every field belongs to the kind; every
   value has the field's type; every reference resolves to a declaration of the expected kind;
   names are unique; a required string is not blank; a document declares something.
2. **Witness.** The witness the kind names is present, and its grade is at least the threshold the
   kind and the use demand. A verdict — a hypothesis marked supported or refuted — cannot stand on
   a model's word; a fact can, and owes a confirmation.
3. **Support.** The declaration's support edges reach a leaf whose grade is `confirmed` or
   `measured`, with no cycle. A hole closes nothing.
4. **Debt.** Every obligation the kind generates names something that could discharge it: a
   command, a document, a person, a question, or a declaration that is itself closable.
5. **Distinction.** No grounded declaration attacks another; two live claims are not one claim
   under two names; a question separates the hypotheses it claims to separate; a declared order
   (parent, refinement, dimension) holds in the extents, not only in the names.
6. **Visibility.** A name is used only where it is visible: not across a layer boundary that does
   not expose it, not after it was retired, not against a kernel name. A claim inside a layer the
   kernel governs stands on one of that layer's laws.

The rules partition the codes the checker can emit:

```
R1 shape        E-SYNTAX E-VERSION E-EMPTY-DOCUMENT E-EMPTY E-KIND-UNKNOWN E-FIELD
                E-MISSING E-TYPE E-DUP-FIELD E-DUP-ID E-KIND E-UNDEFINED E-NOTE-REF
                E-WITNESS E-WARRANT-MODE E-NO-ANSWERS E-NO-ASSUMPTION E-NO-SIGNATURE
                E-NO-PREMISE E-PARTITION-EMPTY E-PARTITION-DUPLICATE E-PARTITION-CLOSURE
                E-CONFIDENCE E-INCOMMENSURABLE
R2 witness      E-SELF-VERDICT E-NO-EVIDENCE E-SCOPE-UNBOUNDED E-OUT-MISMATCH
                W-HOLE W-UNCONFIRMED W-UNSATISFIED W-BARE-NUMBER W-OVERCLAIM
                W-BASIS-NO-CRITERION W-UNWARRANTED-BASIS W-LAW-UNCHECKED
                W-PARTITION-CLOSED-UNJUSTIFIED W-RETIRED-UNBASED W-CONFIDENCE-MODEL
                W-CONFIDENCE-ASSERTED W-CONFIDENCE-INFLATED W-OUT-UNVERIFIED
R3 support      E-UNGROUNDED E-CYCLE E-VACUOUS E-UNCOVERED W-UNWARRANTED W-CONDITIONAL W-OPEN
R4 debt         E-UNFALSIFIABLE E-PARTITION-UNKNOWN-NO-EXIT E-PARTITION-UNKNOWN-USED
                W-UNTESTED W-NO-EXIT W-PARTITION-UNKNOWN W-PARTITION-OPEN-USED
R5 distinction  E-COLLAPSE E-CONTRADICTION E-TENSION E-HIERARCHY E-FALSE-BLINDSPOT
                E-NON-DISCRIMINATING E-PARTITION-ELEMENT E-PARTITION-CLOSED-CONTRADICTED
                W-COLLIDE W-COARSE W-EXAMPLE-SHAPE W-PARTITION-UNLEARNED W-UNANSWERED
                W-BY-MISMATCH
R6 visibility   E-LAYER-LEAK E-SUPERSEDE-CYCLE E-SUPERSEDE-UNDEFINED E-KERNEL-SHADOW
                W-EXPOSE-FOREIGN W-UNPRINCIPLED W-UNREFINED W-RETIRED
view            W-UNREAD
```

Every code belongs to exactly one rule. A code that reports advice rather than a broken
commitment would be a view code; today there is one, and it is marked. The README's five failure
classes are the same questions from the other side: unlicensed is R1, unsupported is R2,
unclosed is R3 and R4, indistinguishable is R5; R6 is the time dimension the README states
separately as retirement.

## The one order

`grade` is the only order the core computes:

| grade | the witness is | who can produce it again |
| --- | --- | --- |
| asserted | nothing named | nobody |
| unconfirmed | a model's word | only the model |
| confirmed | a person, an observation | the person |
| measured | a command, a document, a program | a machine |

`measured > confirmed > unconfirmed > asserted`. The threshold is a property of the kind and the
use, not of the document: a verdict cannot stand on a model's word, a fact can.

Every other order is **declared** by the document and **checked**, not computed: a concept's
parent, a law's refinement, a partition's dimension, a withdrawal. The core never invents an order
over claims; it checks the order the document claims.

## The one graph

A reference field has exactly one role:

| role | the edge means | read by |
| --- | --- | --- |
| types | the reference is a type constraint | R1 |
| grounds | the claim stands on it | the fixed point |
| attacks | the claim contradicts it | the fixed point |
| retires | the claim withdraws it | the fixed point |
| orders | the reference declares an order or a visibility | R5, R6 |

The fixed point reads grounds, attacks and retires; `types` and `orders` are checked, not
navigated. One relation matters because two readings of it can disagree. `derive` once grounded a
conclusion in a fact that `supersedes` had withdrawn, while `engine` did not; a regression test
keeps the case. Today the assignment of role to field is a column the signature table does not
have, and `impact` and `egraph` read field names of their own.

## The fixed point

```
G0 = { d : d is a fact, grade(d) ≥ confirmed, d not retired }
G  = least fixed point of: d ∈ G  iff  support(d) ⊆ G, no attacker of d in G, d not retired
```

`grounded = G`, `defeated = attacked by G`, `undecided = the rest`. An attack is an edge; it does
not matter whether a person declared it or a rule derived it.

Retirement is not deletion: a retired declaration stays visible to the reader and counts for
nothing. That is what lets withdrawal be an append instead of an edit.

## The decision

```
closed    := no debt, and every grade is at least what the kind demands
closable  := every debt names a discharger
open      := some debt names none
```

A discharger is a command, a document, a person, a question, or a declaration that is itself
closable. The recursion is well-founded because a leaf is a command, a document, a person or a
question; a cycle with no leaf is `open`.

`decide` is a function of `Γ` alone, so it can be the first thing a report says. The target is
`check`: its first line, and `decision` in `--json`, because it is the one thing a caller needs
before reading a list of diagnostics. Until that lands — it changes every check golden — a
`decide` command answers it.

## Trusted base

The kernel is: the parser, the kind signatures, the grade order, the obligation table, R1–R6, the
fixed point, and `decide`. No model, no network, no clock, no randomness.

The views are every command that proposes or explains: `abstract`, `ask`, `audit`, `confront`,
`cost`, `coverage`, `debt`, `delta`, `derive`, `egraph`, `fca`, `graph`, `hint`, `impact`,
`induce`, `map`, `meet`, `moves`, `next`, `novel`, `outline`, `pattern`, `shape`, `weigh`,
`whatif`, and `build`. A view may propose a declaration, an attack, a repair, or a question; it
may not admit one. `propose::verify` is the existing form of this rule for a single field: a
model may fill a hole, and the document must not get worse.

Two disciplines make the split real:

1. **Round trip.** Whatever a view writes must parse and re-check to the declarations it scored.
   A renderer that changes the meaning of a document is a kernel bug, not a formatting bug.
2. **One way.** A view may not report a failure that has no rule. If a view needs to reject a
   document, the rule goes in the kernel first, with a code and an entry in `explain`.

The gate in `dsh/` is not part of the core. It is how a session is made to write the ledger; the
core judges what was written. `check/kernel.rs` is not this kernel either: it is the kernel
*document* feature, a target checked against a document of laws.

## What the core does not judge

- **truth.** There is no oracle in the binary. A measured fact is a command that can produce the
  value again, not the value.
- **causation.** A causal claim is a declaration like any other; the core checks that it is
  anchored, not that it is causal.
- **value.** A priority, a budget, a preference is a person's; `confirmed` is where a person's
  word enters, and it does not become `measured`.
- **reference.** A `doc()` path is a name until something resolves it; the core checks that the
  name is written, not that it points at the thing.
- **whether a witness ran.** `measured` is a promise that a command can produce the value again.
  `out` and `artifact` are where a result is pinned; without them the promise is unkept, and the
  core says so.
- **whether the model is honest.** The ledger records what a declaration says. A form can be
  gamed; the defenses are the cost of the debt and the person who confirms.

## What is not yet so

The rules above are the target. The tree today:

- **D1.** No `admit`. `check` is the absolute judgment, `propose::verify` a relative one,
  `closure` the fixed point. `decide` answers `closable`; `check` does not carry the decision yet.
- **D2.** The kind is a string. R1–R6 are not the structure of `check`; it is a sequence of
  passes, and a new kind or a new rule lands in several of them.
- **D3.** The codes are not assigned to rules. A short list is interned; the rest are minted where
  they are reported. This file makes the assignment; nothing enforces it yet.
- **D4.** The graph is a flat list of reference keys. The fixed point reads a handful; `impact`
  and `egraph` re-derive their own.
- **D5.** There is no conversion. `collide`, `egraph`, `fca`, `shape` and `meet` each compute a
  notion of "the same"; R5 has no single relation to read.
- **D6.** There is no context calculus. `scope`, `layer`, `supersedes`, `--against`, `--holds`
  and the kernel document are six features, not one rule for what is visible.
- **D7.** Holes are a warning and a string in `obligations`, not one named row per hole.
- **D8.** Derived attacks are separate diagnostics; they do not enter the fixed point.
- **D9.** Round trip is not enforced. `fix --apply` and `propose --apply` check the candidate
  before writing it; nothing re-checks the bytes that were written.

The order of work follows the dependencies: D3 before D1 (a judgment needs its rules named), D1
before D2 (the judgment first, the structure of the checker second), D4 before D5, and D6 and D9
last because they change behavior rather than vocabulary.
