# tsare-gate

A DSH plugin that runs the checker **inside the turn loop** instead of asking the model to
remember it. The skill (`../tsare/SKILL.md`) is still the instruction; this is the enforcement.

## What it does

`tsare` decides whether a claim can be checked. Whether it is *called*, and whether its answer is
obeyed, is the model's choice — and a compiler that runs at the model's discretion is a compiler
that runs when the model is confident, which is exactly when it is wrong. The plugin removes that
choice at two seams DSH owns:

| seam | behaviour |
| --- | --- |
| `agent/turn-stopping` | the turn is about to close; the plugin runs `tsare check LEDGER --json` and, if the ledger is not checkable, calls `agent.steer(...)` with the diagnostics. The machine runs another step: **the turn cannot close while the checker objects**, up to `steerLimit` times. |
| `tools/post-execute` | for the tools named in `remindAfterTools`, a dirty ledger rides along as context on the next request. It never blocks the tool. |
| `agent/created` | a session that starts without a ledger gets one: a single `note` declaration, which is a valid document. |

The plugin runs no model, opens no network, and writes nothing but that template.

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
| `remindAfterTools` | `[]` | tool names that attach the diagnostics as context; empty means off |
| `command` | none | argv override (`[file, ...args]`) for tests and wrappers |

## Install it into a profile

The package is a **profile bundle**: its own `cordis.patch.yml` inserts its row, so installing it
is one command and needs no edit to your patch layer.

```powershell
$dsh = "$env:LOCALAPPDATA\Programs\DeepSeek Harness\resources\runtime\cli\bin\dsh.cmd"
& $dsh plugin --profile desktop add "link:D:/GitHub/tsare/dsh/tsare-gate"
```

`link:` is pnpm's symlink protocol: the profile points at this directory, so a change to the plugin
is live on the next start without reinstalling. The same command accepts `file:` tarballs,
`github:owner/repo`, and a registry name once the package is published.

Restart the app. Rollback is one command:

```powershell
& $dsh plugin --profile desktop remove tsare-gate
```

To change the config, override the row by id in the profile's own `cordis.patch.yml`:

```yaml
- id: tsare-gate
  config:
    binary: tsare
    ledger: .tsare/session.tsare
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