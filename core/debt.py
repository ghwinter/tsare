"""Transitive debt: which facts gate a conclusion, and how they were obtained.

`cost` counts what a declaration owes directly.  This walks the support graph instead, so a
claim is charged for the evidence at the end of its chain even when a proposition sits in
between.  The output is the first sentence the document does not contain:

    P_SHOULD is gated by F_COMP, whose evidence is user, two steps away.
"""

from __future__ import annotations

import collections

from check import Program, f_id, f_ids, f_str

MEASURED = {"tool", "doc", "code"}


def evidence_status(decl) -> str:
    evidence = decl.fields.get("evidence")
    name = getattr(evidence, "name", None)
    if name in MEASURED:
        return "measured"
    if name == "model":
        return "confirmed" if f_str(decl, "confirmed_by") else "unconfirmed"
    return "asserted"


def support_edges(prog: Program) -> dict[str, list[str]]:
    edges: dict[str, list[str]] = collections.defaultdict(list)
    for warrant in prog.kind("warrant"):
        conclusion = f_id(warrant, "to")
        if conclusion:
            edges[conclusion].extend(f_ids(warrant, "from"))
    for proposition in prog.kind("proposition"):
        edges[proposition.name].extend(f_ids(proposition, "from"))
    for hypothesis in prog.kind("hypothesis"):
        edges[hypothesis.name].extend(f_ids(hypothesis, "assumes"))
    return edges


def walk(prog: Program, start: str, edges: dict[str, list[str]]):
    """Breadth-first over premises; returns {fact name: (status, path)}."""
    found: dict[str, tuple[str, list[str]]] = {}
    seen = {start}
    queue = collections.deque([[start]])
    while queue:
        path = queue.popleft()
        for premise in edges.get(path[-1], ()):
            if premise in seen:
                continue
            seen.add(premise)
            decl = prog.by_name.get(premise)
            if decl is None:
                continue
            step = path + [premise]
            if decl.kind == "fact":
                found[premise] = (evidence_status(decl), step)
            else:
                queue.append(step)
    return found


def report(prog: Program, only_gated: bool = True) -> list[tuple[str, str, int, int, list[str]]]:
    edges = support_edges(prog)
    rows = []
    for decl in prog.decls:
        if decl.kind not in {"proposition", "hypothesis"}:
            continue
        facts = walk(prog, decl.name, edges)
        weak = [(name, status, path) for name, (status, path) in facts.items() if status != "measured"]
        if only_gated and not weak:
            continue
        if any(status == "unconfirmed" for _, status, _ in weak):
            worst = "unconfirmed"
        elif weak:
            worst = "asserted"
        else:
            worst = "clear"
        rows.append(
            (
                decl.name,
                worst,
                len(weak),
                len(facts),
                [f"{name} ({status}) via {' <- '.join(path)}" for name, status, path in weak],
            )
        )
    rows.sort(key=lambda row: (-row[2], row[0]))
    return rows
