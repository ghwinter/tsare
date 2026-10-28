"""Text differential against the frozen reference implementation.

The Python in `obligo` (tag `v1`) was the live implementation until the port finished; it is
kept as the oracle for this script.  The engine's own tests do not need it: `cargo test` holds
the same answers as golden snapshots.

Each case names how its arguments are built:

    file    one document
    name    one document and a removable declaration (whatif)
    concept one document and a concept (pattern)
    pair    two documents (map)
    graph   one document; the written file is compared too

The script is a record, not a gate: the port is finished, so a mismatch now means the
reference and the engine have deliberately diverged.
"""

from __future__ import annotations

import re
import subprocess
import sys
import tempfile
from pathlib import Path

PY = sys.executable
V1 = Path(r"D:\GitHub\metaform\obligo")
BIN = Path(__file__).resolve().parent.parent / "target" / "release" / "tsare.exe"

CASES: list[tuple[str, list[str]]] = [
    ("file", ["argue"]),
    ("file", ["derive"]),
    ("file", ["debt"]),
    ("file", ["debt", "--all"]),
    ("file", ["impact"]),
    ("file", ["next"]),
    ("file", ["cost"]),
    ("file", ["weigh"]),
    ("file", ["cat"]),
    ("file", ["moves"]),
    ("file", ["assumptions"]),
    ("file", ["audit"]),
    ("file", ["egraph"]),
    ("file", ["fix"]),
    ("file", ["induce"]),
    ("file", ["suggest"]),
    ("name", ["whatif"]),
    ("concept", ["pattern"]),
    ("graph", ["graph"]),
]

PAIRS = [
    ("analogy-a.mf", "analogy-b.mf"),
    ("app.v1.mf", "app.v2.mf"),
    ("talk.v1.mf", "talk.v2.mf"),
    ("vision.v1.mf", "vision.v2.mf"),
]

NAME_RE = re.compile(r"^(fact|warrant) (\w+)", re.MULTILINE)
CONCEPT_RE = re.compile(r"^concept (\w+)", re.MULTILINE)


def run_python(argv: list[str]) -> tuple[int, str]:
    finished = subprocess.run(
        [PY, "cli.py", *argv], cwd=V1, capture_output=True, text=True, encoding="utf-8"
    )
    return finished.returncode, finished.stdout


def run_rust(argv: list[str]) -> tuple[int, str]:
    finished = subprocess.run(
        [str(BIN), *argv], capture_output=True, text=True, encoding="utf-8"
    )
    return finished.returncode, finished.stdout


def first(pattern: re.Pattern[str], path: Path) -> str | None:
    match = pattern.search(path.read_text(encoding="utf-8"))
    return match.group(2) if pattern is NAME_RE and match else (match.group(1) if match else None)


def main() -> int:
    missing: set[str] = set()
    crashed: list[str] = []
    mismatches = 0
    compared = 0
    documents = sorted(V1.glob("*.mf"))

    for kind, command in CASES:
        for path in documents:
            if kind == "name":
                name = first(NAME_RE, path)
                if name is None:
                    continue
                argv = [*command, str(path), name]
            elif kind == "concept":
                concept = first(CONCEPT_RE, path)
                if concept is None:
                    continue
                argv = [*command, str(path), concept]
            else:
                argv = [*command, str(path)]
            if kind == "graph":
                # the output path appears in stdout, so both sides write to the same place
                out = Path(tempfile.gettempdir()) / "engine-diff.net.mmd"
                argv = [*command, str(path), "--out", str(out)]
                left_code, left = run_python(argv)
                left_text = out.read_text(encoding="utf-8") if out.exists() else ""
                out.unlink(missing_ok=True)
                right_code, right = run_rust(argv)
                right_text = out.read_text(encoding="utf-8") if out.exists() else ""
                out.unlink(missing_ok=True)
                if left_text != right_text:
                    mismatches += 1
                    print(f"MISMATCH {' '.join(command)} {path.name} (written file)")
            else:
                left_code, left = run_python(argv)
                right_code, right = run_rust(argv)
            compared += 1
            if right_code == 2 and "unknown command" in right:
                missing.add(" ".join(command))
                compared -= 1
                break
            if left_code != 0 and left == "":
                # the reference crashed on this document; a port cannot match a traceback,
                # and the engine is expected not to crash at all
                compared -= 1
                crashed.append(f"{' '.join(command)} {path.name}")
                continue
            if left != right:
                mismatches += 1
                if mismatches <= 80:
                    print(f"MISMATCH {' '.join(command)} {path.name}")
                    left_lines, right_lines = left.splitlines(), right.splitlines()
                    for index in range(max(len(left_lines), len(right_lines))):
                        a = left_lines[index] if index < len(left_lines) else "<none>"
                        b = right_lines[index] if index < len(right_lines) else "<none>"
                        if a != b:
                            print(f"  line {index + 1}\n    py: {a}\n    rs: {b}")
                            break

    for left_name, right_name in PAIRS:
        argv = ["map", str(V1 / left_name), str(V1 / right_name)]
        left_code, left = run_python(argv)
        right_code, right = run_rust(argv)
        compared += 1
        if left != right:
            mismatches += 1
            print(f"MISMATCH map {left_name} {right_name}")
            print(f"    py: {left.splitlines()[:2]}\n    rs: {right.splitlines()[:2]}")

    # build: the emitted files are the artifact, so compare them, not only stdout.
    # stdout is not compared for build: it replays the checker's diagnostics, and the engine
    # words and renders its own (the one deliberate divergence: PROTOCOL.md).
    EMITTED = ["OBLIGATIONS.md", "LEARNED.md", "OPEN.md", "NOTES.md", "predicates.py"]
    for path in documents:
        with tempfile.TemporaryDirectory() as left_dir, tempfile.TemporaryDirectory() as right_dir:
            run_python(["build", str(path), "--out", left_dir])
            run_rust(["build", str(path), "--out", right_dir])
            for name in EMITTED:
                left_file, right_file = Path(left_dir) / name, Path(right_dir) / name
                left_text = left_file.read_text(encoding="utf-8") if left_file.exists() else "<missing>"
                right_text = right_file.read_text(encoding="utf-8") if right_file.exists() else "<missing>"
                compared += 1
                if left_text != right_text:
                    mismatches += 1
                    print(f"MISMATCH build {path.name} ({name})")
                    left_lines, right_lines = left_text.splitlines(), right_text.splitlines()
                    for index in range(max(len(left_lines), len(right_lines))):
                        a = left_lines[index] if index < len(left_lines) else "<none>"
                        b = right_lines[index] if index < len(right_lines) else "<none>"
                        if a != b:
                            print(f"  line {index + 1}\n    py: {a}\n    rs: {b}")
                            break

    print(f"comparisons: {compared}, mismatches: {mismatches}")
    if crashed:
        print("reference crashed: " + ", ".join(crashed))
    if missing:
        print("not implemented: " + ", ".join(sorted(missing)))
    return 0 if mismatches == 0 and not missing else 1


if __name__ == "__main__":
    raise SystemExit(main())



