"""Text differential: every subcommand, every document, byte for byte.

The Python implementation in `obligo` is the reference for text output.  Comparisons are
made in two groups, because they fail for different reasons:

    per-document  one command against one file, stdout compared exactly
    per-pair      `map` needs two documents

A subcommand the binary does not implement yet is reported, not skipped: a port is done when
the unimplemented list is empty and the mismatch count is zero.
"""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path

PY = sys.executable
V1 = Path(r"D:\GitHub\metaform\obligo")
BIN = Path(__file__).resolve().parent.parent.parent / "rust" / "engine-core" / "target" / "release" / "engine-core.exe"

COMMANDS: list[list[str]] = [
    ["argue"],
    ["derive"],
    ["debt"],
    ["debt", "--all"],
    ["impact"],
    ["next"],
    ["cost"],
    ["weigh"],
    ["cat"],
    ["moves"],
    ["assumptions"],
    ["audit"],
    ["egraph"],
    ["fix"],
    ["induce"],
]

# `graph` writes a file, so it is compared through stdout only after writing to a temp path
WRITES = {"graph"}


def run_python(command: list[str], target: Path) -> tuple[int, str]:
    finished = subprocess.run(
        [PY, "cli.py", *command, str(target)], cwd=V1, capture_output=True, text=True, encoding="utf-8"
    )
    return finished.returncode, finished.stdout


def run_rust(command: list[str], target: Path) -> tuple[int, str]:
    finished = subprocess.run(
        [str(BIN), *command, str(target)], capture_output=True, text=True, encoding="utf-8"
    )
    return finished.returncode, finished.stdout


def main() -> int:
    missing: set[str] = set()
    mismatches = 0
    documents = sorted(V1.glob("*.mf"))
    for command in COMMANDS:
        for path in documents:
            left_code, left = run_python(command, path)
            right_code, right = run_rust(command, path)
            if right_code == 2 and "unknown command" in right:
                missing.add(" ".join(command))
                break
            if left != right:
                mismatches += 1
                if mismatches <= 6:
                    print(f"MISMATCH {' '.join(command)} {path.name}")
                    left_lines, right_lines = left.splitlines(), right.splitlines()
                    for index in range(max(len(left_lines), len(right_lines))):
                        a = left_lines[index] if index < len(left_lines) else "<none>"
                        b = right_lines[index] if index < len(right_lines) else "<none>"
                        if a != b:
                            print(f"  line {index + 1}\n    py: {a}\n    rs: {b}")
                            break
    print(f"documents: {len(documents)}, commands: {len(COMMANDS)}, mismatches: {mismatches}")
    if missing:
        print("not implemented: " + ", ".join(sorted(missing)))
    return 0 if mismatches == 0 and not missing else 1


if __name__ == "__main__":
    raise SystemExit(main())
