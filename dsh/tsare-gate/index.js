/**
 * DSH plugin: run the checker inside the loop instead of asking the model to remember it.
 *
 * A skill is a prompt, and a prompt is a probability: whether tsare gets called and whether its
 * answer is obeyed both depend on the model's goodwill.  This plugin moves the gate to two seams
 * the model does not control:
 *
 *   agent/turn-stopping   the turn is held open (agent.steer) while the ledger is not checkable
 *   tools/post-execute    a dirty ledger rides along as context on the next request (opt-in)
 *
 * It writes nothing but the ledger template, and it runs no model.
 *
 * Config (all optional except `binary` when `command` is not given):
 *   binary             the tsare executable, resolved from PATH by default ('tsare')
 *   ledger             ledger path, relative to `cwd` (default 'SESSION.tsare')
 *   cwd                where the ledger lives (default: the process working directory)
 *   mode               'errors' (default) fails on errors, 'warnings' also on warnings (--strict)
 *   steerLimit         steers per agent before the gate gives up (default 3)
 *   timeoutMs          per check (default 10000)
 *   remindAfterTools   tool names that attach the diagnostics as context after they run (default [])
 *   command            argv override for tests and wrappers; replaces [binary, check, ledger, --json]
 */

import { execFile } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { existsSync, mkdirSync, readFileSync, writeFileSync } from 'node:fs'
import { dirname, join } from 'node:path'

export const name = 'tsare-gate'

/** A ledger is a document, and an empty document is an error, so it starts with one note. */
const TEMPLATE = `tsare 1
note N_LEDGER { tag: conjecture, text: "claims this session makes; the checker runs before the turn closes" }
`

const DEFAULTS = {
  binary: 'tsare',
  ledger: '.tsare/session.tsare',
  maxDeclarations: 150,
  cwd: '',
  mode: 'errors',
  steerLimit: 3,
  timeoutMs: 10_000,
  remindAfterTools: [],
  command: null,
}

/** The message a gate injects; the shape is the one `createUserMessage` builds. */
function notice(text) {
  return Object.freeze({
    id: randomUUID(),
    role: 'user',
    content: Object.freeze([{ type: 'text', text }]),
    source: { kind: 'tsare-gate', form: 'notice', summary: 'tsare gate' },
  })
}

/** Prepend our context while preserving whatever a later listener already attached. */
function prepend(ours, theirs) {
  return [ours, ...(theirs ?? [])]
}

/**
 * Run the checker once and classify the outcome.  Never throws: a plugin that crashes the turn
 * is worse than a plugin that reports it could not run.
 * @returns {Promise<{status: 'ok'|'dirty'|'unavailable', text: string}>}
 */
function runGate(config, cwd) {
  const argv = config.command ?? [
    config.binary,
    'check',
    config.ledger,
    '--json',
    ...(config.mode === 'warnings' ? ['--strict'] : []),
  ]
  cwd = cwd || config.cwd || process.cwd()
  const [file, ...args] = argv
  return new Promise((resolve) => {
    execFile(
      file,
      args,
      { cwd, timeout: config.timeoutMs, windowsHide: true, maxBuffer: 8 * 1024 * 1024 },
      (error, stdout, stderr) => {
        if (error && error.code === 'ENOENT') {
          resolve({ status: 'unavailable', text: `tsare gate: cannot run ${file} (ENOENT)` })
          return
        }
        let report = null
        try {
          report = JSON.parse(stdout)
        } catch {
          report = null
        }
        if (!error) {
          resolve({ status: 'ok', text: '' })
          return
        }
        if (!report || typeof report !== 'object') {
          const detail = String(stderr || error.message || '').trim().slice(0, 800)
          resolve({ status: 'unavailable', text: `tsare gate: ${file} printed no report\n${detail}` })
          return
        }
        const wanted = (item) =>
          item.code.startsWith('E') || (config.mode === 'warnings' && item.code.startsWith('W'))
        const rows = (report.diagnostics ?? []).filter(wanted).slice(0, 12)
        const lines = rows.map(
          (item) => `  ${item.code} ${item.name || '<file>'} line ${item.line}: ${item.message}`,
        )
        resolve({
          status: 'dirty',
          text:
            `tsare gate: ${config.ledger} is not checkable (${report.errors} error(s), ` +
            `${report.warnings} warning(s))\n${lines.join('\n')}\n` +
            'Fix the ledger before closing the turn. If a claim cannot be checked, say so in the ' +
            'answer and record the gap in the document; do not delete the declaration to silence it.',
        })
      },
    )
  })
}

/**
 * Install the gate.
 * @param ctx - plugin context; listeners are disposed with it.
 * @param raw - user config.
 */
export function apply(ctx, raw) {
  const config = { ...DEFAULTS, ...(raw ?? {}) }
  const steers = new Map()

  /** The directory the ledger lives in: the config, else the session's own working directory. */
  const cwdOf = (agent) => config.cwd || agent?.session?.header?.cwd || process.cwd()
  const ledgerPath = (agent) => join(cwdOf(agent), config.ledger)

  const ensureLedger = (agent) => {
    const path = ledgerPath(agent)
    const dir = dirname(path)
    mkdirSync(dir, { recursive: true })
    // The harness's own directory, not the project's.  It ignores itself, so a session in any
    // repository never dirties git status, and nothing has to edit the user's .gitignore.
    const ignore = join(dir, '.gitignore')
    if (!existsSync(ignore)) writeFileSync(ignore, '*\n', 'utf8')
    if (!existsSync(path)) writeFileSync(path, TEMPLATE, 'utf8')
    return path
  }

  /** How many declarations the active ledger holds, and a note when that is over the bound. */
  const growthNote = (agent) => {
    let text
    try {
      text = readFileSync(ledgerPath(agent), 'utf8')
    } catch {
      return ''
    }
    const count = (text.match(/^[a-z]+ [A-Z][A-Z0-9_]* \{/gm) ?? []).length
    if (count <= config.maxDeclarations) return ''
    return (
      '\nthe ledger has ' + count + ' declarations (limit ' + config.maxDeclarations + '): move ' +
      'retired ones to .tsare/archive.tsare, so the active document stays readable and cheap to ' +
      'check. Growth is not an error; it is a sign the session is remembering things it should ' +
      'retire.'
    )
  }

  // The only advisory part: a session that starts without a ledger gets one, so the gate has a
  // subject.  Everything else is enforced by the two listeners below.
  ctx.on('agent/created', ({ agent }) => {
    try {
      ensureLedger(agent)
    } catch {
      // a read-only working directory is not a reason to fail the agent
    }
  })

  ctx.on('tools/post-execute', async (exec, _result, next) => {
    const downstream = await next()
    if (!config.remindAfterTools.length || !config.remindAfterTools.includes(exec.name)) {
      return downstream
    }
    try {
      ensureLedger(exec.agent)
    } catch {
      return downstream
    }
    const verdict = await runGate(config, cwdOf(exec.agent))
    if (verdict.status !== 'dirty') return downstream
    const context = notice(verdict.text + growthNote(exec.agent))
    return { ...downstream, additionalContexts: prepend(context, downstream.additionalContexts) }
  })

  ctx.on('agent/turn-stopping', async ({ agent }) => {
    try {
      ensureLedger(agent)
    } catch {
      // reported as a steer below rather than thrown: the turn must still be able to close
    }
    const verdict = await runGate(config, cwdOf(agent))
    if (verdict.status === 'ok') {
      steers.delete(agent)
      return
    }
    const text =
      verdict.status === 'dirty'
        ? verdict.text
        : `${verdict.text}\nThe ledger was not checked. Say so in the answer instead of presenting the result as verified.`
    const full = text + growthNote(agent)
    const used = steers.get(agent) ?? 0
    if (used >= config.steerLimit) {
      // Bounded, and saturated: resetting the counter here would make the limit a periodic
      // allowance instead of a stop.  Only a clean ledger (above) resets it.
      return
    }
    steers.set(agent, used + 1)
    // `steer` reopens the turn: the machine runs another step and the model has to answer the
    // check instead of closing.  This is the only place the loop can be closed mechanically.
    agent.steer(notice(full))
  })
}
