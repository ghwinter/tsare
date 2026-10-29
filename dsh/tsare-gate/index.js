/**
 * DSH plugin: run the checker inside the loop instead of asking the model to remember it.
 *
 * A skill is a prompt, and a prompt is a probability: whether tsare gets called and whether its
 * answer is obeyed both depend on the model's goodwill.  This plugin moves the gate to seams the
 * model does not control:
 *
 *   agent/turn-stopping   the turn is held open (agent.steer) while the ledger is not checkable
 *   systemPrompt.context  one standing line: the ledger's next question, replaced in place
 *   tools/post-execute    a dirty ledger or a verification command rides along as context (opt-in)
 *
 * It writes nothing but the ledger template, and it runs no model.
 *
 * Config (all optional except `binary` when `command` is not given):
 *   binary             the tsare executable, resolved from PATH by default ('tsare')
 *   ledger             ledger path, relative to `cwd` (default '.tsare/session.tsare')
 *   cwd                where the ledger lives (default: the process working directory)
 *   mode               'errors' (default) fails on errors, 'warnings' also on warnings (--strict)
 *   steerLimit         steers per agent before the gate gives up (default 3)
 *   timeoutMs          per check (default 10000)
 *   brief              keep one line in the runtime snapshot: the next question (default true)
 *   briefMaxChars      cap for that line (default 240)
 *   briefStyle         'full' (default) names the next action; 'terse' states only the state
 *   remindAfterTools   tool names that attach the diagnostics as context after they run (default [])
 *   remindWhen         regexes matched against a tool call's arguments; a match gets one reminder
 *                      that the result is a claim (default [])
 *   remindLimit        how many such reminders one agent may receive (default 5)
 *   command            argv override for tests and wrappers; replaces [binary, check, ledger, --json]
 *   askCommand         argv override for the next-question call (default [binary, ask, ledger, --one])
 *   debug              append one line per hook to <tmp>/tsare-gate.log (default false)
 */

import { execFile } from 'node:child_process'
import { randomUUID } from 'node:crypto'
import { appendFileSync, existsSync, mkdirSync, readFileSync, statSync, writeFileSync } from 'node:fs'
import { tmpdir } from 'node:os'
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
  brief: true,
  briefMaxChars: 240,
  briefStyle: 'full',
  remindAfterTools: [],
  remindWhen: [],
  remindLimit: 5,
  command: null,
  askCommand: null,
  debug: false,
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

/** One rendered line: no newlines, no runs of spaces, bounded. */
function oneLine(text, limit) {
  const cap = Number.isFinite(limit) && limit > 0 ? limit : 240
  const flat = String(text).replace(/\s+/g, ' ').trim()
  return flat.length > cap ? `${flat.slice(0, cap - 1)}…` : flat
}

/** Run one child and classify it.  Never throws: a plugin that crashes the turn is worse. */
function run(argv, config, cwd) {
  const [file, ...args] = Array.isArray(argv) ? argv : []
  if (typeof file !== 'string' || file.length === 0) {
    const error = new Error('tsare gate: the command names no executable')
    return Promise.resolve({ error, stdout: '', stderr: error.message })
  }
  return new Promise((resolve) => {
    try {
      execFile(
        file,
        args,
        {
          cwd,
          timeout: Number.isFinite(config.timeoutMs) ? config.timeoutMs : 10_000,
          windowsHide: true,
          maxBuffer: 8 * 1024 * 1024,
        },
        (error, stdout, stderr) => resolve({ error, stdout: String(stdout ?? ''), stderr: String(stderr ?? '') }),
      )
    } catch (error) {
      // execFile throws synchronously for a null cwd or a bad option; the turn must survive it.
      resolve({ error, stdout: '', stderr: String(error?.message ?? error) })
    }
  })
}

/** The check report, or the sentence that says why there is none. */
async function checkLedger(config, cwd) {
  const argv = config.command ?? [
    config.binary,
    'check',
    config.ledger,
    '--json',
    ...(config.mode === 'warnings' ? ['--strict'] : []),
  ]
  const { error, stdout, stderr } = await run(argv, config, cwd)
  let report = null
  try {
    report = JSON.parse(stdout)
  } catch {
    report = null
  }
  if (!error) return { status: 'ok', text: '', report }
  if (error.code === 'ENOENT') {
    return { status: 'unavailable', text: `tsare gate: cannot run ${argv[0]} (ENOENT)`, report: null }
  }
  if (!report || typeof report !== 'object') {
    const detail = String(stderr || error.message || '').trim().slice(0, 800)
    return { status: 'unavailable', text: `tsare gate: ${argv?.[0] ?? 'the checker'} printed no report\n${detail}`, report: null }
  }
  // A checker is a separate process; its report is input, not a promise.  Every field is read
  // defensively because the turn must survive a malformed one.
  const rowsAll = Array.isArray(report.diagnostics) ? report.diagnostics : []
  const codeOf = (item) => (item && typeof item === 'object' && typeof item.code === 'string' ? item.code : '')
  const errors = typeof report.errors === 'number' ? report.errors : rowsAll.filter((item) => codeOf(item).startsWith('E')).length
  const warnings = typeof report.warnings === 'number' ? report.warnings : rowsAll.filter((item) => codeOf(item).startsWith('W')).length
  const wanted = (item) => codeOf(item).startsWith('E') || (config.mode === 'warnings' && codeOf(item).startsWith('W'))
  const rows = rowsAll.filter(wanted).slice(0, 12)
  const lines = rows.map(
    (item) => `  ${item.code} ${item.name || '<file>'} line ${item.line ?? '?'}: ${item.message ?? ''}`,
  )
  return {
    status: 'dirty',
    text:
      `tsare gate: ${config.ledger} is not checkable (${errors} error(s), ` +
      `${warnings} warning(s))\n${lines.join('\n')}\n` +
      'Fix the ledger before closing the turn. If a claim cannot be checked, say so in the ' +
      'answer and record the gap in the document; do not delete the declaration to silence it.',
    report,
  }
}

/** The next question the ledger needs, or null when there is none (or it cannot run). */
async function nextQuestion(config, cwd) {
  const argv = config.askCommand ?? (config.command ? null : [config.binary, 'ask', config.ledger, '--one'])
  if (!argv) return null
  const { error, stdout } = await run(argv, config, cwd)
  if (error) return null
  const text = String(stdout).trim()
  if (!text || text.startsWith('nothing to ask')) return null
  return text.split('\n')[0]
}

/**
 * Install the gate.
 * @param ctx - plugin context; listeners are disposed with it.
 * @param raw - user config.
 */
export function apply(ctx, raw) {
  const config = { ...DEFAULTS, ...(raw ?? {}) }
  const steers = new Map()
  /** agent -> {stamp, text}; the runtime context reads `text`, the post-execute hook refreshes it. */
  const briefs = new Map()
  const briefFibers = new Map()
  /** agent -> Set of ledger stamps already reminded, never more than `remindLimit` in total. */
  const remanded = new Map()
  /** A live integration needs evidence about which hook fired; this is the only file it writes. */
  const trace =
    config.debug === true || process.env.TSARE_GATE_DEBUG === '1'
      ? (line) => {
          try {
            appendFileSync(join(tmpdir(), 'tsare-gate.log'), `${new Date().toISOString()} ${line}\n`)
          } catch {
            // a trace that cannot be written is not a reason to fail a turn
          }
        }
      : () => {}
  trace(`apply pid=${process.pid} cwd=${process.cwd()} ledger=${config.ledger} brief=${config.brief}`)

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

  /** A cheap identity for the ledger's current bytes; null when it cannot be read. */
  const stampOf = (agent) => {
    try {
      const info = statSync(ledgerPath(agent))
      return `${info.mtimeMs}:${info.size}`
    } catch {
      return ''
    }
  }

  const countDecls = (agent) => {
    try {
      return (readFileSync(ledgerPath(agent), 'utf8').match(/^[a-z]+ [A-Z][A-Z0-9_]* \{/gm) ?? []).length
    } catch {
      return 0
    }
  }

  /** The one line the runtime snapshot carries.  It changes only when the ledger does. */
  const composeBrief = async (agent) => {
    const verdict = await checkLedger(config, cwdOf(agent))
    if (verdict.status === 'unavailable') {
      return oneLine(`${verdict.text} The ledger was not checked.`, config.briefMaxChars)
    }
    if (verdict.status === 'dirty') {
      const rows = Array.isArray(verdict.report?.diagnostics)
        ? verdict.report.diagnostics.filter((item) => item && typeof item.code === 'string')
        : []
      const first = rows.find((item) => item.code.startsWith('E')) ?? rows[0]
      const detail = first
        ? `first ${first.code} ${first.name || '<file>'}: ${first.message ?? ''}`
        : 'no diagnostic rows'
      return oneLine(
        `tsare: ledger not checkable (${verdict.report?.errors ?? 0} error(s), ` +
          `${verdict.report?.warnings ?? 0} warning(s)); ${detail}. Fix it, or record why it cannot be checked.`,
        config.briefMaxChars,
      )
    }
    const question = await nextQuestion(config, cwdOf(agent))
    if (question) {
      return oneLine(
        `tsare: next question — ${question} (ledger ${config.ledger})`,
        config.briefMaxChars,
      )
    }
    const many = countDecls(agent)
    // `terse` states the state and nothing else; `full` also tells the model what to do next.
    // The difference is one sentence on every step, which is what the cost experiment varies.
    const tail =
      config.briefStyle === 'terse'
        ? ''
        : ' Record a claim with evidence, or say unchecked with the reason.'
    return oneLine(
      `tsare: ledger clean, ${many} claim(s); nothing to ask.${tail}`,
      config.briefMaxChars,
    )
  }

  /** Recompute the brief only when the ledger's bytes changed.  Never throws. */
  const refreshBrief = async (agent) => {
    if (!config.brief || !agent) return
    const stamp = stampOf(agent)
    const held = briefs.get(agent)
    if (held && held.stamp === stamp) return
    try {
      const text = await composeBrief(agent)
      briefs.set(agent, { stamp, text })
    } catch {
      // a brief that cannot be built is a missing line, not a failed turn
    }
  }

  /** The regex list is user input: one invalid pattern must not silence the rest. */
  const matchesRemind = (haystack) => {
    if (!Array.isArray(config.remindWhen)) return false
    for (const pattern of config.remindWhen) {
      if (typeof pattern !== 'string' || pattern.length === 0) continue
      try {
        if (new RegExp(pattern, 'i').test(haystack)) return true
      } catch {
        // an invalid pattern is skipped, not fatal
      }
    }
    return false
  }

  // The only advisory part: a session that starts without a ledger gets one, and a session with
  // one gets a standing line.  Everything else is enforced by the listeners below.
  const installBrief = (agent) => {
    try {
      ensureLedger(agent)
    } catch {
      // a read-only working directory is not a reason to fail the agent
    }
    if (!config.brief) return
    // A duplicate registration throws; one agent gets one line even if the event fires again.
    if (briefFibers.has(agent)) return
    let registered = false
    try {
      const fiber = agent.ctx.inject(['systemPrompt'], (scope) => {
        scope.systemPrompt.context({
          name: 'tsare-gate:ledger',
          order: 900,
          text: () => briefs.get(agent)?.text ?? '',
        })
      })
      briefFibers.set(agent, fiber)
      registered = true
    } catch {
      // no system prompt service here: the gate still works, the brief is just absent
    }
    trace(`install cwd=${cwdOf(agent)} ctx=${Boolean(agent?.ctx)} registered=${registered}`)
    void refreshBrief(agent)
  }

  ctx.on('agent/created', ({ agent }) => {
    trace('agent/created')
    installBrief(agent)
  })
  // A plugin loaded into a live process (HMR) must reach the agents that already exist; the
  // event only covers agents created after the load.
  try {
    const agents = ctx.get?.('agents')?.list?.() ?? []
    trace(`catch-up agents=${agents.length}`)
    for (const agent of agents) installBrief(agent)
  } catch (error) {
    trace(`catch-up failed: ${error?.message ?? error}`)
  }

  ctx.on('agent/disposed', ({ agent }) => {
    const fiber = briefFibers.get(agent)
    briefFibers.delete(agent)
    briefs.delete(agent)
    remanded.delete(agent)
    if (fiber) {
      try {
        void fiber.dispose().catch(() => {})
      } catch {
        // disposal races with unload; nothing to report
      }
    }
  })

  ctx.on('tools/post-execute', async (exec, _result, next) => {
    const downstream = await next()
    trace(`post-execute ${exec.name}`)
    // A resumed agent never fires agent/created; the first tool call is its registration point.
    if (exec.agent) installBrief(exec.agent)
    // The ledger changes only through tools, so this is the one place the brief needs to look.
    try {
      await refreshBrief(exec.agent)
    } catch {
      // refreshBrief already absorbs its own failures
    }
    let decision = downstream
    // Opt-in: the named tools attach the current diagnostics, dirty only, never blocking.
    const afterTools = Array.isArray(config.remindAfterTools) ? config.remindAfterTools : []
    if (afterTools.length && afterTools.includes(exec.name)) {
      try {
        ensureLedger(exec.agent)
        const verdict = await checkLedger(config, cwdOf(exec.agent))
        if (verdict.status === 'dirty') {
          const context = notice(verdict.text + growthNote(exec.agent))
          decision = {
            ...decision,
            additionalContexts: prepend(context, decision.additionalContexts),
          }
        }
      } catch {
        // a reminder is optional; a failed one must not fail the tool call
      }
    }
    if (!Array.isArray(config.remindWhen) || !config.remindWhen.length) return decision
    const agent = exec.agent
    if (!agent) return decision
    // Only a command can be a verification run.  Matching the whole argument object made a
    // write whose *file content* mentioned `cargo test` look like a test run.
    const command = typeof exec.arguments?.command === 'string' ? exec.arguments.command : ''
    if (!command) return decision
    if (!matchesRemind(`${exec.name} ${command}`)) return decision
    const seen = remanded.get(agent) ?? new Set()
    const limit = Number.isFinite(config.remindLimit) && config.remindLimit >= 0 ? config.remindLimit : 5
    if (seen.size >= limit) return decision
    const stamp = stampOf(agent)
    if (seen.has(stamp)) return decision
    seen.add(stamp)
    remanded.set(agent, seen)
    const context = notice(
      `tsare: that ${exec.name} result is a claim. Record it as a fact with evidence: tool(...) ` +
        `in ${config.ledger}, or say unchecked with the reason.`,
    )
    return { ...decision, additionalContexts: prepend(context, decision.additionalContexts) }
  })

  ctx.on('agent/turn-stopping', async ({ agent }) => {
    try {
      ensureLedger(agent)
    } catch {
      // reported as a steer below rather than thrown: the turn must still be able to close
    }
    // The last chance to register the line for a resumed agent: this turn's text may be empty,
    // but the next turn's snapshot will carry it.
    installBrief(agent)
    void refreshBrief(agent)
    const verdict = await checkLedger(config, cwdOf(agent))
    trace(`turn-stopping status=${verdict.status}`)
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
    const limit = Number.isFinite(config.steerLimit) && config.steerLimit >= 0 ? config.steerLimit : 3
    if (used >= limit) {
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
