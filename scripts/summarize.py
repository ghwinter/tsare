"""Summarise tests/fixtures/harden/artifacts/matrix.jsonl.

Prints the tables REPORT.md needs: per document, per diagnostic code, exit-status disagreement,
and nondeterministic commands.  Nothing here judges whether a diagnostic is right; that is the
author's job and it lives in REPORT.md.
"""

import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
MATRIX = ROOT / "docs" / "artifacts" / "matrix.jsonl"


def main() -> int:
    records = [json.loads(line) for line in MATRIX.read_text(encoding="utf-8").splitlines() if line.strip()]
    by_doc = collections.defaultdict(dict)
    for record in records:
        by_doc[record["doc"]][record["command"]] = record

    print("== documents")
    print(f"{'document':38} {'exit':>4} {'strict':>6} {'warn':>4}  codes")
    for doc in sorted(by_doc):
        check = by_doc[doc].get("check-json", {})
        strict = by_doc[doc].get("check-strict", {})
        codes = check.get("codes", [])
        errors = sum(1 for code in codes if code.startswith("E"))
        warnings = len(codes) - errors
        print(f"{doc:38} {check.get('exit', '?'):>4} {strict.get('exit', '?'):>6} {warnings:>4}  {' '.join(codes)}")

    print()
    print("== diagnostic codes over the corpus")
    counts = collections.Counter()
    docs = collections.defaultdict(set)
    message = {}
    for doc, commands in by_doc.items():
        for code in commands.get("check-json", {}).get("codes", []):
            counts[code] += 1
            docs[code].add(doc)
        for code, text in commands.get("check-json", {}).get("messages", {}).items():
            message.setdefault(code, text)
    for code, count in sorted(counts.items(), key=lambda pair: (-pair[1], pair[0])):
        print(f"{code:34} {count:>4} occurrences in {len(docs[code]):>3} document(s)")
        print(f"    {message.get(code, '')}")

    print()
    print("== exit status of commands that name a document with errors")
    check_errors = {
        doc: sum(1 for code in commands.get("check-json", {}).get("codes", []) if code.startswith("E"))
        for doc, commands in by_doc.items()
    }
    labels = sorted({label for commands in by_doc.values() for label in commands})
    for label in labels:
        zero = sum(
            1
            for doc, commands in by_doc.items()
            if check_errors[doc] > 0 and label in commands and commands[label]["exit"] == 0
        )
        one = sum(
            1
            for doc, commands in by_doc.items()
            if check_errors[doc] > 0 and label in commands and commands[label]["exit"] == 1
        )
        if zero or one:
            print(f"{label:22} on error documents: exit 0 in {zero:>2}, exit 1 in {one:>2}")

    print()
    print("== nondeterministic commands (4 runs)")
    broken = [record for record in records if not record.get("identical", True)]
    if not broken:
        print("none")
    for record in broken:
        print(f"  {record['doc']} {record['command']} exits {record['exit']}/{record['exit_second']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
