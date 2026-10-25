import unittest

from syntax import VNum, VHole, parse

FACT = 'fact F1 { claim: "x", evidence: user }\n'


class LexerTest(unittest.TestCase):
    def test_unit_literal_is_one_token(self):
        decls, errors = parse('fact F1 { claim: "x", evidence: tool("t"), attrs: { frame_ms: 8ms } }\n')
        self.assertEqual(errors, [])
        attrs = decls[0].fields["attrs"]
        value = attrs.fields["frame_ms"]
        self.assertIsInstance(value, VNum)
        self.assertEqual(value.value, 8.0)
        self.assertEqual(value.unit, "ms")

    def test_comments_are_ignored(self):
        decls, errors = parse("# a comment\n" + FACT)
        self.assertEqual(errors, [])
        self.assertEqual(len(decls), 1)

    def test_hole_parses_as_unknown(self):
        decls, errors = parse('value V1 { metric: "m", direction: min, unit: "ms", threshold: ? }\n')
        self.assertEqual(errors, [])
        self.assertIsInstance(decls[0].fields["threshold"], VHole)


class ParserTest(unittest.TestCase):
    def test_kind_comes_first(self):
        decls, errors = parse(FACT)
        self.assertEqual(errors, [])
        self.assertEqual(decls[0].kind, "fact")
        self.assertEqual(decls[0].name, "F1")

    def test_list_and_call_values(self):
        decls, errors = parse(
            'concept C1 { name: "c", grounded_by: [F1, F2], against: [] }\n'
            + FACT
            + FACT.replace("F1", "F2")
        )
        self.assertEqual(errors, [])
        self.assertEqual(decls[0].fields["grounded_by"].items[0].name, "F1")

    def test_syntax_error_is_reported_with_position(self):
        decls, errors = parse("fact F1 { claim: }\n")
        self.assertTrue(errors)
        self.assertGreaterEqual(errors[0][1].line, 1)

    def test_recovery_continues_after_a_bad_declaration(self):
        decls, errors = parse("fact F1 { claim: }\n" + FACT)
        self.assertTrue(errors)
        self.assertEqual(len(decls), 1)


if __name__ == "__main__":
    unittest.main()
