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
CODES = {"E-UNDEFINED", "E-DUP-ID", "E-KIND-UNKNOWN", "E-FIELD", "E-MISSING", "E-TYPE", "E-CYCLE", "E-COLLAPSE", "E-TENSION", "E-LAYER-LEAK", "E-VACUOUS", "W-OPEN", "W-UNWARRANTED", "W-CONFIDENCE-ASSERTED", "E-NO-ASSUMPTION", "E-UNFALSIFIABLE", "E-NO-EVIDENCE", "E-NO-PREMISE", "E-WARRANT-MODE", "W-UNWARRANTED-BASIS", "E-EMPTY", "W-OVERCLAIM", "W-UNCONFIRMED", "E-CONFIDENCE", "E-UNGROUNDED", "W-UNTESTED", "E-CONTRADICTION", "E-HIERARCHY", "E-NO-ANSWERS", "E-NON-DISCRIMINATING", "W-COARSE", "E-WITNESS", "E-NO-SIGNATURE", "W-NO-EXIT", "E-FALSE-BLINDSPOT", "W-LAW-UNCHECKED", "E-INCOMMENSURABLE", "W-EXPOSE-FOREIGN"}


def python_pairs(path: Path):
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(d["code"], d["line"]) for d in payload["diagnostics"] if d["code"] in CODES}


def rust_pairs(path: Path):
    result = subprocess.run([str(BIN), str(path)], capture_output=True, text=True)
    pairs = set()
    for line in result.stdout.splitlines():
        parts = line.strip().split(" ", 2)
        if len(parts) < 2 or len(parts[0]) < 4 or not all(ch.isupper() or ch == "-" for ch in parts[0]):
            continue
        rest = parts[1]
        match = re.search(r":(\d+)$", rest)
        if match and parts[0] in CODES:
            pairs.add((parts[0], int(match.group(1))))
    return pairs




def python_dir_pairs(path: Path):
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(d["code"], d["file"], d["line"]) for d in payload["diagnostics"] if d["code"] in CODES}


def rust_dir_pairs(path: Path):
    result = subprocess.run([str(BIN), str(path)], capture_output=True, text=True)
    pairs = set()
    for line in result.stdout.splitlines():
        parts = line.strip().split(" ", 2)
        if len(parts) < 2 or len(parts[0]) < 4 or not all(ch.isupper() or ch == "-" for ch in parts[0]):
            continue
        match = re.match(r"(.+):(\d+)$", parts[1])
        if match and parts[0] in CODES:
            pairs.add((parts[0], Path(match.group(1)).name, int(match.group(2))))
    return pairs

def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left, right = python_pairs(path), rust_pairs(path)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}: only-python={sorted(left - right)[:2]} only-rust={sorted(right - left)[:2]}")
    for project in sorted((V1 / "projects").glob("*")):
        left, right = python_dir_pairs(project), rust_dir_pairs(project)
        if left != right:
            mismatches += 1
            print(f"MISMATCH project {project.name}: only-python={sorted(left - right)[:3]} only-rust={sorted(right - left)[:3]}")
        else:
            print(f"project {project.name}: {len(left)} diagnostics agree")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
