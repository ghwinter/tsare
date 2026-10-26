# tsare — one implementation, one contract

This repository is the compiler. The crate at the root builds a single, dependency-free
binary: no interpreter, no runtime beyond the OS, no Python, no second code path to keep in
step.

`obligo` (tag `v1`) is the frozen reference: it holds the original Python implementation and
every experiment behind it. It is no longer the live compiler. The differential script that
compared the two stays in the working copies rather than in this tree, and its output is the
record of the port.

## Two numbers, and only two

| number | where it lives | what it promises | when it moves |
| --- | --- | --- | --- |
| **format** | the first line of a document, `tsare <n>`; absent means `1` | the grammar: what a document is allowed to say | only when the grammar changes, and only with a migration |
| **crate** | `Cargo.toml`, `tsare --version` | behaviour: which checks, commands and outputs exist | semver, below |

The format number is the one that must not inflate. It is written in every document, so a new
number means every document needs rewriting. It stays `1` while `2` does not exist, and the
line may be omitted -- which is why the documents written before the line existed still compile
unchanged. A document that declares a format this build does not speak gets `E-VERSION` and
stops: one true error instead of a page of parse noise.

The crate number may move as often as the work needs:

- **major** -- a document that compiled now compiles differently, a command's output changes
  shape, or an existing diagnostic moves or changes meaning. Before `1.0.0` this may happen
  without notice; after it, this is the only thing permitted to break a promise.
- **minor** -- a new check, command or field. Adding a diagnostic is minor: no document becomes
  invalid, some documents gain a line of advice.
- **patch** -- nothing observable: a message, a fix, a speedup.

`1.0.0` ships when the **format** is frozen, not when the features are done. Features keep
arriving afterwards; what stops is the rewriting of documents. A version is not a date and not a
count of work: a number that rises because a week passed says nothing about what changed.

## Commands

Every command reads files and prints text. Where a command also has a machine contract it
answers `--json`.

| command | what it answers | JSON |
| --- | --- | --- |
| `check FILE\|DIR` | which declarations are well formed; `--goals`, `--sugar`, `--cache` | yes |
| `engine FILE\|DIR` | held, conditional, blocked, chains, gates | yes |
| `argue FILE` | which conclusions survive attack | |
| `derive FILE` | which conclusions are grounded in facts | |
| `debt FILE [--all]` | which facts transitively gate a conclusion | |
| `impact FILE` | dead facts, single support, load-bearing nodes | |
| `next FILE` | open decisions ranked by what they touch | |
| `ask FILE` | the recorded attribute that would decide an open list's missing elements | |
| `cost FILE` | external confirmations the document demands, and its longest chain | |
| `weigh FILE` | confidence propagated by min/max, with no probability | |
| `kinds` | the obligation table: what each kind owes | |
| `induce FILE` | the clause each concept's examples support, or why none exists | |
| `pattern FILE CONCEPT [--vars]` | the structure that separates a concept's examples | |
| `moves FILE` | six representation moves as concrete candidates | |
| `assumptions FILE` | load-bearing declarations without support | |
| `whatif FILE NAME` | what falls when one assumption is removed | |
| `graph FILE [--out]` | the typed relation net as Mermaid | |
| `map LEFT RIGHT` | structure mapping between two documents | |
| `audit FILE` | whether a kind can be written with a smaller vocabulary | |
| `egraph FILE [--write]` | equivalent declarations merged, references rewritten | |
| `cat FILE` | the pipeline read as a category, and which functors hold | |
| `project DIR` | a directory checked as one program, grouped by file | |
| `fix FILE [--apply]` | verified repair candidates, and applying them | |
| `suggest FILE` | repairs with an applicability | yes |
| `apply FILE [--dry-run]` | machine repairs spliced and recompiled | |
| `build FILE\|DIR [--out] [--goals]` | obligations, learned clauses, open items, notes, predicates | |
| `mem [--recall CODE...] [--k N]` | the case store: audit it, or recall similar situations | |
| `propose FILE --proposals FILE.json [--policy ...]` | gate values an outside model wants to fill into holes | |
| `fca FILE` | what a concept's positives share, and which facts the attributes imply | |

`suggest` and `apply` are the engine's own shape: `apply` splices a span and keeps the result
only if the document compiles better.

## The JSON contract

### `check --json FILE|DIR`

```json
{
  "diagnostics": [
    {"code": "E-UNDEFINED", "severity": "error", "file": "02-claims.mf",
     "line": 2, "col": 41, "message": "..."}
  ],
  "errors": 1, "warnings": 2
}
```

`file` is required. Attribution by parsing message text is not allowed.

### `engine --json FILE|DIR`

```json
{
  "held": ["P_SMOOTH"],
  "conditional": ["P_B"],
  "blocked": [{"warrant": "W_LINK", "missing": ["P_A"]}],
  "chains": {"P_SMOOTH": "P_SMOOTH <- W_SMOOTH <- F_TRACE"},
  "gates": {"P_SMOOTH": []}
}
```

### `suggest --json FILE`

```json
{
  "suggestions": [
    {"applicability": "MachineApplicable", "message": "drop the duplicate",
     "file": "a.mf", "line": 19, "replacement": "..."}
  ]
}
```

Applicability is one of `MachineApplicable`, `MaybeIncorrect`, `HasPlaceholders`,
`Unspecified` (order is strictness).

A suggestion names a `span` and a `replacement` for exactly that span. A deletion is an empty
replacement over the removed declaration. `span` may be `null` when the repair inserts
something with no existing declaration to replace.

## Directories

Every command accepts a file or a directory. A directory is checked as one program:

- names are global; a duplicate is reported with both file names;
- cross-file references resolve;
- layer visibility is computed once over the merged program;
- a diagnostic is attributed by its span, matching a declaration's span exactly. Attribution
  never inspects the message text.

## Modules

One crate, grouped by what a module needs:

| tier | modules |
| --- | --- |
| syntax | `syntax`, `specs`, `sugar` |
| check | `check`, `coverage`, `project` (via `cli`) |
| reasoning | `closure`, `argue`, `derive`, `debt`, `engine`, `fix`, `apply` |
| analysis | `impact`, `cost`, `weigh`, `cat`, `moves`, `assumptions`, `egraph`, `whatif`, `audit`, `map` (`mapping`), `graph`, `induce`, `paths`, `patterns` |
| emit | `build` |
| stores | `cache`, `cases`, `json`, `hash` |
| front | `cli`, `program` |

`check` calls `closure` for the conditional-conclusion warning, so the check tier depends on
the reasoning tier in exactly one place. Keep it that way.

## Checks added after v1

Each of these follows one rule: a property that can be derived from the document must be
checked against what was declared.

| code | what it compares |
| --- | --- |
| `E-TYPE` on list elements | every element must have the element type, not just be inside a list |
| `W-EXAMPLE-SHAPE` | an example must share a recorded attribute with the concept's positives, or it cannot be compared with them |
| `W-BY-MISMATCH` | an inference type is derived from structure — a warrant makes deduction, concepts without a rule make induction — and a declaration that disagrees is contradicted |
| `W-CONFIDENCE-INFLATED` | a declared number is bounded by the weakest link: the rule's premises, or the premises a rule-less proposition cites |
| `E-PARTITION-*` | a closed list is contradicted by the dimension's extent and by any fact the attributes imply |
| `W-PARTITION-UNKNOWN` | a pile with no dimension may be written and may not discharge anything |

## Gatekeeping values from outside

`propose` takes a declaration, a field, and a value, and answers one of three ways:

- **admitted** — no diagnostic was introduced and no error was added;
- **review** — no error, but a diagnostic appeared that was not there before;
- **rejected** — an error was added, the value did not parse, or nothing changed.

`--policy` decides how much a fill must earn: `harmless` (the three above), `strict` (an
introduced diagnostic is a rejection), `improve` (something besides the hole must get better).
The default is `harmless`. None of the three looks at content, and the experiments in
`experiments/hole-filling` measure what that costs.

## Deliberate differences from v1

These are decisions, not drift:

1. **Diagnostic wording and rendering.** The engine words its own messages and prints
   `CODE file:line message`; v1 printed rustc-style frames with hints. Codes, severities,
   lines and columns agree; prose does not.
2. **`project` attribution.** v1 guessed the owning file from the message text; the engine
   attributes by span.
3. **`E-LAYER-LEAK` span.** v1 pointed at the target declaration; the engine points at the
   declaration that reaches across layers, which is the one to edit. Line numbers differ.
4. **`apply` only applies `MachineApplicable`.** Both sides do this; the note is here because
   a "verified" repair that changes meaning is still a suggestion.
5. **`mem` salt.** The case store records which checker produced an entry. The engine's salt
   is its own, so entries written by v1 read as expired. That is what the field is for.
6. **`egraph` does not crash.** v1 raised `KeyError` on a document whose concept references a
   note; the engine merges what it can and reports the rest.
7. **Empty extents do not collide.** A concept with no positive example asserts nothing, so
   it is not reported as `E-COLLAPSE`.
8. **`partition` is new.** v1 had no obligation attached to an enumeration. The engine adds
   one: a list must name the concept its elements share, say whether it is closed, and give a
   reason when it is closed. The checks are `E-PARTITION-EMPTY`, `E-PARTITION-DUPLICATE`,
   `E-PARTITION-ELEMENT`, `E-PARTITION-CLOSED-CONTRADICTED`, `W-PARTITION-CLOSED-UNJUSTIFIED`
   and `W-PARTITION-OPEN-USED`.
9. **Two facts can collide.** A fact records `evidence` and `attrs`; two facts that record the
   same evidence and the same attributes are reported as `W-COLLIDE`, because nothing a machine
   can read tells them apart -- the difference lives only in prose. It is a warning, not an
   error: the tool does not decide that they must be merged, only that this vocabulary cannot
   separate them. A fact with no attributes never collides, since the attribute table then says
   nothing about it.

## What a partition does and does not check

```
partition E_REFUSE_NOW {
  of: Q_STAGE,
  dimension: C_REFUSE_NOW,
  elements: [F_NO_PROOF_LANGUAGE, F_MORE_ILP, F_ADVICE_PROSE],
  closure: closed,
  basis: [F_DECIDABILITY_BILL]
}
```

Checked: every element is an instance of the dimension (a concept records its extent, so this
is a set membership test); a closed list covers that extent; "closed" names a reason; an open
list is not silently used as a premise; a named dimension whose own examples no attribute
separates is reported, because a type whose examples collide is a name, not a type.

Not checked: whether the dimension is the right one, and whether the world contains a fourth
element. Those stay where they were — in the concept's counterexample and in the `basis`
declaration. The list's completeness is now a stated claim with a named reason instead of a
rhetorical effect, which is the whole of the improvement.

### A pile is not a partition

`closure` has three values, and exactly one of them may be true:

| closure | dimension | means | consequence |
| --- | --- | --- | --- |
| `closed` | required | these are all of them, and here is why | checked against the extent |
| `open` | required | these are some of them | cannot carry a conclusion without `W-PARTITION-OPEN-USED` |
| `unknown` | forbidden | I cannot name what they have in common yet | compiles, warns, and **cannot discharge anything** |

The third state is the error path: a list whose common property is not yet known can be
written down, and it must name the question that would type it (`closes_with`). Using it as a
premise is an error, not a warning — a hole cannot discharge anything. Naming a dimension and
admitting you have none at the same time is `E-PARTITION-CLOSURE`.

This is deliberate: refusing to compile the pile would teach the author to invent a dimension,
which is the failure the language exists to prevent. A declared hole is visible, unusable, and
attached to its own question.

A partition is not yet a premise in the fixpoint: `argue` and `derive` do not use one to
discharge anything. `SELF.mf` records that as a blind spot rather than guessing either way.

## Acceptance

```powershell
cd D:\GitHub\tsare   # or the checkout root
cargo test            # semantics + golden snapshots for every command
cargo build --release # the binary
```

`tests/semantics.rs` holds properties (`an_attack_defeats_its_target`). `tests/golden.rs` holds
the exact text of every command on six fixture documents, captured while the engine still
agreed with v1 byte for byte. `UPDATE_GOLDEN=1 cargo test` rewrites them; read the diff first.

The port itself is recorded by `scripts/diff_text.py`, which compares the binary with the
frozen Python in `obligo`: 760 comparisons across 32 documents, no mismatch, one document the
reference cannot process (item 6 above).


