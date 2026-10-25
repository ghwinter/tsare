import unittest
from pathlib import Path

import derive
from check import check
from syntax import parse

BASE = Path(__file__).resolve().parent.parent

DOC = """
fact F1 { claim: "x", evidence: tool("t") }
fact F2 { claim: "y", evidence: tool("t") }
concept C { name: "c", grounded_by: [F1], against: [F2] }
proposition P_A { statement: "a", uses: [C] }
proposition P_B { statement: "b", uses: [C] }
proposition P_OTHER { statement: "o" }
warrant W { claim: "r", from: [F1], concludes: { uses: [C] }, basis: [F1] }
"""


def codes(src: str) -> set[str]:
    decls, errors = parse(src)
    assert not errors, errors
    _, diags = check(decls, BASE)
    return {d.code for d in diags}


def analyze(src: str):
    decls, errors = parse(src)
    assert not errors, errors
    prog, _ = check(decls, BASE)
    return derive.analyze(prog)


class SchemaHeadTest(unittest.TestCase):
    def test_class_head_derives_every_match(self):
        result = analyze(DOC)
        self.assertIn("P_A", result["derived"])
        self.assertIn("P_B", result["derived"])

    def test_class_head_skips_non_matching_propositions(self):
        result = analyze(DOC)
        self.assertNotIn("P_OTHER", result["derived"])

    def test_warrant_needs_exactly_one_head(self):
        src = (
            'fact F1 { claim: "x", evidence: tool("t") }\n'
            'proposition P { statement: "p", from: [F1] }\n'
            'warrant W { claim: "r", from: [F1] }\n'
        )
        self.assertIn("E-WARRANT-MODE", codes(src))

    def test_warrant_cannot_have_both_heads(self):
        src = (
            'fact F1 { claim: "x", evidence: tool("t") }\n'
            'proposition P { statement: "p", from: [F1] }\n'
            'warrant W { claim: "r", from: [F1], to: P, concludes: { from: [F1] } }\n'
        )
        self.assertIn("E-WARRANT-MODE", codes(src))

    def test_demo_document(self):
        decls, errors = parse(Path("schema.mf").read_text(encoding="utf-8"))
        self.assertEqual(errors, [])
        prog, _ = check(decls, BASE)
        result = derive.analyze(prog)
        self.assertEqual(result["derived"], ["P_SMOOTH", "P_STABLE"])


if __name__ == "__main__":
    unittest.main()
