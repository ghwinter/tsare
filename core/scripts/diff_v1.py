"""Differential test: v1 and engine must agree on every document.

Compares (code, line, message) triples, because that is what a reader relies on.  Any
disagreement is printed and the exit code is non-zero.
"""

from __future__ import annotations

import json
import re
import subprocess
import sys
from pathlib import Path

PY = sys.executable
V1 = Path(r"D:\GitHub\metaform\obligo")
CORE = Path(__file__).resolve().parent.parent

LINE = re.compile(r"^(?P<file>.+?):(\d+):(\d+): (?P<sev>\w+)\[(?P<code>[A-Z-]+)\]: (?P<msg>.*)$")


def v1_codes(path: Path) -> set[tuple[str, int, str]]:
    result = subprocess.run(
        [PY, "cli.py", "check", str(path)], cwd=V1, capture_output=True, text=True
    )
    out = set()
    for line in result.stdout.splitlines():
        match = LINE.match(line)
        if match:
            out.add((match.group("code"), int(match.group(2)), match.group("msg")))
    return out


def core_codes(path: Path) -> set[tuple[str, int, str]]:
    result = subprocess.run(
        [PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True
    )
    payload = json.loads(result.stdout)
    return {(d["code"], d["line"], d["message"]) for d in payload["diagnostics"]}


def main() -> int:
    documents = sorted(V1.glob("*.mf"))
    mismatches = 0
    for path in documents:
        left, right = v1_codes(path), core_codes(path)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}")
            print(f"  only v1:   {sorted(left - right)[:3]}")
            print(f"  only core: {sorted(right - left)[:3]}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
