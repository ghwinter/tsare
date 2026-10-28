# tsare — one implementation, one contract

This repository is the compiler. The crate at the root builds a single, dependency-free
binary: no interpreter, no runtime beyond the OS, no second code path to keep in step.

The text is the contract. Every command's output is pinned byte for byte by
`tests/fixtures/golden/`, so a change to what the tool says appears as a diff before it ships.

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

## Exit codes

| code | meaning |
| --- | --- |
| `0` | the document is as checkable as it claims: no errors, and warnings are advice |
| `1` | an error, a file that is not there, or a warning under `--strict` |
| `2` | the invocation is wrong: it names no document, or names a command that does not exist |

`--strict` turns every warning into a gate. Use it where a document is meant to be finished: a
loop that reads only the number then cannot ignore `W-UNCONFIRMED`, which is the engine's way of
saying that a declaration rests on a model's word.

## Commands

Every command reads files and prints text. Where a command also has a machine contract it
answers `--json`.

| command | what it answers | JSON |
| --- | --- | --- |
| `check FILE\|DIR` | which declarations are well formed; `--goals`, `--sugar`, `--cache`, `--against OTHER`, `--strict` | yes |
| `engine FILE\|DIR` | held, conditional, blocked, chains, gates | yes |
| `argue FILE [--against OTHER]` | which conclusions survive attack; with `--against`, the attack graph is the merge | |
| `confront LEFT RIGHT` | which rules attack across the pair, and which attacks nobody answered; exit 1 when `LEFT` leaves one unanswered | |
| `derive FILE` | which conclusions are grounded in facts | |
| `debt FILE [--all]` | which facts transitively gate a conclusion | |
| `impact FILE` | dead facts, single support, load-bearing nodes | |
| `next FILE [--json]` | open decisions ranked by what they touch | yes |
| `ask FILE [--json] [--one]` | the next question the document needs: a gap in a list, two facts nothing tells apart, or a concept whose examples look alike; `--one` gives the question that settles the most | yes |
| `novel FILE --attribute N [--holds IDS]` | new, redundant, or a recombination: whether a proposed attribute is a distinction the recorded vocabulary already contains | |
| `shape FILE [--top N] [--depth D] [--layers] [--against OTHER [--near N]] [--json]` | the forms declarations share, at a depth; `--layers` the forms that cross a layer, `--against` the forms two documents differ in, `--near` the closest unshared forms and how close they are | yes |
| `meet FILE LEFT RIGHT` | the most general form two declarations share, and the fields where they differ | |
| `hint FILE [--json]` | the concept a shared form implies and the document does not declare: the free positions, whether anything names them, and the equation with them open | yes |
| `imply FILE [--json] [--max-antecedent N] [--min-support N]` | the laws the attribute table already holds, with support and a counterexample to the converse; bounded on purpose | yes |
| `abstract FILE [--min-support N] [--max-intent N]` | concept declarations the attribute table supports but nobody wrote: extent, closed intent, and the witness pair that separates them | |
| `cost FILE [--top N] [--json]` | external confirmations the document demands, and its longest chain | yes |
| `weigh FILE` | confidence propagated by min/max, with no probability | |
| `kinds` | the obligation table: what each kind owes | |
| `induce FILE` | the clause each concept's examples support, or why none exists | |
| `pattern FILE CONCEPT [--vars]` | the structure that separates a concept's examples | |
| `moves FILE` | six representation moves as concrete candidates | |
| `assumptions FILE` | load-bearing declarations without support | |
| `delta BEFORE AFTER [--record STORE --doc DOC [--action A]]` | what changed between two `check --json` reports, keyed by declaration; `--record` appends the change to a case store | yes |
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
| `fmt FILE` | the document in one spelling: one field per line, one separator everywhere, for a diff and a hash that mean something | |
| `outline FILE` | the document read as prose: each field quoted, its ground and its diagnostics in the margin | |
| `mem [--store FILE] [--recall CODE...] [--k N]` | the case store: audit it, or recall similar situations | |
| `propose FILE --proposals FILE.json [--policy ...]` | gate values an outside model wants to fill into holes | |
| `fca FILE` | what a concept's positives share, and which facts the attributes imply | |

`suggest` and `apply` are the engine's own shape: `apply` splices a span and keeps the result
only if the document compiles better.

## MCP

`tsare mcp` speaks JSON-RPC 2.0 over stdio and exposes nine tools: `check`, `argue`, `derive`,
`fix`, `explain`, `kinds`, `ask`, `novel`, `imply`. Each answers with the same text the command
line prints -- one renderer per command, so the two cannot drift -- and no tool calls a model.
`tests/mcp.rs` opens the server the way a client would and checks that every listed tool is
dispatched.

The server is not registered by a harness by default. Registration costs about 500 tokens of tool
schema in every turn; the skill in `dsh/tsare/SKILL.md` costs one catalog line, which is why it is
the cheaper way to keep the tool reachable.

## The JSON contract

### `check --json FILE|DIR`

```json
{
  "diagnostics": [
    {"code": "E-UNDEFINED", "severity": "error", "name": "P_RATE", "file": "02-claims.tsare",
     "line": 2, "col": 41, "message": "..."}
  ],
  "errors": 1, "warnings": 2
}
```

`file` is required. Attribution by parsing message text is not allowed.

`name` is the declaration the diagnostic is about, and it is the only field a diff can be keyed
by: a line moves whenever anything above it is edited, a name does not.  A diagnostic about the
file rather than about a declaration (a parse error) carries an empty name.  `delta` compares two
of these reports by `(name, code)` and says what went and what arrived.

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
     "file": "a.tsare",
     "span": {"line": 19, "col": 1, "end_line": 19, "end_col": 42},
     "replacement": ""}
  ]
}
```

Applicability is one of `MachineApplicable`, `MaybeIncorrect`, `HasPlaceholders`,
`Unspecified` (order is strictness).

A suggestion names a `span` and a `replacement` for exactly that span. `file` is always the
document the span indexes: for a single-file invocation it is the path the command was given,
and for a directory it is the file name inside it. `span` is an object with `line`, `col`,
`end_line` and `end_col`; it is never `null`, and a repair that inserts rather than replaces
uses a zero-width span at the insertion point. A deletion is an empty replacement over the
removed declaration.

### `delta --json BEFORE AFTER`

```json
{
  "before": 17, "after": 16,
  "gone": [{"name": "F_ENUM_FORM", "code": "W-UNCONFIRMED", "count": 1}],
  "appeared": []
}
```

Both files are the output of `check --json`. A complaint is identified by `(name, code)` and
counted, so a warning that moved down the file is not a change. The report says what changed, not
whether the change is an improvement: fewer warnings can also mean a document that stopped
demanding a confirmation, which is the opposite.

### `next`, `cost`, `shape`

```json
{"decisions": [{"label": "Q_GOAL (question)", "node": "Q_GOAL", "touches": 3}]}
{"total": 20, "depth": 4, "chain": ["F_A", "W_B"], "by_kind": [{"kind": "fact", "count": 13}], "rows": [{"name": "F_A", "needs": ["a measured value"]}]}
{"depth": 0, "total": 44, "forms": [{"kind": "fact", "signature": "...", "members": ["F_A"], "variables": [{"field": "evidence", "values": ["user"]}]}]}
```

`shape --layers` answers `{"depth": D, "fibres": [...]}`; `shape --against OTHER` answers
`{"depth": D, "shared": [...], "only_here": [...], "only_there": [...]}`. In both of those a name
is written `<kind> <name>`, joined by one space.

`--near N` adds `"near": [{"here": ..., "there": ..., "score": ...}]`: for each form only one
document has, the closest form only the other has, scored by the share of positions -- kind and
`field:shape` -- they both name. A score is an approximation, not an equivalence: two forms can
score `1.00` and still mean different things, which is why the exact lists above are not replaced.

`--top` shapes the text. `--json` carries every row, because a machine that asked for the data did
not ask for a sample.

## The case store

A repair can be remembered, and a later run can ask what worked. `delta --record STORE --doc DOC`
appends one line:

```json
{"doc": "claim.tsare", "codes": ["W-UNCONFIRMED"], "action": "model",
 "before": {"errors": 0, "warnings": 2}, "after": {"errors": 0, "warnings": 1},
 "checker": "tsare-0.2"}
```

Every field but `action` is read from the two reports. `action` is `mechanical`, `model`, `manual`
or `unspecified`, and no free text is accepted: a sentence written by the agent that made the edit
is the one thing the store cannot audit.

The store is one line per case, appended to and never rewritten, so a malformed line is skipped
instead of losing the memory around it. Nothing writes `.tsare/cases.jsonl` unless a command was
handed `--record`.

`mem --store FILE` audits a store: `expired` (written by another checker), `conflicting` (the same
situation and action, opposite outcome), `regressed` (the counts changed but did not improve),
`unchanged` (the counts are identical), and `orphans` (the document is gone; a relative `doc` is
resolved from the store's directory, not from the shell). `regressed` used to be printed as
`no-op`, which was the opposite of what the filter tests: a no-op is `unchanged`. `mem --recall CODE...`
ranks cases by how much of their code set overlaps the one given, improved cases first.

## The prelude

`prelude/` holds the documents this project admits, and one that it does not:

| file | what it is |
| --- | --- |
| `KERNEL.tsare` | the six principles a claim is held to, each backed by a fact in the document |
| `VIOLATION.tsare` | a claim that skips them: it redefines a name the kernel owns and sits in a governed layer citing none of its laws |
| `WORLDVIEW.tsare` | the categories, and the partition whose exhaustiveness is an open question |
| `PROCESS.tsare` | what makes a sentence sayable at all: a bound domain, a re-playable witness, a knowable limit |

These are **provisional the way an unstable ABI is provisional**. They are compiled in CI, and
`tests/prelude.rs` fails if the kernel stops refusing `VIOLATION.tsare` — so a revision has to
show up as a diff and a failing test rather than as a paragraph somewhere. The warnings they
carry are deliberate: `PROCESS.tsare` keeps a universal unbound because the only honest fix is to
weaken the sentence, and `KERNEL.tsare` over-claims about its own kind table because a document
cannot enumerate the language's own type system.

## Directories

Every command accepts a file or a directory. A directory is checked as one program:

- names are global; a duplicate is reported with both file names;
- cross-file references resolve;
- layer visibility is computed once over the merged program;
- a diagnostic is attributed by its span, matching a declaration's span exactly. Attribution
  never inspects the message text.

## Modules

One crate, grouped by what a module needs:

| directory | modules |
| --- | --- |
| `lang/` | `syntax`, `specs`, `program`, `sugar`, `render`, `hash`, `json`, `paths`, `explain` |
| `check/` | `check` and its checks, plus `fix` and `kernel` |
| `reason/` | `closure`, `argue`, `derive`, `debt`, `impact`, `cost`, `weigh`, `cat`, `moves`, `assumptions`, `egraph`, `whatif`, `audit`, `coverage`, `delta`, `fca`, `imply`, `induce`, `abstraction`, `generalize`, `novel`, `ask`, `shape`, `hint`, `patterns`, `propose` |
| `emit/` | `build`, `outline`, `graph`, `mapping`, `session` |
| `io/` | `cache`, `apply`, `cases` |
| `cli/` | the front: argument parsing, dispatch, and one file per command group |

`check` calls `closure` for the conditional-conclusion warning, so the check tier depends on
the reasoning tier in exactly one place. Keep it that way.

`outline` renders a document as prose. It may not break one rule: it quotes a field instead of
paraphrasing it, and it puts every ground in the margin, because a ground is not part of the
sentence. A projection that reads smoothly while hiding what the sentence stands on is the
failure this engine exists to refuse — produced by the engine.

## Checks the contract has grown

Each of these follows one rule: a property that can be derived from the document must be
checked against what was declared.

| code | what it compares |
| --- | --- |
| `E-TYPE` on list elements | every element must have the element type, not just be inside a list |
| `E-DUP-FIELD` | a field written twice: every reader takes the first, so the second is a claim the checker never sees |
| `W-EXAMPLE-SHAPE` | an example must share a recorded attribute with the concept's positives, or it cannot be compared with them |
| `W-BY-MISMATCH` | an inference type is derived from structure — a warrant makes deduction, concepts without a rule make induction — and a declaration that disagrees is contradicted |
| `W-CONFIDENCE-INFLATED` | a declared number is bounded by the weakest link: the rule's premises, or the premises a rule-less proposition cites |
| `E-PARTITION-*` | a closed list is contradicted by the dimension's extent and by any fact the attributes imply |
| `W-BARE-NUMBER` | a digit run with a unit in a `statement`/`claim` must be matched by a `value` carrying `measured_by` in `from:` or `basis:`; identifiers are not quantities |
| `W-BASIS-NO-CRITERION` | a `basis` entry is compared with the criterion its own kind owes: a law is checked, a value measured, a proposition concluded, a fact not merely a model opinion |
| `E-SUPERSEDE-UNDEFINED` | a `supersedes` name is resolved against the declared set; the edge is not a dependency, so nothing else would notice |
| `W-RETIRED-UNBASED` | a declaration that carries `supersedes` must name what makes the withdrawal right in `because:`; a reference field, so the reason resolves |
| `W-RETIRED` | a reference is compared with the set of names retired by `supersedes:`; it resolves, but it no longer grounds |
| `W-UNSATISFIED` | a citation is compared with the fields the cited law declares it requires under `requires:` |
| `E-OUT-MISMATCH` | a recorded hash is compared with the bytes of the artifact it names (path read from the working directory) |
| `W-OUT-UNVERIFIED` | a hash with no artifact, or an artifact that cannot be read — unverifiable, not wrong |
| `E-SELF-VERDICT` | a verdict is compared with the evidence grade of the fact that decides it; `evidence: model` cannot decide |
| `W-UNREAD` | a `value` with `threshold`/`priority` is compared with the set of names any rule references |
| `E-SCOPE-UNBOUNDED` | a universal claim's `over:` is compared with what the engine can enumerate — concept positives, closed partition elements, a layer's contents |
| `W-PARTITION-UNKNOWN` | a pile with no dimension may be written and may not discharge anything |

## Gatekeeping values from outside

`propose` takes a declaration, a field, and a value, and answers one of three ways:

- **admitted** — no diagnostic was introduced and no error was added;
- **review** — no error, but a diagnostic appeared that was not there before;
- **rejected** — an error was added, the value did not parse, or nothing changed.

`--policy` decides how much a fill must earn: `harmless` (the three above), `strict` (an
introduced diagnostic is a rejection), `improve` (something besides the hole must get better).
The default is `harmless`. None of the three looks at content.

## What a machine can check here

The checker operates on **structure**, not on meaning. A declaration's kind, its fields, the
references between declarations, the shape of those references and the closure they induce are all
machine-readable. The text of a `claim` or a `statement` is not: to every command here it is an
opaque string that only equality can compare. That is why this is a checkability checker and not a
semantic one -- and why "the model does not have to be trusted" is a statement about structure, not
about understanding.

The twelve kinds are an **epistemic type system**: each names an obligation rather than a data
shape. A `fact` owes evidence someone else can check, a `value` owes an instrument and a threshold,
a `warrant` owes premises, a rule and a licence. Passing the type check means the obligation is met,
not that the sentence is true.

| Layer | What is checked | What it does not give you |
| --- | --- | --- |
| syntax and types | the declaration is well formed, and its references have the kinds they claim | that the content is true |
| evidence and dependency | evidence is declared and support is whole | that the evidence settles the world |
| closure | which conclusions follow from the rules the document declares | that every valid inference has been found |
| sets and equivalence | extents, the equivalences this format defines, duplicate declarations | the equivalences a person would recognise |
| category | the named mappings preserve the relations they claim | that the document is a category |
## Deliberate choices about the text

These are decisions, not drift. Each one was the other way in an earlier draft of the contract,
and each is the way it is now for a reason a reader can check:

1. **Diagnostic wording and rendering.** The engine words its own messages and prints
   `CODE file:line message`; an earlier draft printed rustc-style frames with hints. Codes,
   severities, lines and columns agree; prose does not.
2. **`project` attribution.** An earlier draft guessed the owning file from the message text; the
   engine attributes by span.
3. **`E-LAYER-LEAK` span.** An earlier draft pointed at the target declaration; the engine points
   at the declaration that reaches across layers, which is the one to edit. Line numbers differ.
4. **`apply` only applies `MachineApplicable`.** Both sides do this; the note is here because
   a "verified" repair that changes meaning is still a suggestion.
5. **`mem` salt.** The case store records which checker produced an entry. The engine's salt is
   its own, so entries written by anything else read as expired. That is what the field is for.
6. **`egraph` does not crash.** An earlier draft aborted on a document whose concept references a
   note; the engine merges what it can and reports the rest.
7. **Empty extents do not collide.** A concept with no positive example asserts nothing, so
   it is not reported as `E-COLLAPSE`.
8. **`partition` is new.** An enumeration used to carry no obligation. The engine adds
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

10. **An abstraction needs a signatory.** A `layer` hides detail and a `law` says what is
    preserved, but neither says the hidden detail implements the law. `refines: layer` on a law
    is the third condition: it states that this law, holding in its own layer, implements the
    named one. A law with no such signatory anywhere is reported as `W-UNREFINED` -- the layer's
    promise is unsigned. It is a warning because "nobody has written the signatory yet" is a
    state, not a mistake; the bottom layer of a real system usually has none either.

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
discharge anything. `SELF.tsare` records that as a blind spot rather than guessing either way.

## Acceptance

```powershell
cargo test            # semantics + golden snapshots for every command
cargo build --release # the binary
```

`tests/codes.rs` holds the engine to its own claim: every code in `explain::CODES` must be turned
on by something the repository can produce. Three sources, because not every diagnostic comes from
one document — `tests/codes/*.tsare` (the smallest single document per code), the golden snapshots
(recorded output of the two-document commands), and direct assertions for parsing, the kernel and
goal files. A code that nothing can produce is either unreachable or stale, and both are bugs.

`tests/semantics.rs` holds properties (`an_attack_defeats_its_target`). `tests/golden.rs` holds
the exact text of every command on six fixture documents. `UPDATE_GOLDEN=1 cargo test` rewrites
them; read the diff first.


The content-addressed cache is bypassed for a document that names an `artifact:` or `out:` field: the
artifact's bytes are part of the answer, and the cache key is only the source text.
