"""Engine differential: the JSON from the Python core and the binary must be equal."""

from __future__ import annotations

import json
import subprocess
import sys
from pathlib import Path

PY = sys.executable
CORE = Path(__file__).resolve().parent.parent
BIN = Path(__file__).resolve().parent.parent.parent / "rust" / "engine-core" / "target" / "release" / "engine-core.exe"
V1 = Path(r"D:\GitHub\metaform\obligo")


def main() -> int:
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for path in documents:
        left = json.loads(subprocess.run([PY, "cli.py", "engine", str(path)], cwd=CORE, capture_output=True, text=True).stdout)
        right = json.loads(subprocess.run([str(BIN), "engine", str(path)], capture_output=True, text=True).stdout)
        if left != right:
            mismatches += 1
            print(f"MISMATCH {path.name}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
