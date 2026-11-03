// A stand-in for `tsare check LEDGER --json`, `tsare status LEDGER --json` and `tsare decide
// LEDGER --json`, so the plugin can be tested without the real binary.  The ledger is "clean"
// when it contains the word OK, it has a question when it contains ASK, and it has an open debt
// when it contains OPEN_DEBT.
import { appendFileSync, readFileSync } from 'node:fs'

const args = process.argv.slice(2)
const ledger = process.argv.find((item) => item.endsWith('.tsare'))
const text = ledger ? readFileSync(ledger, 'utf8') : ''

if (args[0] === 'status') {
  const report = {
    decision: 'closed',
    verified: 1,
    inferred: 1,
    unverified: 2,
    errors: 0,
    warnings: 0,
    claims: 1,
    question: null,
    first: null,
    debt: null,
  }
  if (text.includes('OPEN_DEBT')) {
    report.decision = 'open'
    report.debt = { name: 'D1', kind: 'fact', reason: "a model's word, with no confirmation named" }
  } else if (text.includes('ASK')) {
    report.decision = 'closable'
    report.question = 'does F_GAP record weight?'
  } else if (!text.includes('OK')) {
    report.decision = 'open'
    report.errors = 1
    report.warnings = 1
    report.verified = 0
    report.inferred = 0
    report.unverified = 0
    report.first = { code: 'E-MISSING', name: 'F_X', message: 'F_X needs evidence' }
  }
  const traceAt = args.indexOf('--trace')
  if (traceAt >= 0) {
    try {
      appendFileSync(args[traceAt + 1], 'run\n')
    } catch {
      // the fake cannot promise the caller's file either
    }
  }
  process.stdout.write(`${JSON.stringify(report)}\n`)
  process.exit(report.errors > 0 ? 1 : 0)
}

if (args[0] === 'decide') {
  process.stdout.write(
    text.includes('OPEN_DEBT')
      ? '{"decision":"open","items":[{"name":"D1","kind":"fact","decision":"open","reason":"a model\'s word, with no confirmation named"}]}\n'
      : '{"decision":"closed","items":[]}\n',
  )
  process.exit(0)
}

if (text.includes('OK')) {
  process.stdout.write('{"diagnostics":[],"errors":0,"warnings":0}\n')
  process.exit(0)
}
process.stdout.write(
  `${JSON.stringify({
    diagnostics: [
      { code: 'E-MISSING', severity: 'error', name: 'F_X', file: ledger, line: 2, col: 1, message: 'F_X needs evidence' },
      { code: 'W-UNCONFIRMED', severity: 'warning', name: 'F_Y', file: ledger, line: 3, col: 1, message: 'F_Y was supplied by a model' },
    ],
    errors: 1,
    warnings: 1,
  })}\n`,
)
process.exit(1)
