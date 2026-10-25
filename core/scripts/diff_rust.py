"""Differential: the Rust binary against the Python core, on counts so far.

Positions are not in the Rust output yet, so this compares how many E-UNDEFINED and
E-DUP-ID each side reports per document.  When the Rust side carries spans this becomes a
line-by-line comparison.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

PY = sys.executable
CORE = Path(__file__).resolve().parent.parent
BIN = Path(__file__).resolve().parent.parent.parent / "rust" / "engine-core" / "target" / "release" / "engine-core.exe"
V1 = Path(r"D:\GitHub\metaform\obligo")


def python_pairs(path: Path):
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(d["code"], d["line"]) for d in payload["diagnostics"] if d["code"] in ("E-UNDEFINED", "E-DUP-ID")}


def rust_pairs(path: Path):
    result = subprocess.run([str(BIN), str(path)], capture_output=True, text=True)
    pairs = set()
    for line in result.stdout.splitlines():
        parts = line.strip().split(" ", 2)
        if len(parts) < 2 or not parts[0].startswith("E-"):
            continue
        rest = parts[1]
        match = re.search(r":(\d+)$", rest)
        if match:
            pairs.add((parts[0], int(match.group(1))))
    return pairs


def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left, right = python_pairs(path), rust_pairs(path)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}: only-python={sorted(left - right)[:2]} only-rust={sorted(right - left)[:2]}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
