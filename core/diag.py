"""Diagnostics with source spans and the standards every code must satisfy.

A diagnostic is not a log line: it names the declaration, the rule that fired, and
the smallest edit that would discharge it.  Codes are stable and quoted in tests.
"""

from __future__ import annotations

from dataclasses import dataclass

from syntax import Span


@dataclass
class Diagnostic:
    code: str
    message: str
    span: Span
    hint: str | None = None
    file: str | None = None

    @property
    def severity(self) -> str:
        return "error" if self.code.startswith("E-") else "warning"


def error(code: str, message: str, span: Span, hint: str | None = None) -> Diagnostic:
    assert code.startswith("E-"), code
    return Diagnostic(code, message, span, hint)


def warn(code: str, message: str, span: Span, hint: str | None = None) -> Diagnostic:
    assert code.startswith("W-"), code
    return Diagnostic(code, message, span, hint)


def sort_key(d: Diagnostic):
    return (d.span.line, d.span.col, d.code, d.message)


def render(path: str, src: str, diags: list[Diagnostic]) -> str:
    lines = src.splitlines()
    out: list[str] = []
    for d in sorted(diags, key=sort_key):
        where = f"{d.file or path}:{d.span.line}:{d.span.col}"
        out.append(f"{where}: {d.severity}[{d.code}]: {d.message}")
        gutter = " " * len(str(d.span.line))
        if 1 <= d.span.line <= len(lines):
            text = lines[d.span.line - 1]
            out.append(f" {d.span.line} | {text}")
            # a sugar line expands to a longer line, so a column can point past the end;
            # line numbers stay exact, columns are clamped instead of lying
            start = min(max(1, d.span.col), len(text) + 1)
            width = max(1, min(len(text) + 1, d.span.end_col) - start) if d.span.end_line == d.span.line else 1
            out.append(f" {gutter} | {' ' * (start - 1)}{'^' * width}")
        if d.hint:
            out.append(f" {gutter} = hint: {d.hint}")
        out.append("")
    return "\n".join(out).rstrip() + "\n"
