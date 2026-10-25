import unittest
from pathlib import Path

import derive
from check import check
from syntax import parse

GROUNDED = """
fact F1 { claim: "x", evidence: tool("t") }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r", from: [F1], to: P1 }
"""

CONDITIONAL = """
fact F1 { claim: "x", evidence: tool("t") }
fact F2 { claim: "y", evidence: tool("t") }
concept C { name: "c", grounded_by: [F1], against: [F2] }
proposition P_ASSUMED { statement: "a", uses: [C] }
proposition P_DERIVED { statement: "b", from: [P_ASSUMED] }
warrant W1 { claim: "r", from: [P_ASSUMED], to: P_DERIVED }
"""


def analyze(src: str):
    decls, errors = parse(src)
    assert not errors, errors
    prog, _ = check(decls, Path("."))
    return derive.analyze(prog)


class DeriveTest(unittest.TestCase):
    def test_grounded_chain_is_derived(self):
        result = analyze(GROUNDED)
        self.assertEqual(result["conditional"], [])
        self.assertIn("P1", result["derived"])

    def test_assumed_premise_makes_the_conclusion_conditional(self):
        result = analyze(CONDITIONAL)
        self.assertEqual(result["conditional"], ["P_DERIVED"])
        self.assertTrue(any("P_ASSUMED" in reason for reason in result["reasons"]["P_DERIVED"]))

    def test_demo_document_matches(self):
        decls, errors = parse(Path("ungrounded.mf").read_text(encoding="utf-8"))
        self.assertEqual(errors, [])
        prog, diags = check(decls, Path("."))
        self.assertEqual([d for d in diags if d.severity == "error"], [])
        result = derive.analyze(prog)
        self.assertEqual(result["conditional"], ["P_SMOOTH"])


if __name__ == "__main__":
    unittest.main()
