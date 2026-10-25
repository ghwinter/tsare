"""Apply a suggestion: splice its replacement over its span, then recompile.

A suggestion that does not improve the document is discarded — the same rule rustfix
follows, and the same rule the loop learned the hard way (note 55): never trust a repair
that was not re-verified.
"""

from __future__ import annotations

from pathlib import Path

from check import check
from syntax import load


def offset(text: str, line: int, col: int) -> int:
    lines = text.splitlines(keepends=True)
    if line < 1 or line > len(lines):
        return len(text)
    return sum(len(item) for item in lines[: line - 1]) + (col - 1)


def splice(text: str, span, replacement: str) -> str:
    start = offset(text, span.line, span.col)
    end = offset(text, span.end_line, span.end_col)
    if end < start:
        start, end = end, start
    # a removed declaration leaves its line break behind; drop the blank line it creates
    tail = text[end:]
    if replacement == "" and tail.startswith("\n"):
        tail = tail[1:]
    return text[:start] + replacement + tail


def score(text: str, base: Path):
    path = base / "__applied__.mf"
    path.write_text(text, encoding="utf-8")
    decls, parse_errors, _ = load(path)
    prog, diags = check(decls, base)
    path.unlink()
    errors = sum(1 for d in diags if d.severity == "error")
    warnings = sum(1 for d in diags if d.severity == "warning")
    return errors, warnings


def apply_best(text: str, base: Path, suggestions) -> tuple[str, list[str]]:
    """Apply suggestions in order while each one improves the score."""
    applied: list[str] = []
    current = text
    current_score = score(current, base)
    for item in suggestions:
        if item.span is None or item.replacement is None:
            continue
        candidate = splice(current, item.span, item.replacement)
        candidate_score = score(candidate, base)
        if candidate_score < current_score:
            current = candidate
            current_score = candidate_score
            applied.append(item.message)
    return current, applied
