# Hardening tsare against a corpus of real decisions

**Status: stopped on request. The tree is green; the last commit is `f1a6079`.**

The subject is `tsare 0.4.0`, a deterministic checker for `.tsare` claims. The work was: write
real decision documents, run the whole command surface over them, classify every diagnostic as
right / arguable / wrong, then fix the wrong ones with a failing test and a separate commit.

## 1. Method

* Corpus: **49 authored `.tsare` documents** in `docs/`, plus `docs/project/` (3 files checked as
  one directory program) and the existing `docs/REPO.tsare` as a control. 19 minimal reproducers
  in `docs/repro/`.
* Harness: `scripts/matrix.py` runs 50 command shapes over every document **four times**, recording
  exit status, a stdout hash, whether the four runs were byte-identical, and every check diagnostic.
  It ran **2,490 command runs before** the fixes and **2,540 after** (induce was added). Snapshot:
  `docs/artifacts/matrix-baseline.jsonl`, `matrix-after.jsonl`; human summary in
  `docs/artifacts/matrix-comparison.txt`.
* Judgement: for each code that fired on the corpus I read the message, the declaration and the
  check source, and decided whether the document really had that defect. The same was done for
  reproducers, where the "document" is a deliberate trigger.
* Before/after matrices differ in exactly the six diagnostics the fixes removed
  (`docs/artifacts/matrix-comparison.txt`): `04-cache-hit` E-CYCLE, `23-overclaim-small`
  W-OVERCLAIM ×2, `43-model-settled` W-BASIS-NO-CRITERION, `48-concludes` W-UNWARRANTED ×2.
  Those four documents now compile clean; they are kept as the record of the pre-fix behaviour.

## 2. Document inventory

`docs/NN-*.tsare`. Every document records a decision made while hardening this repository:
release budget, renderer contract, determinism experiment, cache replay, JSON surface, layer
split, priority tension, supersede, universal binding, blind spot, question, rejected layouts,
confirmed model fact, confidence, cycle, collapse, collision, unread bound, bare number,
truncation, schema examples, partitions (open/closed/pile), artifacts, self-verdict, hierarchy,
layer leak, unsatisfied law, retirement, scope, notes, verbose/terse/multi-layer/fibre/bilingual,
concludes, and the engine-order reproducer.

Table key: `decls` = declarations, `diag after` = diagnostics in the post-fix matrix.

| document | style / features | decls | diag after |
| --- | --- | ---: | --- |
| 01-release-budget | value + instrument + artifact hash | 4 | – |
| 02-renderer-contract | facts from code/doc, proposition, warrant | 5 | – |
| 03-engine-order | measured experiment, hypothesis with verdict | 5 | – |
| 04-cache-hit | cache finding, two rival hypotheses | 5 | – (was E-CYCLE) |
| 05-json-surface | concept, question, closed partition | 11 | – |
| 06-layer-split | two layers, two laws, `refines` | 10 | – |
| 07-priority | two values, opposite direction, priorities | 5 | – |
| 08-tension-unresolved | same metric, no priority | 8 | E-TENSION |
| 09-supersede | `supersedes` + `because` | 5 | – |
| 10-universal-bound | `over:` a concept extent | 6 | – |
| 11-universal-unbound | the negative example for `over:` | 1 | W-OVERCLAIM |
| 12-blindspot | witnesses, signature, closing question | 8 | – |
| 13-question | question + two hypotheses | 5 | – |
| 14-rejected-alternatives | partition over a concept, closure basis | 10 | – |
| 15-model-confirmed | `evidence: model` + `confirmed_by` | 4 | – |
| 16-confidence-honest | honest confidence on a derived proposition | 4 | – |
| 17-confidence-inflated | number above its weakest premise | 2 | W-UNWARRANTED, W-CONFIDENCE-ASSERTED, W-CONFIDENCE-INFLATED |
| 18-cycle | two propositions as each other's premise | 2 | E-CYCLE, W-UNWARRANTED ×2 |
| 19-collapse | two concepts, same extent | 5 | E-COLLAPSE, W-UNCONFIRMED ×3 |
| 20-collide | two facts, one attribute record | 2 | W-COLLIDE ×2 |
| 21-unread-value | a bound no rule reads | 3 | W-UNREAD, W-UNWARRANTED |
| 22-bare-number | before/after quantity without a `value` | 5 | W-BARE-NUMBER, W-UNWARRANTED |
| 23-overclaim-small | "the small cache" (pre-fix false positive) | 3 | – |
| 24-example-shape | negative example from another measurement | 4 | W-EXAMPLE-SHAPE |
| 25-partition-open-used | open list used as a premise | 6 | W-PARTITION-OPEN-USED, W-UNWARRANTED |
| 26-partition-closed-contradicted | closed list omits an instance | 6 | E-PARTITION-CLOSED-CONTRADICTED, W-UNTESTED |
| 27-partition-pile | `closure: unknown`, closing question | 8 | W-PARTITION-UNKNOWN, W-UNCONFIRMED ×4 |
| 28-value-incommensurable | instrument, unit, no threshold | 2 | E-INCOMMENSURABLE |
| 29-out-mismatch | stale artifact hash | 1 | E-OUT-MISMATCH |
| 30-self-verdict | model fact settles its own hypothesis | 2 | E-SELF-VERDICT, W-UNCONFIRMED |
| 31-hierarchy | child example outside parent extent | 4 | E-HIERARCHY |
| 32-layer-leak | unexposed cross-layer reference | 4 | E-LAYER-LEAK, W-UNWARRANTED |
| 33-unsatisfied | citing a law that requires `measured_by` | 8 | W-UNSATISFIED |
| 34-retired-unbased | withdrawal with no reason, still cited | 4 | W-RETIRED-UNBASED, W-CONDITIONAL, W-RETIRED ×2, W-BARE-NUMBER |
| 35-scope-unbounded | `over:` bound to a fact | 2 | E-SCOPE-UNBOUNDED |
| 36-artifact-unverified | hash with no readable artifact | 1 | W-OUT-UNVERIFIED |
| 37-terse | the smallest clean document | 3 | – |
| 38-verbose | 20 declarations, every kind, layers, partition, fibre | 20 | – |
| 39-header | `tsare 1` header line | 3 | – |
| 40-notes | notes only, plus one fact | 4 | – |
| 41-research-question | determinism as a question between hypotheses | 7 | – |
| 42-multi-layer-fibre | same form at two layers | 8 | – |
| 43-model-settled | confirmed model fact settles a verdict | 4 | – (was W-BASIS-NO-CRITERION) |
| 44-causal | causal explanation with a rival | 7 | – (was W-OVERCLAIM on "stall") |
| 45-bilingual | Chinese: 所有 bound, 首都 misread | 6 | W-OVERCLAIM (left open, see §6) |
| 46-suggest-contract | suggest JSON shape as a decision | 4 | – |
| 47-exit-codes | exit-status survey as a decision | 7 | – |
| 48-concludes | `concludes:` class warrant | 4 | – (was W-UNWARRANTED ×2) |
| 49-engine-order-repro | four conditional rows, one assumption | 11 | W-UNCONFIRMED, W-CONDITIONAL ×4 |
| REPO.tsare (control) | the repository's own document | 20 | W-OVERCLAIM ×3, W-UNREFINED, W-UNREAD |

`docs/project/` is one three-file program; `check docs/project` is 0 errors / 0 warnings and
`project docs/project` prints nothing but the summary (`docs/artifacts/project-program.txt`).

## 3. Diagnostic table (post-fix corpus)

Totals include the REPO control. Baseline is the matrix before any fix.

| code | baseline → after | documents | judgement | why |
| --- | ---: | --- | --- | --- |
| E-COLLAPSE | 1 → 1 | 19 | right | two names, one extent |
| E-CYCLE | 2 → 1 | 18 | right (1); the removed one was wrong | a genuine premise cycle remains in 18; mutual rivals in 04 were not a justification cycle |
| E-HIERARCHY | 1 → 1 | 31 | right | child extent not inside parent |
| E-INCOMMENSURABLE | 1 → 1 | 28 | right | no threshold |
| E-LAYER-LEAK | 1 → 1 | 32 | right | core facts not exposed |
| E-OUT-MISMATCH | 1 → 1 | 29 | right | stale hash |
| E-PARTITION-CLOSED-CONTRADICTED | 1 → 1 | 26 | right | list omits an instance |
| E-SCOPE-UNBOUNDED | 1 → 1 | 35 | right | `over:` names a fact |
| E-SELF-VERDICT | 1 → 1 | 30 | right | model decides its own guess |
| E-TENSION | 1 → 1 | 08 | right | opposite bounds, no priority |
| W-BARE-NUMBER | 2 → 2 | 22, 34 | arguable | the instrument exists as a measured fact, but the criterion demands a `value` |
| W-BASIS-NO-CRITERION | 1 → 0 | 43 | wrong, fixed | confirmed model fact is a licence |
| W-COLLIDE | 2 → 2 | 20 | right | one attribute record |
| W-CONDITIONAL | 5 → 5 | 34, 49 | right | conclusions wait on a retired fact / hypothesis |
| W-CONFIDENCE-ASSERTED | 1 → 1 | 17 | right | number with no rule |
| W-CONFIDENCE-INFLATED | 1 → 1 | 17 | right | 0.9 above 0.5 |
| W-EXAMPLE-SHAPE | 1 → 1 | 24 | right | negative from another measurement |
| W-OUT-UNVERIFIED | 1 → 1 | 36 | right | artifact absent |
| W-OVERCLAIM | 7 → 5 | 11, 45, REPO | right 4, wrong 1 | 11 and REPO are genuinely unbound universals; 45 is `首都` (see §6); 23 and 44 were false positives and are fixed |
| W-PARTITION-OPEN-USED | 1 → 1 | 25 | right | open list used as premise |
| W-PARTITION-UNKNOWN | 1 → 1 | 27 | right | declared pile |
| W-RETIRED | 2 → 2 | 34 | right | citations of a withdrawn threshold |
| W-RETIRED-UNBASED | 1 → 1 | 34 | right | withdrawal with no reason |
| W-UNCONFIRMED | 9 → 9 | 19, 27, 30, 49 | right | model facts |
| W-UNREAD | 2 → 2 | 21, REPO | right | bound no rule reads |
| W-UNREFINED | 1 → 1 | REPO | right | law with no signatory |
| W-UNSATISFIED | 1 → 1 | 33 | right | law requires a field the citation lacks |
| W-UNTESTED | 1 → 1 | 26 | right | concept without a counterexample |
| W-UNWARRANTED | 9 → 7 | 17, 18, 21, 22, 25, 32 | right (7); removed 2 were wrong | the class-conclusion document was falsely accused |

**After: 54 diagnostic occurrences, 51 right, 2 arguable, 1 wrong (open).**
**Baseline: 60 occurrences, 51 right, 2 arguable, 7 wrong; all 6 differential removals are fixed defects.**
No diagnostic was counted in the totals twice except where the checker prints one per declaration
(noted as ×N in the inventory).

### Reproducer-only findings (not part of the corpus counts)

The reproducers in `docs/repro/` and the regression tests trigger findings that no real document
should contain: missing `signature` resolution, duplicate fields, `confidence: 150%`, the
partition and unit cascades, `propose --apply` losing its flag, the cache replay, the E-CYCLE
pair, and the exit statuses. Each is in §4 with its own command.

## 4. Defects fixed (34 commits)

Each was reproduced first and the test failed on the parent commit (the regression suite was
built that way; the build-directory tests were re-run against the parent with `git stash`).
`tests/regressions.rs` has one test per defect; `tests/codes/E-DUP-FIELD.tsare` covers the new code.

| # | defect | reproducer / command | commit |
| ---: | --- | --- | --- |
| 1 | cache hit drops `name`, `end_*`: `check --cache` ≠ `check` | `tsare check docs/repro/engine-order.tsare --json --cache` twice | 6f8207e |
| 2 | `signature` not resolved or kind-checked | `tsare check docs/repro/signature-undefined.tsare` | 3ca6ca8 |
| 3 | W-OVERCLAIM substring: "small"/"stall" fire, "All" misses | `tsare check docs/repro/overclaim-small.tsare` | af6a72b |
| 4 | engine/`project` nondeterministic (2,490-run matrix: 3 commands varied; after: 0) | `tsare engine --json docs/49-engine-order-repro.tsare` ×4 | 77bbe88 |
| 5 | unknown options ignored; `propose --apply --policy` silently did not write | `tsare check doc --strcit`; `tsare propose docs/repro/propose-target.tsare --proposals docs/repro/proposals.json --apply --policy harmless` | 16c4ead |
| 6 | `cost` charges a decided hypothesis for a test | `tsare cost docs/repro/decided-hypothesis.tsare` | 811fded |
| 7 | `build` invents `tool` for a hole evidence | `tsare build docs/repro/build-hole-evidence.tsare --out X` | 06386c1 |
| 8 | `build` marks a rule-less proposition ready | `tsare build docs/repro/build-unwarranted.tsare --out X` | 01e66d7 |
| 9 | `derive` grounds conclusions in retired facts | `tsare derive docs/repro/retired-derive.tsare` vs `engine` | b6f2075 |
| 10 | `concludes:` invisible to check/argue/weigh/cost/debt | `tsare argue docs/48-concludes.tsare` (was "asserted/no rule") | 206e0ec |
| 11 | E-CYCLE on mutual rivals and mutual attacks | `tsare check docs/repro/mutual-rival.tsare` | acede5a |
| 12 | duplicate field silently ignored | `tsare check docs/repro/duplicate-field.tsare` | 3dbc350 |
| 13 | `confidence: 150%` skipped E-CONFIDENCE | `tsare check docs/repro/confidence-percent.tsare` | d8fb661 |
| 14 | `unit: 5` produced E-TYPE + a false E-INCOMMENSURABLE | `tsare check docs/repro/unit-num.tsare` | c3fa7c7 |
| 15 | undefined partition dimension condemned its elements | `tsare check docs/repro/partition-undef.tsare` | 37cc1ba |
| 16 | weigh ignored proposition premises; W-CONFIDENCE-INFLATED missed | `tsare weigh docs/repro/confidence-chain.tsare` | dab8a07 |
| 17 | a syntax error hid every semantic error in the other files | `tsare check docs/repro/project-dir` | 8a1ff9c |
| 18 | `project` stopped at the first syntax error, blamed the directory | `tsare project docs/repro/project-dir` | a40a91d |
| 19 | argue/impact/weigh/cat/induce/graph printed "N error(s)" and exited 0 | `tsare argue docs/repro/project-dir/b.tsare; echo $?` | 204f19b |
| 20 | `mem` called a regression a "no-op (changed nothing)" | `docs/artifacts/matrix-*`; regression test writes a store | a46a9c4 |
| 21 | `--explain W-OVERCLAIM` advised `scope:`, which the check does not read | `tsare --explain W-OVERCLAIM` | 5c48f47 |
| 22 | `--help` said exit 2 for an unreadable file; the binary returns 1 | `tsare --help` | 67fadef |
| 23 | `suggest --json` printed `"file": ""` for a single file | `tsare suggest docs/repro/project-dir/b.tsare --json` | ef7b161 |
| 24 | graph/egraph/fix --apply panicked (exit 101) on an unwritable path | graph `--out <file>/x.mmd` | 4735846 |
| 25 | `build DIR` emitted 2 of 5 artifacts and did not check errors | `tsare build docs/project --out X` | 4c52f82 |
| 26 | retired law still signed a refinement | regression test in `tests/regressions.rs` | e35e4f6 |
| 27 | retired warrant still connected premises | regression test | 7afaab1 |
| 28 | W-CONDITIONAL called a hypothesis premise "(fact)" | `tsare check docs/49-engine-order-repro.tsare` | 0bfc498 |
| 29 | engine/derive/debt/W-CONDITIONAL truncated lists without "more" | regression test with four gates/reasons | 99bbd22 |
| 30 | PROTOCOL suggest shape and `--explain E-VERSION` described non-existent things | `tsare --explain E-VERSION`; PROTOCOL §suggest | c623262 |
| 31 | `ask` gap message ended "from " with an empty twin list | regression test | 9a37a13 |
| 32 | basis criterion rejected a confirmed model fact | `tsare check docs/43-model-settled.tsare` | d7c532e |
| 33 | `check --cache` replayed a clean report after the artifact changed | regression test changes the artifact | bc38925 |
| 34 | E-UNCOVERED message named two of six commitment kinds | regression test via a goal file | f1a6079 |

New diagnostic: **E-DUP-FIELD** (error, `check`). The document grammar and the format number are
unchanged; the code is in `--explain` and in the `tests/codes` corpus. Per PROTOCOL a new check is
a minor crate change; `Cargo.toml` was **not** bumped (see §6).

## 5. Verification

Run from the repository root; all results below were run on the final tree.

```text
cargo fmt --all --check                 -> exit 0
cargo clippy --all-targets -- -D warnings -> Finished (no warnings)
cargo test --release                    -> 23+0+4+1+12+10+2+5+36+70+0 tests, 0 failed
```

`cargo test --release` includes the binary tests: golden snapshots, JSON contract, MCP, prelude,
and the 36 regression tests. The golden updates in this work are deliberate and limited:
`talk.v2.fix.txt` (bogus `scope: ?` candidates removed), `talk.v2.build.OBLIGATIONS.md.txt` and
`talk.v2.outline.txt` (two rule-less propositions blocked instead of ready),
`schema.{argue,cost,debt-all,weigh,whatif}.txt` (a class conclusion is now a conclusion),
`talk.v2.suggest-json.txt` (file named).

Corpus evidence (all committed under `docs/artifacts/`):

```text
python scripts/matrix.py
  50 documents, 2540 command runs, nondeterministic: 0
python scripts/compare.py
  baseline: 60 diagnostic occurrences, 3 nondeterministic commands
  after:    54 diagnostic occurrences, 0 nondeterministic commands
  exit status on error documents: argue/impact/weigh/cat/induce/graph were
  exit 0 in 11/11 and are now exit 1 in 10/10 (the 11th document lost its error)
```

Selected direct evidence:

```text
tsare check docs/repro/project-dir       -> both E-SYNTAX in c.tsare and E-DUP-ID in b.tsare
tsare project docs/repro/project-dir    -> "# b.tsare" and "# c.tsare", exit 1
tsare check docs/49-engine-order-repro.tsare --json  (4 runs) -> one byte sequence
tsare derive docs/repro/retired-derive.tsare -> P_FAST conditional, not derived
tsare weigh docs/repro/confidence-chain.tsare -> P2 = 0.90, check warns W-CONFIDENCE-INFLATED
MCP vs CLI divergence recorded in docs/artifacts/mcp-vs-cli.txt (left open, §6)
```

## 6. Left open (not fixed)

1. **`都` inside a noun.** `首都` is still read as the universal `都`; the cue is documented as
   deliberately generous. Document: `docs/45-bilingual.tsare`. Deliberate, left open.
2. **W-BARE-NUMBER does not accept a measured fact as the instrument.** Documents 22 and 34 cite a
   tool-evidenced fact but no `value`; the warning is defensible under the documented criterion but
   arguably wrong. Not changed.
3. **`cost` does not charge a `value` that has an instrument but no threshold** (`obligations`
   checks only the instrument) while `build` calls it blocked. Design question, not fixed.
4. **Commands that print no error count still exit 0 on a document with errors** (`derive`,
   `debt`, `next`, `ask`, `outline`, `audit`, `egraph`, `fix`, `suggest`, `whatif`). The six that
   do print a count now return 1; the rest is a design decision about exploratory commands.
5. **`concludes:` is still invisible to the graph-position analyses** (`impact`, `map`, `pattern`,
   `induce`). `closure`, `program`, `argue`, `weigh`, `check`, `cost` and `debt` were fixed; the
   remaining ones need their own edge model and would move `map`/`pattern` goldens.
6. **`engine`'s chain printer caps depth at 12 silently** (`closure::chain`); not fixed.
7. **Non-numeric option values silently fall back** (`--top abc` uses 8; `--policy strick` is
   harmless). Unknown options now exit 2, but bad values do not.
8. **The crate version was not bumped** although E-DUP-FIELD is a new check (PROTOCOL says a new
   diagnostic is a minor change). Recommended: 0.5.0 at release time.
9. **MCP tools `argue`, `derive`, `fix`, `kinds` do not print the CLI text**, although PROTOCOL
   says one renderer per command; see `docs/artifacts/mcp-vs-cli.txt` (argue and kinds differ).
10. **`check --against` prints kernel warnings only as a text note**; JSON carries target
    diagnostics only. Design.
11. **`suggest --json` `HasPlaceholders` suggestions are not applicable by `apply`** (only
    MachineApplicable is), so applicability is not a promise of applicability. Design.

## 7. Paths

* Corpus: `docs/01-*.tsare` … `docs/49-*.tsare`, `docs/project/`, `docs/repro/`
* Evidence: `docs/artifacts/` (`matrix-*.jsonl`, `matrix-comparison.txt`, `matrix-summary-*.txt`,
  `engine-order.txt`, `cache-name.txt`, `exit-codes.txt`, `build-obligations.txt`, `weigh-chain.txt`,
  `project-syntax.txt`, `project-program.txt`, `mcp-vs-cli.txt`, `binary-size.txt`, `src-lines.txt`,
  `command-count.txt`)
* Harness: `scripts/matrix.py`, `scripts/summarize.py`, `scripts/compare.py`
* Tests: `tests/regressions.rs` (36), `tests/codes/E-DUP-FIELD.tsare`
