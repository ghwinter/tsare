// A stand-in for `tsare check LEDGER --json` and `tsare ask LEDGER --one`, so the plugin can be
// tested without the real binary.  The ledger is "clean" when it contains the word OK, and it has
// a question when it contains ASK.
import { readFileSync } from 'node:fs'

const args = process.argv.slice(2)
const ledger = process.argv.find((item) => item.endsWith('.tsare'))
const text = ledger ? readFileSync(ledger, 'utf8') : ''

if (args[0] === 'ask') {
  process.stdout.write(
    text.includes('ASK')
      ? 'does F_GAP record weight?\n'
      : 'nothing to ask: every list covers its extent and nothing collides\n',
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
