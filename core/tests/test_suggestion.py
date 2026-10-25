import unittest
from pathlib import Path

import fix
import suggestion
from syntax import parse

DOC = 'fact F { claim: "a", evidence: user }\nfact F { claim: "b", evidence: user }\n'


class SuggestionTest(unittest.TestCase):
    def test_deletion_points_at_the_removed_declaration(self):
        decls, errors = parse(DOC)
        self.assertEqual(errors, [])
        accepted = fix.repairs(decls, Path("."))
        self.assertTrue(accepted)
        description, trial, _ = accepted[0]
        item = suggestion.from_fix(decls, trial, True, description)
        self.assertEqual(item.replacement, "")
        self.assertEqual(item.span.line, 2)

    def test_changed_declaration_points_at_itself(self):
        decls, _ = parse('fact F { claim: "a", evidence: user }\n')
        trial, _ = parse('fact F { claim: "b", evidence: user }\n')
        item = suggestion.from_fix(decls, trial, True, "change the claim")
        self.assertEqual(item.span.line, 1)
        self.assertIn('"b"', item.replacement)


if __name__ == "__main__":
    unittest.main()
