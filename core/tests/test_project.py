import tempfile
import unittest
from pathlib import Path

from project import check_project


class ProjectTest(unittest.TestCase):
    def setUp(self):
        self.tmp = tempfile.TemporaryDirectory()
        self.dir = Path(self.tmp.name)

    def tearDown(self):
        self.tmp.cleanup()

    def write(self, name: str, text: str) -> None:
        (self.dir / name).write_text(text, encoding="utf-8")

    def test_cross_file_reference_resolves(self):
        self.write("a.mf", 'fact F { claim: "x", evidence: tool("t") }\n')
        self.write("b.mf", 'proposition P { statement: "p", from: [F] }\n')
        _, errors = check_project(self.dir)
        self.assertEqual(errors, 0)

    def test_duplicate_names_report_both_files(self):
        self.write("a.mf", 'fact F { claim: "x", evidence: user }\n')
        self.write("b.mf", 'fact F { claim: "y", evidence: user }\n')
        results, errors = check_project(self.dir)
        self.assertEqual(errors, 1)
        messages = " ".join(d.message for diags in results.values() for d in diags)
        self.assertIn("a.mf", messages)
        self.assertIn("b.mf", messages)

    def test_undefined_reference_belongs_to_its_file(self):
        self.write("a.mf", 'fact F { claim: "x", evidence: user }\n')
        self.write("b.mf", 'proposition P { statement: "p", from: [NOPE] }\n')
        results, _ = check_project(self.dir)
        owners = [
            path.name
            for path, diags in results.items()
            for d in diags
            if d.code == "E-UNDEFINED"
        ]
        self.assertEqual(owners, ["b.mf"])

    def test_layer_leak_is_attributed_to_the_offending_declaration(self):
        self.write(
            "core.mf",
            'layer L_CORE { name: "core", exposes: [F_OK] }\n'
            'fact F_OK { claim: "ok", evidence: tool("t"), layer: L_CORE }\n'
            'fact F_HIDDEN { claim: "hidden", evidence: tool("t"), layer: L_CORE }\n',
        )
        self.write(
            "app.mf",
            'layer L_APP { name: "app", exposes: [] }\n'
            'proposition P_LEAK { statement: "p", from: [F_HIDDEN], layer: L_APP }\n',
        )
        results, errors = check_project(self.dir)
        self.assertGreaterEqual(errors, 1)
        leaks = [
            (path.name, d.file)
            for path, diags in results.items()
            for d in diags
            if d.code == "E-LAYER-LEAK"
        ]
        self.assertEqual(leaks, [("app.mf", "app.mf")])


if __name__ == "__main__":
    unittest.main()
