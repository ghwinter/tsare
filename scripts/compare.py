"""Compare the baseline matrix with the post-fix matrix.

Prints: diagnostic counts before/after, documents whose diagnostics changed, exit-status
survey for commands on documents with errors, and nondeterministic commands in both runs.
"""

import collections
import json
import pathlib
import sys

ROOT = pathlib.Path(__file__).resolve().parent.parent
ARTIFACTS = ROOT / "docs" / "artifacts"


def load(name):
    path = ARTIFACTS / name
    if not path.exists():
        return []
    return [json.loads(line) for line in path.read_text(encoding="utf-8").splitlines() if line.strip()]


def by_doc(records):
    out = collections.defaultdict(dict)
    for record in records:
        out[record["doc"]][record["command"]] = record
    return out


def codes(records):
    counts = collections.Counter()
    docs = collections.defaultdict(set)
    for record in records:
        for code in record.get("codes", []):
            counts[code] += 1
            docs[code].add(record["doc"])
    return counts, docs


def main() -> int:
    before = load("matrix-baseline.jsonl")
    after = load("matrix-after.jsonl")
    if not before or not after:
        print("missing matrix-baseline.jsonl or matrix-after.jsonl", file=sys.stderr)
        return 1
    bcounts, bdocs = codes([r for r in before if r["command"] == "check-json"])
    acounts, adocs = codes([r for r in after if r["command"] == "check-json"])
    print("== diagnostic counts, baseline -> after")
    for code in sorted(set(bcounts) | set(acounts)):
        print(
            f"{code:34} {bcounts.get(code, 0):>3} -> {acounts.get(code, 0):>3}"
            f"   ({len(bdocs.get(code, ()))} -> {len(adocs.get(code, ()))} documents)"
        )

    print()
    print("== documents whose diagnostics changed")
    bmap = by_doc(before)
    amap = by_doc(after)
    for doc in sorted(set(bmap) | set(amap)):
        b = bmap.get(doc, {}).get("check-json", {})
        a = amap.get(doc, {}).get("check-json", {})
        bc = collections.Counter(b.get("codes", []))
        ac = collections.Counter(a.get("codes", []))
        if bc != ac:
            gone = bc - ac
            came = ac - bc
            print(f"{doc}")
            for code, count in sorted(gone.items()):
                print(f"   gone     {code} x{count}")
            for code, count in sorted(came.items()):
                print(f"   appeared {code} x{count}")

    print()
    print("== exit status on documents with errors (check-json errors > 0)")
    for label in ("check", "argue", "impact", "weigh", "cat", "induce", "graph", "project", "build"):
        line = []
        for name, mapping in (("before", bmap), ("after", amap)):
            zero = one = 0
            for doc, commands in mapping.items():
                errs = sum(1 for code in commands.get("check-json", {}).get("codes", []) if code.startswith("E"))
                if errs > 0 and label in commands:
                    if commands[label]["exit"] == 0:
                        zero += 1
                    elif commands[label]["exit"] == 1:
                        one += 1
            line.append(f"{name}: exit0={zero} exit1={one}")
        print(f"{label:10} {'   '.join(line)}")

    print()
    print("== nondeterministic commands (4 runs)")
    for name, records in (("before", before), ("after", after)):
        broken = [record for record in records if not record.get("identical", True)]
        print(f"{name}: {len(broken)}")
        for record in broken[:20]:
            print(f"   {record['doc']} {record['command']}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
