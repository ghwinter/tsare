import unittest
from pathlib import Path

import cost
import egraph
import whatif
from argue import conclusions, grounded
from check import check
from syntax import parse

BASE = Path(__file__).resolve().parent.parent

DOC = """
fact F1 { claim: "x", evidence: user }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r1", from: [F1], to: P1 }
warrant W2 { claim: "r2", from: [F1], to: P1, attacks: [W1] }
"""


def program(src: str):
    decls, errors = parse(src)
    assert not errors, errors
    prog, _ = check(decls, BASE)
    return decls, prog


class ArgumentTest(unittest.TestCase):
    def test_attack_defeats_the_target(self):
        _, prog = program(DOC)
        inside, outside, _ = grounded(prog)
        self.assertIn("W2", inside)
        self.assertIn("W1", outside)

    def test_conclusion_survives_on_the_other_rule(self):
        _, prog = program(DOC)
        status = {name: state for name, state, _ in conclusions(prog)}
        self.assertEqual(status["P1"], "supported")

    def test_rule_without_premises_licenses_nothing(self):
        src = """
fact F1 { claim: "x", evidence: user }
proposition P1 { statement: "a", from: [F1] }
warrant W1 { claim: "r", from: [], to: P1 }
"""
        _, prog = program(src)
        inside, outside, _ = grounded(prog)
        self.assertNotIn("W1", inside)
        self.assertIn("W1", outside)


class CostTest(unittest.TestCase):
    def test_model_fact_costs_one_confirmation(self):
        _, prog = program('fact F1 { claim: "x", evidence: model }\n')
        total, _, rows, _ = cost.report(prog)
        self.assertEqual(total, 1)
        self.assertEqual(rows[0][0], "F1")

    def test_supported_document_has_zero_cost(self):
        _, prog = program(
            'fact F1 { claim: "x", evidence: tool("t") }\n'
            'proposition P1 { statement: "a", from: [F1] }\n'
            'warrant W1 { claim: "r", from: [F1], to: P1, basis: [V1] }\n'
            'value V1 { metric: "m", direction: min, unit: "ms", threshold: 1, measured_by: tool("t") }\n'
        )
        total, _, _, _ = cost.report(prog)
        self.assertEqual(total, 0)


class WhatIfTest(unittest.TestCase):
    def test_removing_the_only_premise_defeats_the_conclusion(self):
        decls, prog = program(DOC)
        before = {name: state for name, state, _ in conclusions(prog)}
        reduced, error = whatif.remove(decls, "F1")
        self.assertIsNone(error)
        prog_after, _ = check(reduced, BASE)
        after = {name: state for name, state, _ in conclusions(prog_after)}
        self.assertEqual(before["P1"], "supported")
        self.assertNotEqual(after["P1"], "supported")


class EGraphTest(unittest.TestCase):
    def test_identical_facts_are_merged(self):
        decls, prog = program(
            'fact F1 { claim: "x", evidence: user }\n'
            'fact F2 { claim: "x", evidence: user }\n'
        )
        uf, groups = egraph.merge_groups(prog)
        merged = [group for group in groups if len(group) > 1]
        self.assertEqual(len(merged), 1)
        self.assertEqual(sorted(merged[0]), ["F1", "F2"])


if __name__ == "__main__":
    unittest.main()
