// End-to-end check against the real binary: the plugin's parser must agree with what
// `tsare check --json` actually prints.  TSARE_BIN names the executable (default: from PATH).
import assert from 'node:assert/strict'
import { mkdtempSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { apply } from './index.js'

const binary = process.env.TSARE_BIN ?? 'tsare'
const dir = mkdtempSync(join(tmpdir(), 'tsare-gate-real-'))

const handlers = new Map()
const ctx = { on(event, fn) { handlers.set(event, [...(handlers.get(event) ?? []), fn]) } }
const fire = async (event, payload) => {
  const out = []
  for (const fn of handlers.get(event) ?? []) out.push(await fn(payload))
  return out
}

apply(ctx, { binary })

// The plugin creates the directory and the template at session start; the test then edits it.
await fire('agent/created', { agent: { session: { header: { cwd: dir } } } })

// An empty ledger is a document with no declarations, which the checker refuses, and a fact
// without evidence is a real error: the message must carry the code and the declaration name.
writeFileSync(join(dir, '.tsare', 'session.tsare'), 'tsare 1\nfact F_X { claim: "nothing behind it" }\n')
const agent = { session: { header: { cwd: dir } }, steered: [], steer(message) { this.steered.push(message) } }
await fire('agent/turn-stopping', { agent, turn: 1, signal: new AbortController().signal })
assert.equal(agent.steered.length, 1, 'the real binary must have made the gate object')
const text = agent.steered[0].content[0].text
assert.match(text, /E-MISSING/, `the code is missing from: ${text}`)
assert.match(text, /F_X/, `the declaration name is missing from: ${text}`)

// A checkable ledger closes the turn.
writeFileSync(join(dir, '.tsare', 'session.tsare'), 'tsare 1\nnote N_OK { tag: conjecture, text: "the ledger is a note" }\n')
await fire('agent/turn-stopping', { agent, turn: 2, signal: new AbortController().signal })
assert.equal(agent.steered.length, 1, 'a clean ledger must not steer')

console.log(`tsare-gate against ${binary}: real binary agrees with the parser`)
