"""Run every command over every document, twice, and record what it said.

The output is tests/fixtures/harden/artifacts/matrix.jsonl: one record per (document, command) with the exit
status, a hash of stdout, and whether two runs produced identical bytes.  It is evidence, not
a test: the tests live in tests/, and this only says what the binary did on the corpus.

Usage:
    python scripts/matrix.py [--binary target/release/tsare.exe] [--out tests/fixtures/harden/artifacts/matrix.jsonl]
"""

import argparse
import hashlib
import json
import re
import subprocess
import sys
import tempfile
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
DOCS = ROOT / "docs"
FIXED = DOCS / "37-terse.tsare"


def first_name(text: str, kind: str | None = None) -> str | None:
    pattern = r"^(\w+)\s+([A-Za-z0-9_.]+)\s*\{" if kind is None else rf"^{kind}\s+([A-Za-z0-9_.]+)\s*\{{"
    for line in text.splitlines():
        match = re.match(pattern, line)
        if match:
            return match.group(2) if kind is None else match.group(1)
    return None


def commands(doc: Path, text: str, work: Path, report: Path, fixed_report: Path):
    name = doc.name
    shown = doc.relative_to(ROOT).as_posix()
    fixed = FIXED.relative_to(ROOT).as_posix()
    facts = [m.group(1) for m in re.finditer(r"^fact\s+([A-Za-z0-9_.]+)\s*\{", text, re.M)]
    concepts = [m.group(1) for m in re.finditer(r"^concept\s+([A-Za-z0-9_.]+)\s*\{", text, re.M)]
    decls = [m.group(2) for m in re.finditer(r"^(\w+)\s+([A-Za-z0-9_.]+)\s*\{", text, re.M)]
    out = work / (name + ".out")
    cases = [
        ("check", ["check", shown]),
        ("check-json", ["check", shown, "--json"]),
        ("check-strict", ["check", shown, "--strict"]),
        ("check-strict-json", ["check", shown, "--strict", "--json"]),
        ("check-against", ["check", shown, "--against", fixed]),
        ("engine", ["engine", shown]),
        ("engine-json", ["engine", shown, "--json"]),
        ("argue", ["argue", shown]),
        ("argue-against", ["argue", shown, "--against", fixed]),
        ("derive", ["derive", shown]),
        ("debt", ["debt", shown]),
        ("debt-all", ["debt", shown, "--all"]),
        ("impact", ["impact", shown]),
        ("next", ["next", shown]),
        ("next-json", ["next", shown, "--json"]),
        ("ask", ["ask", shown]),
        ("ask-one", ["ask", shown, "--one"]),
        ("ask-json-one", ["ask", shown, "--json", "--one"]),
        ("novel", ["novel", shown, "--attribute", "proposed"] + (["--holds", facts[0]] if facts else [])),
        ("shape", ["shape", shown]),
        ("shape-json", ["shape", shown, "--json"]),
        ("shape-layers", ["shape", shown, "--layers"]),
        ("shape-against", ["shape", shown, "--against", fixed, "--near", "2"]),
        ("imply", ["imply", shown, "--min-support", "2"]),
        ("abstract", ["abstract", shown]),
        ("cost", ["cost", shown]),
        ("cost-json", ["cost", shown, "--json"]),
        ("weigh", ["weigh", shown]),
        ("assumptions", ["assumptions", shown, "--top", "3"]),
        ("fca", ["fca", shown]),
        ("induce", ["induce", shown]),
        ("cat", ["cat", shown]),
        ("moves", ["moves", shown]),
        ("audit", ["audit", shown]),
        ("egraph", ["egraph", shown]),
        ("fix", ["fix", shown]),
        ("suggest", ["suggest", shown]),
        ("suggest-json", ["suggest", shown, "--json"]),
        ("apply-dry", ["apply", shown, "--dry-run"]),
        ("outline", ["outline", shown]),
        ("project", ["project", shown]),
        ("delta-identity", ["delta", str(report), str(report)]),
        ("delta-vs-fixed", ["delta", str(report), str(fixed_report)]),
        ("delta-json", ["delta", str(report), str(fixed_report), "--json"]),
        ("graph", ["graph", shown, "--out", str(out.with_suffix(".mmd"))]),
        ("build", ["build", shown, "--out", str(out.with_suffix(".build"))]),
        ("map", ["map", shown, fixed]),
        ("mem-empty", ["mem", "--store", str(work / "empty.jsonl")]),
    ]
    if "?" in text:
        cases.append(("propose", ["propose", shown, "--proposals", "tests/fixtures/harden/repro/proposals.json"]))
    if len(facts) >= 2:
        cases.append(("whatif", ["whatif", shown, facts[0]]))
    if concepts:
        cases.append(("pattern", ["pattern", shown, concepts[0]]))
    if len(decls) >= 2:
        cases.append(("meet", ["meet", shown, decls[0], decls[1]]))
        cases.append(("confront", ["confront", shown, shown]))
    return cases


def run(binary: str, args: list[str], cwd: Path) -> tuple[int, bytes]:
    completed = subprocess.run([binary] + args, cwd=cwd, capture_output=True)
    return completed.returncode, completed.stdout


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("--binary", default=str(ROOT / "target" / "release" / "tsare.exe"))
    parser.add_argument("--out", default=str(ROOT / "docs" / "artifacts" / "matrix.jsonl"))
    args = parser.parse_args()

    docs = sorted(DOCS.glob("*.tsare"))
    work = Path(tempfile.mkdtemp(prefix="tsare-matrix-"))
    records = []
    reports: dict[str, Path] = {}
    for doc in docs:
        report = work / (doc.name + ".check.json")
        code, out = run(args.binary, ["check", doc.relative_to(ROOT).as_posix(), "--json"], ROOT)
        report.write_bytes(out)
        reports[doc] = report
    fixed_report = reports[FIXED]

    for doc in docs:
        text = doc.read_text(encoding="utf-8")
        shown = doc.relative_to(ROOT).as_posix()
        for label, argv in commands(doc, text, work, reports[doc], fixed_report):
            outputs = [run(args.binary, argv, ROOT) for _ in range(4)]
            first_code, first = outputs[0]
            second_code, second = outputs[1]
            identical = len({(code, out) for code, out in outputs}) == 1
            stderr = ""
            codes: list[str] = []
            messages: dict[str, str] = {}
            if label in {"check-json", "check-strict-json"}:
                try:
                    payload = json.loads(first.decode("utf-8"))
                    codes = [item["code"] for item in payload.get("diagnostics", [])]
                    for item in payload.get("diagnostics", []):
                        messages.setdefault(item["code"], item.get("message", ""))
                except Exception:
                    codes = ["<unparseable>"]
            records.append(
                {
                    "doc": shown,
                    "command": label,
                    "argv": argv,
                    "exit": first_code,
                    "exit_second": second_code,
                    "sha256": hashlib.sha256(first).hexdigest(),
                    "identical": identical,
                    "lines": len(first.decode("utf-8", "replace").splitlines()),
                    "codes": codes,
                    "messages": messages,
                }
            )
    out = Path(args.out)
    out.parent.mkdir(parents=True, exist_ok=True)
    with out.open("w", encoding="utf-8") as handle:
        for record in records:
            handle.write(json.dumps(record, sort_keys=True) + "\n")
    print(f"{len(docs)} documents, {len(records)} command runs, wrote {out}")
    nondeterministic = [r for r in records if not r["identical"]]
    print(f"nondeterministic: {len(nondeterministic)}")
    for r in nondeterministic[:20]:
        print("  ", r["doc"], r["command"], r["exit"], r["exit_second"])
    return 0


if __name__ == "__main__":
    sys.exit(main())


