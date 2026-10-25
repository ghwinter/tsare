import tempfile
import unittest
from pathlib import Path

import apply
import fix
import suggestion
from syntax import parse


class ApplyTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.base = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def test_machine_repair_is_applied_and_verified(self):
        text = 'fact F { claim: "a", evidence: user }\nfact F { claim: "b", evidence: user }\n'
        decls, _ = parse(text)
        items = [
            suggestion.from_fix(decls, trial, True, description)
            for description, trial, _ in fix.repairs(decls, self.base)
        ]
        before = apply.score(text, self.base)
        updated, applied = apply.apply_best(text, self.base, items)
        self.assertEqual(len(applied), 1)
        self.assertLess(apply.score(updated, self.base), before)
        self.assertNotIn('"b"', updated)

    def test_a_repair_that_does_not_improve_is_dropped(self):
        text = 'fact F { claim: "a", evidence: user }\n'
        decls, _ = parse(text)
        bad = suggestion.Suggestion("no-op", suggestion.Applicability.MachineApplicable, text, decls[0].span)
        updated, applied = apply.apply_best(text, self.base, [bad])
        self.assertEqual(applied, [])
        self.assertEqual(updated, text)


if __name__ == "__main__":
    unittest.main()
