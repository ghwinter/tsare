"""Decompression cost: how much external work a document demands before it can be checked.

Compression is nearly free; the bottleneck is expanding a claim back into something another
party can verify.  So the unit here is not characters saved, it is **external confirmations**
(a person, a tool, an instrument) plus **verification depth** (how many links a claim sits
from its evidence).
"""

from __future__ import annotations

from check import Program, f_id, f_ids, f_str, instrument_of
from syntax import VHole


def reasons(decl, prog: Program) -> list[str]:
    """The single definition lives in kinds_meta; every analysis reads it."""
    from kinds_meta import obligations

    return obligations(decl, prog)


def chain(prog: Program) -> tuple[int, list[str]]:
    """Longest support chain, and the path that realises it."""
    from impact import dependents

    adjacency: dict[str, list[str]] = {}
    support = {"premise", "basis", "concludes", "assumes", "separates", "from", "closes with"}
    for source, targets in dependents(prog).items():
        adjacency[source] = [
            dst for dst, kind in targets if dst in prog.by_name and kind in support
        ]

    best: tuple[int, list[str]] = (0, [])
    for decl in prog.decls:
        stack = [(decl.name, [decl.name], frozenset({decl.name}))]
        while stack:
            node, path, seen = stack.pop()
            if len(path) > best[0]:
                best = (len(path) - 1, path)
            for nxt in adjacency.get(node, ()):
                if nxt in seen:
                    continue
                stack.append((nxt, path + [nxt], seen | {nxt}))
    return best


def report(prog: Program) -> tuple[int, dict[str, int], list[tuple[str, list[str]]], tuple[int, list[str]]]:
    rows: list[tuple[str, list[str]]] = []
    by_kind: dict[str, int] = {}
    for decl in prog.decls:
        found = reasons(decl, prog)
        if found:
            rows.append((decl.name, found))
            by_kind[decl.kind] = by_kind.get(decl.kind, 0) + len(found)
    rows.sort(key=lambda row: -len(row[1]))
    return sum(by_kind.values()), by_kind, rows, chain(prog)
