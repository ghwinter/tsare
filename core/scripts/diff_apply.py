"""Apply differential: both implementations repair the same document, then we compare the
diagnostics of the result rather than the bytes, because the Rust renderer formats the text
it writes differently while meaning the same document.
"""

from __future__ import annotations

import json
import shutil
import subprocess
import sys
import tempfile
from pathlib import Path

PY = sys.executable
CORE = Path(__file__).resolve().parent.parent
BIN = Path(__file__).resolve().parent.parent.parent / "rust" / "engine-core" / "target" / "release" / "engine-core.exe"
V1 = Path(r"D:\GitHub\metaform\obligo")


def codes(path: Path) -> set:
    result = subprocess.run([PY, "cli.py", "check", str(path)], cwd=CORE, capture_output=True, text=True)
    payload = json.loads(result.stdout)
    return {(item["code"], item["line"]) for item in payload["diagnostics"]}


def main() -> int:
    documents = sorted(V1.glob("*.mf"))
    mismatches = 0
    with tempfile.TemporaryDirectory() as folder:
        for path in documents:
            py_copy = Path(folder) / ("py_" + path.name)
            rs_copy = Path(folder) / ("rs_" + path.name)
            shutil.copy(path, py_copy)
            shutil.copy(path, rs_copy)
            subprocess.run([PY, "cli.py", "apply", str(py_copy)], cwd=CORE, capture_output=True, text=True)
            subprocess.run([str(BIN), "apply", str(rs_copy)], capture_output=True, text=True)
            left, right = codes(py_copy), codes(rs_copy)
            if left != right:
                mismatches += 1
                print(f"MISMATCH {path.name}: python-only={sorted(left - right)[:2]} rust-only={sorted(right - left)[:2]}")
    print(f"documents: {len(documents)}, mismatches: {mismatches}")
    return 0 if mismatches == 0 else 1


if __name__ == "__main__":
    raise SystemExit(main())
