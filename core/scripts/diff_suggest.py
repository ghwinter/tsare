"""Compare suggestions between the Python core and the binary.

Python omits `span` for repairs derived from diagnostics, so the comparison is on
(applicability, message).  A suggestion that exists on one side only is a gap.
"""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

PY = sys.executable
CORE = Path(__file__).resolve().parent.parent
BIN = Path(__file__).resolve().parent.parent.parent / "rust" / "engine-core" / "target" / "release" / "engine-core.exe"
V1 = Path(r"D:\GitHub\metaform\obligo")


def python_suggestions(path: Path) -> set:
    result = subprocess.run([PY, "cli.py", "suggest", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(item["applicability"], item["message"]) for item in payload["suggestions"]}


def rust_suggestions(path: Path) -> set:
    result = subprocess.run([str(BIN), "suggest", str(path), "--json"], capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(item["applicability"], item["message"]) for item in payload["suggestions"]}


def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left, right = python_suggestions(path), rust_suggestions(path)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}: missing={sorted(left - right)[:2]} extra={sorted(right - left)[:2]}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
