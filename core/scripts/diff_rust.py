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


def python_counts(path: Path) -> dict[str, int]:
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    counts = {"E-UNDEFINED": 0, "E-DUP-ID": 0}
    for item in payload["diagnostics"]:
        if item["code"] in counts:
            counts[item["code"]] += 1
    return counts


def rust_counts(path: Path) -> dict[str, int]:
    result = subprocess.run([str(BIN), str(path)], capture_output=True, text=True)
    undefined = re.search(r"undefined: (\d+)", result.stdout)
    duplicates = re.search(r"duplicates: (\d+)", result.stdout)
    return {
        "E-UNDEFINED": int(undefined.group(1)) if undefined else -1,
        "E-DUP-ID": int(duplicates.group(1)) if duplicates else -1,
    }


def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left, right = python_counts(path), rust_counts(path)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}: python={left} rust={right}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
