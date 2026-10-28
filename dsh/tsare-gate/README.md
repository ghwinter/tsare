# tsare-gate

A DSH plugin that runs the checker **inside the turn loop** instead of asking the model to
remember it. The skill (`../tsare/SKILL.md`) is still the instruction; this is the enforcement.

## What it does

`tsare` decides whether a claim can be checked. Whether it is *called*, and whether its answer is
obeyed, is the model's choice — and a compiler that runs at the model's discretion is a compiler
that runs when the model is confident, which is exactly when it is wrong. The plugin removes that
choice at three seams DSH owns:

| seam | behaviour |
| --- | --- |
| `agent/turn-stopping` | the turn is about to close; the plugin runs `tsare check LEDGER --json` and, if the ledger is not checkable, calls `agent.steer(...)` with the diagnostics. The machine runs another step: **the turn cannot close while the checker objects**, up to `steerLimit` times. |
| `systemPrompt.context` | one standing line: the ledger's **next question** (`tsare ask --one`) or its first error, replaced in place and recomputed only when the ledger changes. This is the engine; the gate is the brake. |
| `tools/post-execute` | for the tools named in `remindAfterTools`, a dirty ledger rides along as context on the next request; for arguments matching `remindWhen`, one reminder that the result is a claim, once per ledger state. It never blocks the tool. |
| `agent/created` | a session that starts without a ledger gets one: a single `note` declaration, which is a valid document. |

The brief is the cheap channel: DSH renders runtime context every step but replaces it in place,
so a line that only changes when the ledger does costs one message per change, not one per turn.
The gate is the expensive channel: one steer is a full model request, so it is capped and should
almost never fire. The plugin runs no model, opens no network, and writes nothing but the template.

## Config

| key | default | meaning |
| --- | --- | --- |
| `binary` | `tsare` | the executable, resolved from `PATH` unless absolute |
| `ledger` | `.tsare/session.tsare` | ledger path, relative to `cwd` |
| `cwd` | the session's own working directory (from its header) | where the ledger lives; set it to pin one directory for every session |
| `maxDeclarations` | `150` | after this many declarations the gate says so on every objection |
| `mode` | `errors` | `errors` fails on errors; `warnings` also fails on warnings (`--strict`) |
| `steerLimit` | `3` | how many times one agent may be held open before the gate gives up |
| `timeoutMs` | `10000` | per check |
| `brief` | `true` | keep the next question in the runtime snapshot |
| `briefMaxChars` | `240` | cap for that line |
| `remindAfterTools` | `[]` | tool names that attach the diagnostics as context; empty means off |
| `remindWhen` | `[]` | regexes matched against a call's arguments (e.g. `cargo (test|build)`); a match gets one claim reminder per ledger state |
| `remindLimit` | `5` | how many claim reminders one agent may receive |
| `command` | none | argv override (`[file, ...args]`) for the check, for tests and wrappers |
| `askCommand` | none | argv override for the next-question call |
| `debug` | `false` | append one line per hook (apply, created, install, post-execute, turn-stopping) to a temp file, for diagnosing a live integration |

## Activate it in a profile

The plugin is a package with `name` and `apply`; DSH profiles compose plugins through their own
patch layer, so no change to DSH itself is needed. Add one entry to
`$DSH_HOME/profiles/<name>/cordis.patch.yml`, install the package into that profile (a path
dependency works), and the listeners are live for the next session:

```yaml
- id: tsare-gate
  name: 'tsare-gate'
  config:
    binary: tsare
    ledger: SESSION.tsare
    mode: errors
    remindAfterTools: [bash]
```

## Test

```powershell
node selftest.mjs                 # fake checker: 7 checks, no dependencies
$env:TSARE_BIN = "tsare"; node verify-real.mjs   # the real binary, same parser
```

`verify-real.mjs` needs the executable but no absolute path: it reads `TSARE_BIN`.

## Limits, stated rather than hidden

- The ledger lives in the session's working directory (`session.header.cwd`), one file per
  session; a profile that wants a different place sets `cwd`.
- The gate checks the **ledger**, not the answer. An answer can cite a clean ledger and still say
  something the ledger does not contain; closing that needs `tsare cite` (prose → declarations),
  which does not exist yet.
- A thin ledger passes. The defence is not this plugin but `tsare cost`, `tsare assumptions` and
  `tsare ask --one`, which say what the ledger owes; a session that writes one note and six numbers
  is a visible mismatch, not a hidden one.
- The steer limit is finite on purpose: a checker that always objects must not be able to hold a
  turn forever. When the limit is reached the turn closes with the last objection unaddressed, and
  that is visible in the transcript.
- The brief is a reading of the ledger, not of the answer. It can say "next question: does F_X
  record weight?" while the answer says something the ledger never mentions; closing that needs
  `tsare cite`, which does not exist yet.
- The brief has no freshness rule: a ledger that was not touched this session renders the same
  line. That is why the line names the ledger path and the question, not "this session checked N
  claims" — the former is true even when stale, the latter would not be.

## Where the ledger lives, and why not in the tree

DSH designates three homes for content a plugin produces, and none of them is "a dot-directory in
the user's repository":

- **the session log** for records that belong to the conversation (session.append; the hook
  protocol writes `hook/invoked` and `hook/result` this way), and
- **storage domains** (ctx.storageDomain) for a plugin's own durable state, and
- **spill** for large artifacts, which land outside the workspace in a per-session temporary root
  and are swept at startup.

The ledger is a document the agent edits and a person reads, so it has to be a file — but it is
the harness's file, not the project's. It goes in `.tsare/` at the session's working directory,
and the plugin writes `.tsare/.gitignore` containing `*` when it creates the directory: the
directory ignores itself, so a session in any repository leaves `git status` clean without the
plugin editing the user's `.gitignore`.

Growth: one declaration per claim is stable, but a long session keeps every claim it ever made.
The gate counts declarations and, past `maxDeclarations`, appends a note to each objection: move
retired ones to `.tsare/archive.tsare`. It does not rotate automatically — a claim that vanishes
from the ledger is a claim the model may still cite, and silently deleting one is worse than a
long file. The bound exists to make growth visible, not to hide it.