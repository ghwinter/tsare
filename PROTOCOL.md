# engine — protocol first

This repository is the rewrite. It starts from the contract, not from code.

## Why a separate repository

`obligo` (now frozen at tag `v1`) holds the working implementation and every experiment.
`metaform` holds the theory and the Rust prototype (VSA, SME, ILP, store). Both stay as
they are. This repository exists so the next implementation does not inherit their
directory layout, their nested-repository problems, or their accumulated scripts.

## The contract

Every command reads files and writes JSON. The Python CLI is a thin shell over these, and
a future Rust implementation must produce byte-identical JSON for the same input.

### `check FILE|DIR`

```json
{
  "diagnostics": [
    {"code": "E-UNDEFINED", "severity": "error", "file": "02-claims.mf",
     "line": 2, "col": 41, "message": "...", "hint": "..."}
  ],
  "errors": 1, "warnings": 2
}
```

`file` is required. Attribution by parsing message text (what v1 does for cross-file
diagnostics) is not allowed.

### `engine FILE|DIR`

```json
{
  "held": ["P_SMOOTH"],
  "conditional": ["P_B"],
  "blocked": [{"warrant": "W_LINK", "missing": ["P_A"]}],
  "chains": {"P_SMOOTH": "P_SMOOTH <- W_SMOOTH <- F_TRACE"},
  "gates": {"P_SMOOTH": []}
}
```

### `suggest FILE|DIR`

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

A suggestion names a `span` and a `replacement` for exactly that span.  A deletion is an
empty replacement over the removed declaration.  `span` may be `null` when the repair
inserts something with no existing declaration to replace.

## Directories

Every command accepts a file or a directory.  A directory is checked as one program:

- names are global; a duplicate is reported with both file names;
- cross-file references resolve;
- layer visibility is computed once over the merged program;
- a diagnostic is attributed by its span, matching a declaration's span exactly.  Attribution
  never inspects the message text.

## Module tiers

| tier | modules | in the protocol |
| --- | --- | --- |
| core | `syntax`, `diag`, `check`, `project`, `cli` | yes |
| reasoning | `argue`, `closure`, `derive`, `engine`, `debt`, `fix`, `suggestion` | yes |
| analysis | `cost`, `impact`, `egraph`, `whatif` | no — experiments and tests only |

`check` calls into `derive` for the conditional-conclusion warning, so the core tier depends
on the reasoning tier in exactly one place.  Keep it that way.

## What carries over from v1

Semantics, not structure:

1. Kinds and their obligations (`kinds_meta`).
2. Local checks: record shape, references, cycles, semantics.
3. Global checks: collapse, tension, layer visibility.
4. Grounding closure and attack propagation (`engine`).
5. Verified repairs with applicability (`suggest`).

Everything else — generation, loops, benchmarks, memory — lives on the `experiments`
branch of `obligo` and is not part of this contract.

## Acceptance

The 64 tests in `obligo/tests` are the acceptance suite. A port is done when it passes
them, and when `check` on every `obligo/*.mf` produces the same diagnostic set.  Line
numbers are compared only when spans are unchanged; the layer-leak span is a deliberate
difference.
