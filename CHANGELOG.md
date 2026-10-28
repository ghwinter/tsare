# Changelog

All notable changes are recorded here. The format follows
[Keep a Changelog](https://keepachangelog.com/en/1.1.0/); versions follow
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Changed

- The document extension is `.tsare`. `.mf` collided with Java manifests.
  The header line is unchanged (`tsare 1`), because the tool and the format now share a name. The
  generated `predicates.py` header also stopped saying `mf7`, which nothing had noticed.

- `src/cli.rs` (2099 lines) is now `src/cli/`: `mod.rs` holds the argument model, the dispatch
  path and the help table; the commands live in four files that follow the groups the help screen
  already names — `checking`, `reasoning`, `repairing`, `emitting`. Nothing gained a feature; the
  largest source file is now `check/mod.rs` at 1286 lines instead of 2099. The rules the split
  keeps: one dispatch path, one place that decides, and a command that loads, calls one library
  function and prints.

- A diagnostic now records the declaration it is about (`Diagnostic.witness`). `fix` uses it:
  a repair is accepted when the complaints **about that declaration** shrink, not when the
  document's total goes down. The old rule let a repair be accepted for a warning it silenced
  elsewhere, and it could not say which of two same-code complaints it had answered. The line
  numbers move under a repair; the declaration name does not.
- `W-BASIS-NO-CRITERION` — a warrant that rests on something which is not a licence. `basis` was
  `ref:any` and only checked for emptiness, so it was the second field where writing anything
  counted as writing something (the first was `from:`, closed by `law.requires`). Each kind now
  has one criterion, and it is the same one that kind owes elsewhere: a `fact` must not be a
  model opinion, a `proposition` must stand and be concluded by some rule, a `concept` needs a
  positive example, a `partition` needs elements, a `layer` needs declarations, a `law` needs
  something checking it, a `value` needs a threshold and an instrument. A `note`, a `question`, a
  `hypothesis` and a `blindspot` are never licences.

  Its first run found three in `SELF.tsare`: three warrants that rest on model-supplied facts.
  They are left in place -- the warning is the finding.
- `fix` accepts a repair when the complaints on the declarations that already existed shrink, and
  refuses one that puts a new complaint on any of them. `fix::verified` still compared the
  document's error and warning totals, so a repair that silenced a warning in one corner while
  breaking something else could read as progress -- and "verified" is the word this command uses.
  A repair may still bring the declaration it needs: adding a rule to connect a proposition creates
  the rule, and the rule can owe something of its own. `W-HOLE` is ignored on both sides, because
  turning a missing field into a hole is the repair, not a new complaint.
- `W-BARE-NUMBER` — a conclusion states a quantity and cites no value that carries the instrument
  that produced it. The criterion is structural (does the sentence cite a `value` with
  `measured_by`); only the question "what counts as a number" is lexical, and that part was
  measured rather than guessed: firing on any digit produced 6 warnings over 76 documents, 4 of
  them identifiers like `A1`; firing on a digit with a unit produced 2, both real. So a quantity is
  a digit run with a unit (`16 ms`, `60 fps`, `3x`, `40%`), and `A1`, `0.4.3` and `F_1` are names.

  Known limit, in the code: `97 tests` and `第 3 条` are not caught, because there is no criterion
  yet for counting or for ordinals.
- `because:` and `W-RETIRED-UNBASED` — a declaration that withdraws another now says what makes the
  withdrawal right, in the same place every other decision does. The warning fires when a
  `supersedes` carries no `because:`; it is a warning rather than an error because an old document
  that withdrew something is still readable, and the reader only loses the ability to tell a
  considered withdrawal from a quiet one. `because` is a reference field, so a reason that names
  nothing is `E-UNDEFINED`, and a declaration cited as a reason counts as affected by `impact`.
- Induction could not find a clause that needed four literals: the search stopped at three and
  reported "no consistent conjunction of at most three literals exists". The literals of any
  covering clause must hold on every positive, so the search space is exactly that set -- and the
  full set is always checked, whatever its size. Measured: a concept whose four negatives each
  differ in one attribute went from `_failed_` to `a <= 1 and b <= 1 and c <= 1 and d <= 1`.
  The "impossible" answer now says why in the general case: when the most specific conjunction still
  covers a negative, nothing more general can exclude it either.
- A retired kernel law stops licensing. The governed set of a layer was built from
  `kind == "law"` alone, so `supersedes` inside the kernel was a no-op there: a target could stand
  on a principle the kernel had withdrawn, receive `W-RETIRED`, and still read as licensed.
  Measured before the fix: citing the retired law produced no `W-UNPRINCIPLED`, while citing
  nothing did.
- `shape PATH [--top N]` — the forms a document's declarations share, and the positions that vary
  inside each form. A form is the declaration's kind plus the shape of every field: the literal kind,
  and for a reference the kind at the other end. Content does not enter the signature, so two claims
  with nothing in common still fall into one class when the rest of the declaration has the same
  form. Classes of one are dropped -- a form with one instance is that declaration, not a structure.
  On `SELF.tsare` (44 declarations) it finds 8 shared forms, among them five warrants that all name
  one fact as both premise and licence; on `relational.v2.tsare` the warrant class puts `W_Y`'s
  `basis` next to its `from` in the same table, which is the difference that fixture exists to show.
- `shape --depth N` — the quotient refines. At depth 0 a declaration is grouped by its own fields;
  at depth `k+1` what it points at counts too, recursively. The classes only get finer, so the
  sequence is a filtration of the same set rather than a different reading. On `SELF.tsare` depth 1
  splits the five-warrant class into a pair (`W_OBLIGATION_ON_USE`, `W_TWO_OF_THREE`, whose
  conclusions are built the same way) plus three singletons. Finding it also found a wiring bug:
  `--depth` was not in the value-flag list, so `--depth 1` was parsed as a flag plus a positional
  and every run silently read depth 0 -- the first output looked like "the filtration never
  splits", which is exactly how a flag that does nothing reads.
- The attribute reading is now a closure operator, and it was not one before. `intent` of an empty
  sample returned no attributes; mathematically the intent of no objects is every attribute, since
  all of them are vacuously common. With the empty intent, a larger attribute set could have a
  smaller closure, so monotonicity failed at exactly the sets no fact satisfies -- found by a test
  that checks the three axioms over every subset of the attribute universe, not by reading the code.
  `intent` now takes the program and returns the attribute universe for the empty sample.
- `shape --layers` — the fibre view: a shared form that appears at more than one layer. A refinement
  looks exactly like a contradiction from the outside (the same structure claimed twice at different
  heights), so it is reported rather than judged. `--depth` applies here too: at depth 1 a fibre is
  a form whose members also agree on what they point at. Measured: no document in this repository
  has a fibre at depth 0 today -- the prelude files that use `layer:` are too heterogeneous for two
  declarations to share a shape -- so the behaviour is pinned by a crafted test, not by the corpus.
- No tracked file names the machine it was written on. `.github/workflows/ci.yml` now fails on a
  drive letter or a home directory in any tracked file (`:!.github/**` excluded so the guard does not
  match its own pattern). The audit that prompted it found one: the skill document told the reader
  the binary lives at an absolute path, which is wrong in every other checkout.
- `cat` reports each structural law from a table instead of from two branches in the command. Each
  entry names what the mapping takes, the condition it preserves, and the diagnostic that reports
  its failure; adding a law is adding an entry. A test cross-checks every entry against the
  document that violates it, so a law whose code never fires fails the build.

- `docs/PROTOCOL.md` now says, layer by layer, what a machine can check here and what it cannot:
  the checker operates on structure, and the text of a `claim` is an opaque string to every command.
  The twelve kinds are named for what they are -- an epistemic type system, where each kind owes an
  obligation rather than a data shape.
- `generalize` — subsumption between declaration forms, and the meet it induces. A form with holes
  is a pattern; one pattern subsumes another when every constraint of the first is met by the
  second; the meet is the fields both agree on, with everything else free. That is anti-unification
  over shapes, and the three laws that define a meet are checked as tests (it is below both inputs,
  and anything below both is below it). It generalises over **shapes**, not content: two facts with
  different sentences still share `claim:text`. Different kinds have no meet, and the API says so by
  returning nothing rather than inventing a top.
- `meet PATH LEFT RIGHT` — the most general form two declarations share, and the hole-template that
  fills what is left. It prints the shared constraints as a line and the draft as a declaration with
  `?` in every position, which is valid input: the pattern says what the family looks like, and the
  holes are the decisions a substitution still has to make. Two declarations of different kinds have
  no meet, and the command says so instead of inventing one.
- `shape PATH --against OTHER` — two documents compared as structures rather than as texts: which
  forms both use, which only one of them has. It is a set difference over signatures, so it cannot be
  talked into agreeing, and it is the primitive a question like "should this idea be written in that
  document instead" needs. Measured: `SELF.tsare` against `docs/REPO.tsare` gives 5 shared, 16 only
  there, 12 only here; `KERNEL.tsare` against `WORLDVIEW.tsare` gives 3 shared.
### Added

- `E-SUPERSEDE-CYCLE` — a `supersedes` graph that closes on itself. A withdraws B and B withdraws
  A, so each is withdrawn because of the other and neither stands; `check` reported only the
  missing `because:` lines, `engine` reported nothing held, and `derive` said every conclusion was
  grounded. Self-withdrawal was already an error; the walk is iterative because a long chain must
  not overflow the stack.

- `E-SUPERSEDE-UNDEFINED` — a `supersedes` field that names nothing, or names itself. A withdrawal
  that leaves nothing standing withdraws nothing; because `supersedes` is an edge rather than a
  dependency, a typo in it was completely silent. The self-verification test caught this code
  before the commit -- it had no document that turned it on -- the first time that test paid.

- `tests/prelude.rs` pins the SHA-256 of all four prelude files. Measured, not imagined: an agent
  deleted `requires: ["measured_by"]` from the kernel and every other test stayed green, because
  `W-UNPRINCIPLED` alone already fires on the violation.

### Changed

- Evidence has one grade, in one place: `program::Grade { Asserted < ModelSupplied < Confirmed <
  Measured }`. `closure` had its own copy that disagreed about whether a `confirmed_by` hole counts
  as a confirmation. Two consequences worth naming: `evidence: user` and `evidence: observation`
  are now `confirmed` rather than `asserted` (a person, or a sighting, is a witness), and a
  `hypothesis` is settled only by a witness graded `Confirmed` or better -- the rule used to be
  the word "model". `debt` reports the worst grade it found instead of calling every weak premise
  asserted.

- `W-UNWARRANTED-BASIS` pointed its caret at `claim`, blaming the sentence for a missing licence.
  It points at `basis` when the field exists and at the declaration otherwise.
### Added

- `prelude/` — the documents this project admits, and one it does not: the six principles
  (`KERNEL.tsare`), a claim that skips them (`VIOLATION.tsare`), the categories
  (`WORLDVIEW.tsare`), and what makes a sentence sayable at all (`PROCESS.tsare`). They are
  provisional the way an unstable ABI is: compiled in CI, and `tests/prelude.rs` fails if the
  kernel stops refusing the violation. A prelude nothing ever fails against is decoration.

- `tests/contract.rs` — the `--json` contract pinned by shape: the three envelopes of `check`,
  `engine` and `suggest`, the six fields of a diagnostic in order, and the invariant that a code
  starting with `E` carries severity `error`. Messages are deliberately not pinned; they are prose
  and they improve.

- `tests/codes.rs` — the engine signing for its own vocabulary. Every code in `explain::CODES`
  must be turned on by something the repository produces: one small document per code in
  `tests/codes/`, the golden snapshots for the codes only a two-document command emits, and
  direct assertions for parsing, the kernel and goal files. 73 codes, 66 of them from a single
  document. Before this the six fixtures turned on eight.

- `supersedes: [ID]` on every kind — withdrawal appends, it does not delete. The superseded
  declaration stays in the document and references to it still resolve, but it grounds nothing,
  supports nothing and attacks nothing: `closure` and `engine` drop it, conclusions that leaned on
  it become conditional with `retired` in the reason, `check` reports `W-RETIRED` at the citation,
  and `impact` prints the transitive set of declarations that still reach it. This is the whole of
  the memory feature — no store, no query language, only "what was replaced, and what falls with
  it".

- `abstract FILE [--min-support N] [--max-intent N]` — concept declarations the attribute table
  supports but nobody wrote. A candidate needs three things: at least two facts, an extent no
  named concept already has, and a witness pair (one fact in, one out). The third is what keeps a
  proposal from being a prettier name for "all of these". Candidates whose members are
  indistinguishable in the recorded vocabulary are dropped — that is a `W-COLLIDE`, and naming it
  adds a synonym rather than a distinction. Each proposal prints as a `concept` block that can be
  pasted back and compiles clean.

- `argue FILE --against OTHER` and `confront LEFT RIGHT` — an objection refers to the rule it
  objects to, so the pair only compiles together. `argue --against` computes the attack graph
  over the merge and reports the conclusions of `FILE`; `confront` shows which rule attacks
  which across the pair and `W-UNANSWERED` when `LEFT` has no rule that attacks back, with exit
  1 for that case. It judges whether an objection was answered, never whether it is right.

- `check --against` now requires the other document to compile *in context* instead of alone.
  The old rule was written for a prelude and is wrong for an opponent: an objection that must
  be self-contained cannot object to anything. What still holds is that nothing on the other
  side may be unresolvable once both documents are present, so a genuinely broken context is
  refused exactly as before.

- `fact over:` / `proposition over:` — the domain a universal claim ranges over. A universal is
  bound only when its domain can be enumerated: a concept's positive examples, a closed
  partition's elements, or a layer's declarations. Binding it to anything else is
  `E-SCOPE-UNBOUNDED`. `scope:` stops being a silencer: it is prose, and prose cannot be
  enumerated, so a universal carrying `scope:` and no `over:` is still `W-OVERCLAIM`. That is
  the point — naming a domain is not binding one. Propositions are covered as well, and they have
  no `scope:` field at all, so `over:` is their only binding.

- `fact artifact:` + `fact out:` — the hash of what the instrument produced. `evidence: tool(...)`
  records that something ran; `out` records what came out, and a mismatch is `E-OUT-MISMATCH`.
  A hash with nothing to compare against is `W-OUT-UNVERIFIED`, which is not the same as wrong.
  Artifact paths are read from the working directory. This closes a case that keeps recurring:
  the process exits 0 while the artifact is a placeholder, and every other check in this file is
  satisfied.

- `E-SELF-VERDICT` — a `hypothesis` may not be marked `supported` or `refuted` by a fact whose
  only witness is a model. A model assessing its own guess is not an observation; the verdict
  would assert an evidence grade nothing reached. The fact stays legal; it just cannot decide.

- `W-UNREAD` — a `value` that declares `threshold` or `priority` which no rule reads. A bound
  nothing cites cannot change a decision. It fired four times on `tests/fixtures/talk.v2.tsare` the
  first time it ran: four thresholds in that document were decoration.

- `law requires: ["field", ...]`: a law may name what citing it costs. A declaration that cites
  the law through `from`, `basis` or `checked_by` without carrying a named field gets
  `W-UNSATISFIED`. Before this, citing a principle was free: a reference was resolved for
  existence alone, so a claim about button colour could stand on a law about instruments and
  nothing noticed. The requirement does not decide whether the value is the right one -- it
  makes the claim pay for the name it used, which is the difference between a citation and a
  label. Where no kind that can cite a law even has the field (nothing but a `value` has
  `measured_by`), the principle cannot be cited past at all: it has to be satisfied by writing
  the thing down.

- `outline PATH`: the document read as prose. Each kind's spoken field is quoted verbatim and
  everything that makes it checkable -- a ground, an evidence kind, a closure, a diagnostic --
  goes in the margin beside it, because those are not part of the sentence. Declarations are
  grouped by the layer that admits them; a ready obligation owes nothing and says only that;
  the counted tail (`N obligation(s), M ready`) is part of the document. The projection never
  paraphrases: a paraphrase of a claim is a new claim.

- `check FILE --against KERNEL`: a kernel is a prelude. Its names may be used, not redefined
  (`E-KERNEL-SHADOW`), and a claim that puts itself in one of the kernel's layers must cite one
  of that layer's laws (`W-UNPRINCIPLED`). The kernel is checked alone first and must compile,
  or nothing is checked against it. It cannot refute what a claim says -- only refuse to admit
  it as grounded.

## [0.4.0] - 2026-10-26

The version where the engine starts judging its own vocabulary: whether a predicate is new,
which observation would decide an open list, and what the attribute table already implies.

### Added

- `ask`: the next question the document needs, from three families that are one defect in three
  places -- a list that does not cover its dimension, two facts that record the same evidence and
  the same attributes, and a concept whose positive and negative examples look alike. Each
  question names the attribute that would decide it, or says that none can. `--one` gives the
  single question that settles the most; `--json` is the shape a loop consumes.
- A test that walks the `--help` screen and fails when a command is listed there but missing
  from the dispatcher. Without it the command word is taken as a path and the error is
  `cannot find the file ask`, which says nothing about what is actually wrong.
- `novel --attribute N [--holds IDS]`: whether a proposed attribute is a new distinction, a
  column that is already recorded, or a recombination of the recorded attributes. The decision
  is a partition comparison -- two facts with identical records that the proposal separates are
  the witness -- so it needs no model and no statistics.
- `imply [--json] [--max-antecedent N] [--min-support N]`: the laws the attribute table already
  holds -- `items=1 -> urgent=0`, with the number of facts behind it and a fact that makes the
  converse fail. The search is bounded on purpose (antecedents up to N attributes, at least M
  facts) and says so in its own output instead of claiming to be the canonical basis.
- `ask`, `novel` and `imply` are MCP tools, so a model can ask for a judgement without a shell.
  The command line and the server print the same text: both call one renderer per command, and
  a test walks `tools/list` to check every listed tool is dispatched.
- `law.refines: LAYER`: the third condition of an abstraction. A `layer` hides detail and a
  `law` says what is preserved; `refines` states that a law at another layer implements it. A
  law with no signatory anywhere is reported as `W-UNREFINED` -- a warning, because an unsigned
  promise is a state of the document, not a mistake in it.

### Changed

- The text of `ask`, `novel` and `imply` is produced by one function each, used by both the
  command line and the MCP server, so the two cannot drift.

### Fixed

- The check cache and the case store live in `.tsare/`, under the tool's own name, so a run does
  not create directories named after a project that no longer owns them.
- `cargo package` carries no stray README from an unlisted directory.

## [0.3.0] - 2026-10-26

The version that learns to say what it cannot tell apart, and to say which format it speaks.

### Added

- `W-COLLIDE`: two facts that record the same evidence and the same attributes are reported,
  because nothing a machine can read tells them apart. A fact with no attributes never collides.
- A document may declare its format on the first line, `tsare 1`. The line is optional and its
  absence means format 1, so every existing document compiles unchanged. A document that
  declares a newer format stops with `E-VERSION` instead of producing parse noise.
- `--version` prints the crate version and the document format it speaks.
- A release workflow: a `v*` tag whose number differs from `Cargo.toml` fails before anything is
  published; `cargo package --locked` builds the exact package; publishing needs a token.

### Changed

- The tree tracks what the build, the tests or the package need, and nothing else: `src/`,
  `tests/`, four root documents, `docs/PROTOCOL.md`, `docs/REPO.tsare` and `SELF.tsare`. The rounds of
  documents, the examples, the experiment harness and the integration notes stay out of git.
- `cargo package` carries the same list, so the published crate contains no stray README.

## [0.2.0] - 2026-10-26

The version that can be published: one crate, no dependencies, MIT.

### Added

- `partition`: a list must name the concept its elements share, say whether it is closed, and
  give a reason when it is. A list with no dimension may be written only as `unknown`, names
  the question that would type it, and cannot discharge a premise.
- `fca`: the attribute pairs a concept's example share, and the facts those attributes imply.
- `propose`: gate values an outside model wants to fill into holes, with three verdicts
  (`admitted`, `review`, `rejected`) and three policies (`harmless`, `strict`, `improve`).
- `--explain CODE`: what a diagnostic means and the smallest fix; a test keeps the table
  complete for every code the compiler can print.
- `--version`, `--help` covering every command, and `PATH` may be `-` for standard input.
- Diagnostics carry an end position, are rendered with the source line and a caret, and a
  directory is rendered file by file.
- A `Session` owns ordering, counts, text, JSON and the exit code.
- Parser recovery: every syntax error in one run, as `E-SYNTAX` with a span.
- Checks derived from structure: list element types, `W-EXAMPLE-SHAPE`, `W-BY-MISMATCH`,
  `W-CONFIDENCE-INFLATED`, and the partition family.

### Changed

- The crate lives at the repository root; `rust/tsare` is gone.
- Documentation moved under `docs/`.

### Fixed

- A comma between fields is required; it had been optional, so a whole class of typos parsed.
- `E-COLLAPSE` no longer fires for a concept with no positive example.
- `at all` is not treated as a universal claim.


