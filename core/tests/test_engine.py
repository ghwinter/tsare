import unittest
from pathlib import Path

import engine
from check import check
from syntax import parse

BASE = Path(__file__).resolve().parent.parent

CASCADE = """
fact F { claim: "trace", evidence: tool("trace") }
proposition P_A { statement: "a", from: [F] }
proposition P_N { statement: "n", from: [F] }
proposition P_B { statement: "b", from: [P_A] }
warrant W_GOOD { claim: "ok", from: [F], to: P_A, basis: [F] }
warrant W_BAD { claim: "noisy", from: [F], to: P_N, basis: [F], attacks: [W_GOOD] }
warrant W_LINK { claim: "link", from: [P_A], to: P_B, basis: [F] }
"""


def run(src: str) -> dict:
    decls, errors = parse(src)
    assert not errors, errors
    prog, _ = check(decls, BASE)
    return engine.run(prog)


class EngineTest(unittest.TestCase):
    def test_defeated_premise_blocks_the_conclusion(self):
        result = run(CASCADE)
        self.assertEqual(result["conditional"], ["P_B"])
        self.assertEqual(result["held"], ["P_N"])

    def test_chain_names_the_rule_and_the_fact(self):
        result = run(CASCADE)
        self.assertIn("W_BAD", result["chains"]["P_N"])
        self.assertIn("F", result["chains"]["P_N"])

    def test_blocked_rule_lists_the_missing_premise(self):
        result = run(CASCADE)
        self.assertEqual(result["dead_rules"], [("W_LINK", ["P_A"])])

    def test_clean_document_has_no_conditional(self):
        decls, _ = parse(
            'fact F { claim: "x", evidence: tool("t") }\n'
            'proposition P { statement: "p", from: [F] }\n'
            'warrant W { claim: "r", from: [F], to: P, basis: [F] }\n'
        )
        prog, _ = check(decls, BASE)
        result = engine.run(prog)
        self.assertEqual(result["conditional"], [])
        self.assertEqual(result["held"], ["P"])


if __name__ == "__main__":
    unittest.main()
