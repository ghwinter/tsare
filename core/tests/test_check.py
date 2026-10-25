import unittest
from pathlib import Path

from check import check
from syntax import parse

BASE = Path(__file__).resolve().parent.parent


def codes(src: str) -> set[str]:
    decls, errors = parse(src)
    prog, diags = check(decls, BASE)
    return {d.code for d in diags}


FACT = 'fact F1 { claim: "x", evidence: user }\n'


class KindRulesTest(unittest.TestCase):
    def test_empty_document_is_an_error(self):
        self.assertIn("E-EMPTY-DOCUMENT", codes(""))

    def test_undefined_reference(self):
        src = FACT + 'proposition P1 { statement: "y", from: [F_MISSING] }\n'
        self.assertIn("E-UNDEFINED", codes(src))

    def test_wrong_kind_reference(self):
        src = (
            FACT
            + 'proposition P1 { statement: "y", from: [F1] }\n'
            + 'concept C1 { name: "c", grounded_by: [P1], against: [] }\n'
        )
        self.assertIn("E-KIND", codes(src))

    def test_cycle(self):
        src = (
            FACT
            + 'proposition P1 { statement: "a", from: [P2] }\n'
            + 'proposition P2 { statement: "b", from: [P1] }\n'
        )
        self.assertIn("E-CYCLE", codes(src))

    def test_collapse_of_two_concepts_with_one_extent(self):
        src = (
            FACT
            + 'fact F2 { claim: "z", evidence: user }\n'
            + 'concept C1 { name: "c1", grounded_by: [F1], against: [F2] }\n'
            + 'concept C2 { name: "c2", grounded_by: [F1], against: [F2] }\n'
        )
        self.assertIn("E-COLLAPSE", codes(src))

    def test_tension_on_one_metric(self):
        src = (
            'value V1 { metric: "ms", direction: min, unit: "ms", threshold: 16 }\n'
            'value V2 { metric: "ms", direction: max, unit: "ms", threshold: 60 }\n'
        )
        self.assertIn("E-TENSION", codes(src))

    def test_unfalsifiable_hypothesis(self):
        src = FACT + 'hypothesis H1 { assumes: [F1], predicts: "it works" }\n'
        self.assertIn("E-UNFALSIFIABLE", codes(src))

    def test_note_cannot_support_anything(self):
        src = (
            FACT
            + 'proposition P1 { statement: "y", from: [F1] }\n'
            + 'note N1 { tag: intuition, text: "feels right" }\n'
            + 'warrant W1 { claim: "r", from: [F1], to: P1, basis: [N1] }\n'
        )
        self.assertIn("E-NOTE-REF", codes(src))

    def test_confidence_out_of_range(self):
        src = 'fact F1 { claim: "x", evidence: user, confidence: 1.5 }\n'
        self.assertIn("E-CONFIDENCE", codes(src))

    def test_instrument_accepts_tool_call(self):
        src = (
            'value V1 { metric: "ms", direction: min, unit: "ms", threshold: 16, '
            'measured_by: tool("trace") }\n'
        )
        self.assertNotIn("E-TYPE", codes(src))

    def test_instrument_rejects_bare_identifier(self):
        src = 'value V1 { metric: "ms", direction: min, unit: "ms", threshold: 16, measured_by: experiment }\n'
        self.assertIn("E-TYPE", codes(src))



    def test_conditional_conclusion_is_a_warning(self):
        src = (
            'fact F1 { claim: "x", evidence: tool("t") }\n'
            'fact F2 { claim: "y", evidence: tool("t") }\n'
            'concept C { name: "c", grounded_by: [F1], against: [F2] }\n'
            'proposition P_A { statement: "a", uses: [C] }\n'
            'proposition P_B { statement: "b", from: [P_A] }\n'
            'warrant W { claim: "r", from: [P_A], to: P_B }\n'
        )
        self.assertIn("W-CONDITIONAL", codes(src))

    def test_grounded_document_has_no_conditional_warning(self):
        src = (
            'fact F1 { claim: "x", evidence: tool("t") }\n'
            'proposition P1 { statement: "a", from: [F1] }\n'
            'warrant W1 { claim: "r", from: [F1], to: P1 }\n'
        )
        self.assertNotIn("W-CONDITIONAL", codes(src))


if __name__ == "__main__":
    unittest.main()
