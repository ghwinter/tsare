"""Full JSON differential: every diagnostic the Python core emits, compared with the binary.

The curated CODES list hid codes nobody remembered to add.  This compares the whole
diagnostic set instead: (code, severity, line) for files and (code, severity, file, line)
for directories.
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


def python_set(path: Path) -> set:
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    if path.is_dir():
        return {(d["code"], d["severity"], d["file"], d["line"]) for d in payload["diagnostics"]}
    return {(d["code"], d["severity"], d["line"]) for d in payload["diagnostics"]}


def rust_set(path: Path) -> set:
    result = subprocess.run([str(BIN), str(path), "--json"], capture_output=True, text=True)
    payload = json.loads(result.stdout)
    if path.is_dir():
        return {(d["code"], d["severity"], d["file"], d["line"]) for d in payload["diagnostics"]}
    return {(d["code"], d["severity"], d["line"]) for d in payload["diagnostics"]}


def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left, right = python_set(path), rust_set(path)
        if left != right:
            mismatches += 1
            missing = sorted(left - right)
            extra = sorted(right - left)
            print(f"MISMATCH {path.name}: missing={missing[:3]} extra={extra[:3]}")
    for project in sorted((V1 / "projects").glob("*")):
        left, right = python_set(project), rust_set(project)
        if left != right:
            mismatches += 1
            print(f"MISMATCH project {project.name}: missing={sorted(left - right)[:3]} extra={sorted(right - left)[:3]}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
