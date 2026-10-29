// Self-test for the gate: a fake ctx, a fake checker, and the decisions the plugin produces.
// Run with: node selftest.mjs   (no dependencies, no DSH install needed)
import assert from 'node:assert/strict'
import { mkdtempSync, readFileSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
import { join } from 'node:path'

import { apply } from './index.js'

const fake = join(import.meta.dirname, 'fake-tsare.mjs')
const dir = mkdtempSync(join(tmpdir(), 'tsare-gate-'))
const ledger = join(dir, '.tsare', 'session.tsare')

/** A cordis-shaped context: `on` records handlers, everything else is unused here. */
function context() {
  const handlers = new Map()
  return {
    handlers,
    on(event, fn) {
      const list = handlers.get(event) ?? []
      list.push(fn)
      handlers.set(event, list)
    },
    async fire(event, payload, next = async () => ({ kind: 'accept' })) {
      const out = []
      for (const fn of handlers.get(event) ?? []) out.push(await fn(payload, undefined, next))
      return out
    },
  }
}

const command = [process.execPath, fake, 'check', '.tsare/session.tsare', '--json']
const askCommand = [process.execPath, fake, 'ask', '.tsare/session.tsare', '--one']
const dirty = 'tsare 1\nfact F_X { claim: "a claim with nothing behind it" }\n'
const clean = 'tsare 1\nnote N_OK { tag: conjecture, text: "OK" }\n'
const session = { header: { cwd: dir } }
const contexts = []

/** Every agent gets a ctx whose only job is to capture the runtime-context registration. */
function makeAgent() {
  return {
    session,
    steered: [],
    contexts,
    steer(message) {
      this.steered.push(message)
    },
    ctx: {
      inject(_services, fn) {
        fn({
          systemPrompt: {
            context(options) {
              contexts.push(options)
              return () => {}
            },
          },
        })
        return { dispose: async () => {} }
      },
    },
  }
}

const agent = makeAgent()
const ctx = context()
apply(ctx, {
  command,
  askCommand,
  remindAfterTools: ['bash'],
  steerLimit: 3,
  maxDeclarations: 1,
})

// 1. the session starts with a ledger, in the harness's own directory, which ignores itself
await ctx.fire('agent/created', { agent })
assert.match(readFileSync(ledger, 'utf8'), /^tsare 1\n/, 'the template was not written')
assert.equal(readFileSync(join(dir, '.tsare', '.gitignore'), 'utf8').trim(), '*', 'the directory does not ignore itself')

// 2. a dirty ledger holds the turn open, with the diagnostics in the message
writeFileSync(ledger, dirty)
await ctx.fire('agent/turn-stopping', { agent, turn: 1, signal: new AbortController().signal })
assert.equal(agent.steered.length, 1, 'a dirty ledger must steer')
assert.match(agent.steered[0].content[0].text, /E-MISSING F_X line 2/, 'the diagnostic is missing')
assert.equal(agent.steered[0].role, 'user', 'a steer is a user message')

// 3. a clean ledger lets the turn close
writeFileSync(ledger, clean)
await ctx.fire('agent/turn-stopping', { agent, turn: 1, signal: new AbortController().signal })
assert.equal(agent.steered.length, 1, 'a clean ledger must not steer')

// 4. growth over the configured bound is said out loud, and is not an error
writeFileSync(ledger, `${dirty}note N_MORE { tag: conjecture, text: "second" }\n`)
await ctx.fire('agent/turn-stopping', { agent, turn: 2, signal: new AbortController().signal })
assert.equal(agent.steered.length, 2, 'a dirty ledger still steers')
assert.match(agent.steered[1].content[0].text, /has 2 declarations \(limit 1\)/, 'growth was not reported')

// 5. post-execute attaches the diagnostics without vetoing the tool result
const [decision] = await ctx.fire('tools/post-execute', { name: 'bash', arguments: {}, agent })
assert.equal(decision.kind, 'accept', 'post-execute must not block')
assert.equal(decision.additionalContexts.length, 1, 'the reminder was not attached')
assert.match(decision.additionalContexts[0].content[0].text, /E-MISSING/, 'the context is not the report')

// 6. an unrelated tool is left alone
const [untouched] = await ctx.fire('tools/post-execute', { name: 'read', arguments: {}, agent })
assert.equal(untouched.additionalContexts, undefined, 'only configured tools are reminded')

// 7. the steer limit is finite: a checker that always objects cannot hold a turn forever
const other = makeAgent()
for (let turn = 0; turn < 6; turn += 1) {
  await ctx.fire('agent/turn-stopping', { agent: other, turn, signal: new AbortController().signal })
}
assert.equal(other.steered.length, 3, `expected 3 steers, got ${other.steered.length}`)

// 8. a checker that cannot run says so instead of passing silently
const broken = context()
apply(broken, { command: ['definitely-not-tsare', 'check', '.tsare/session.tsare', '--json'] })
const missing = makeAgent()
await broken.fire('agent/turn-stopping', { agent: missing, turn: 1, signal: new AbortController().signal })
assert.equal(missing.steered.length, 1, 'an unavailable checker must steer')
assert.match(missing.steered[0].content[0].text, /cannot run/, 'the reason is not visible')

// 9. the brief is registered under a stable name, and renders a string
assert.ok(contexts.length >= 1, `expected at least one runtime context, got ${contexts.length}`)
assert.equal(contexts[0].name, 'tsare-gate:ledger')
assert.equal(typeof contexts[0].text, 'function', 'the brief must be a function')
assert.equal(typeof contexts[0].text(), 'string', 'the brief must render a string')

// 10. after a tool writes a clean ledger with a question, the brief carries that question
writeFileSync(ledger, clean.replace('OK', 'OK ASK'))
await ctx.fire('tools/post-execute', { name: 'write', arguments: { file: 'x' }, agent })
assert.match(contexts[0].text(), /next question/, contexts[0].text())
assert.match(contexts[0].text(), /does F_GAP record weight/, contexts[0].text())

// 11. a dirty ledger replaces the question with the first error
writeFileSync(ledger, dirty)
await ctx.fire('tools/post-execute', { name: 'write', arguments: { file: 'x' }, agent })
assert.match(contexts[0].text(), /not checkable/, contexts[0].text())
assert.match(contexts[0].text(), /E-MISSING/, contexts[0].text())

// 12. a verification command gets one reminder per ledger state, and an unrelated one gets none
const remindCtx = context()
apply(remindCtx, { command, askCommand, remindWhen: ['cargo (test|build)'], remindLimit: 2 })
const remindAgent = makeAgent()
await remindCtx.fire('agent/created', { agent: remindAgent })
writeFileSync(ledger, clean)
const [first] = await remindCtx.fire('tools/post-execute', {
  name: 'pwsh',
  arguments: { command: 'cargo test --release' },
  agent: remindAgent,
})
assert.equal(first.additionalContexts.length, 1, 'the verification reminder was not attached')
assert.match(first.additionalContexts[0].content[0].text, /claim/, 'the reminder does not name the claim')
const [second] = await remindCtx.fire('tools/post-execute', {
  name: 'pwsh',
  arguments: { command: 'cargo test --release' },
  agent: remindAgent,
})
assert.equal(second.additionalContexts, undefined, 'the same ledger state must not be reminded twice')
const [unrelated] = await remindCtx.fire('tools/post-execute', {
  name: 'pwsh',
  arguments: { command: 'git status' },
  agent: remindAgent,
})
assert.equal(unrelated.additionalContexts, undefined, 'an unrelated command must not be reminded')
const [written] = await remindCtx.fire('tools/post-execute', {
  name: 'write',
  arguments: { file: 'notes.md', content: 'run `cargo test` and record it' },
  agent: remindAgent,
})
assert.equal(written.additionalContexts, undefined, 'a file whose content mentions a command is not a run')
writeFileSync(ledger, `${clean}note N_MORE { tag: conjecture, text: "OK" }\n`)
const [third] = await remindCtx.fire('tools/post-execute', {
  name: 'pwsh',
  arguments: { command: 'cargo build' },
  agent: remindAgent,
})
assert.equal(third.additionalContexts.length, 1, 'a new ledger state may be reminded again')

// 13. a malformed report is data, not a promise: a non-object row must not reject the turn
const malformed = [
  process.execPath,
  '-e',
  'process.stdout.write(JSON.stringify({diagnostics:[null],errors:1,warnings:0}));process.exit(1)',
]
const malformedCtx = context()
apply(malformedCtx, { command: malformed })
const malformedAgent = makeAgent()
await malformedCtx.fire('agent/turn-stopping', {
  agent: malformedAgent,
  turn: 1,
  signal: new AbortController().signal,
})
assert.equal(malformedAgent.steered.length, 1, 'a malformed report must still steer')
assert.match(malformedAgent.steered[0].content[0].text, /not checkable/, 'the malformed report had no words')

// 14. a null binary is a config mistake, not a failed turn
const nullCtx = context()
apply(nullCtx, { binary: null, command: null })
const nullAgent = makeAgent()
await nullCtx.fire('agent/turn-stopping', {
  agent: nullAgent,
  turn: 1,
  signal: new AbortController().signal,
})
assert.equal(nullAgent.steered.length, 1, 'a null binary must steer, not throw')
assert.match(nullAgent.steered[0].content[0].text, /not checked|no executable/, 'the reason is not visible')

console.log('tsare-gate selftest: 14 checks passed')
