import unittest

import debt
from check import check
from syntax import parse

SRC = """
fact F_RAW { claim: "raw", evidence: model }
fact F_OK { claim: "ok", evidence: tool("t") }
proposition P_INNER { statement: "a", from: [F_RAW, F_OK] }
proposition P_OUTER { statement: "b", from: [P_INNER] }
warrant W1 { claim: "r", from: [P_INNER], to: P_OUTER }
"""


def program():
    decls, errors = parse(SRC)
    assert not errors, errors
    prog, _ = check(decls)
    return prog


class DebtTest(unittest.TestCase):
    def test_finds_the_unconfirmed_fact_through_a_proposition(self):
        rows = {name: (worst, lines) for name, worst, _, _, lines in debt.report(program())}
        self.assertIn("P_OUTER", rows)
        worst, lines = rows["P_OUTER"]
        self.assertEqual(worst, "unconfirmed")
        self.assertTrue(any("F_RAW" in line for line in lines))
        self.assertTrue(any("P_OUTER <- P_INNER <- F_RAW" in line for line in lines))

    def test_measured_claims_are_not_reported(self):
        decls, _ = parse('fact F_OK { claim: "ok", evidence: tool("t") }\n'
                         'proposition P_CLEAN { statement: "a", from: [F_OK] }\n')
        prog, _ = check(decls)
        self.assertEqual(debt.report(prog, only_gated=True), [])

    def test_walk_returns_paths(self):
        prog = program()
        edges = debt.support_edges(prog)
        facts = debt.walk(prog, "P_OUTER", edges)
        self.assertIn("F_RAW", facts)
        status, path = facts["F_RAW"]
        self.assertEqual(status, "unconfirmed")
        self.assertEqual(path[0], "P_OUTER")
        self.assertEqual(path[-1], "F_RAW")


if __name__ == "__main__":
    unittest.main()
