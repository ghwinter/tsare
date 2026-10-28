// A stand-in for `tsare check LEDGER --json` so the plugin can be tested without the real binary.
// The ledger is "clean" when it contains the word OK.
const ledger = process.argv.find((item) => item.endsWith('.tsare'))
const text = ledger ? (await import('node:fs')).readFileSync(ledger, 'utf8') : ''
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
