"""Global analyses: the part of a compiler that is not a per-declaration rule.

A rule can only look at one declaration.  Intelligence is what you get when you look at
the whole net: what is dead, what everything hangs on, what would still stand if a node
were wrong.  These are the cheap analogues of liveness, dominators and dead-code
elimination -- on a graph of commitments instead of a control-flow graph.
"""

from __future__ import annotations

import collections

from check import Program, f_id, f_ids, neg, pos


def dependents(prog: Program) -> dict[str, list[tuple[str, str]]]:
    """node -> [(dependent, edge kind)]"""
    out: dict[str, list[tuple[str, str]]] = collections.defaultdict(list)
    for c in prog.kind("concept"):
        for f in pos(c):
            out[f].append((c.name, "positive"))
        for f in neg(c):
            out[f].append((c.name, "negative"))
    for w in prog.kind("warrant"):
        for p in f_ids(w, "from"):
            out[p].append((w.name, "premise"))
        basis = f_ids(w, "basis")
        for b in basis:
            out[b].append((w.name, "basis"))
        for t in f_ids(w, "attacks"):
            out[t].append((w.name, "attacked by"))
        conclusion = f_id(w, "to")
        if conclusion:
            out[w.name].append((conclusion, "concludes"))
    for h in prog.kind("hypothesis"):
        for p in f_ids(h, "assumes"):
            out[p].append((h.name, "assumes"))
    for p in prog.kind("proposition"):
        for f in f_ids(p, "from"):
            out[f].append((p.name, "from"))
    for q in prog.kind("question"):
        for h in f_ids(q, "separates"):
            out[h].append((q.name, "separates"))
    for b in prog.kind("blindspot"):
        for q in f_ids(b, "closes_with"):
            out[b.name].append((q, "closes with"))
    return out


def supporting_warrants(prog: Program) -> dict[str, list[str]]:
    out: dict[str, list[str]] = collections.defaultdict(list)
    for w in prog.kind("warrant"):
        conclusion = f_id(w, "to")
        if conclusion:
            out[conclusion].append(w.name)
    return out


def adjacency(prog: Program) -> dict[str, set[str]]:
    adj: dict[str, set[str]] = collections.defaultdict(set)
    for src, targets in dependents(prog).items():
        for dst, _ in targets:
            adj[src].add(dst)
            adj[dst].add(src)
    return adj


def unlocks(prog: Program, node: str, depth: int = 2) -> int:
    """How many claims sit within two hops of this decision."""
    adj = adjacency(prog)
    claims = {"proposition", "hypothesis", "question"}
    seen = {node}
    frontier = {node}
    for _ in range(depth):
        nxt: set[str] = set()
        for name in frontier:
            for other in adj.get(name, ()):
                if other not in seen:
                    seen.add(other)
                    nxt.add(other)
        frontier = nxt
    return sum(
        1 for name in seen if name in prog.by_name and prog.by_name[name].kind in claims
    )


def decisions(prog: Program) -> list[tuple[str, str, int]]:
    """Open decisions: holes, questions, blind spots. Ranked by what they touch."""
    from syntax import VHole

    rows: list[tuple[str, str, int]] = []
    for d in prog.decls:
        for key in d.order:
            if isinstance(d.fields[key], VHole):
                rows.append((f"{d.name}.{key} (hole)", d.name, unlocks(prog, d.name)))
    for q in prog.kind("question"):
        rows.append((f"{q.name} (question)", q.name, unlocks(prog, q.name)))
    for b in prog.kind("blindspot"):
        rows.append((f"{b.name} (blind spot)", b.name, unlocks(prog, b.name)))
    rows.sort(key=lambda row: (-row[2], row[0]))
    return rows


def report(prog: Program) -> list[str]:
    deps = dependents(prog)
    supports = supporting_warrants(prog)
    lines: list[str] = []

    dead_facts = [d.name for d in prog.kind("fact") if not deps.get(d.name)]
    if dead_facts:
        lines.append("dead facts (nothing depends on them): " + ", ".join(dead_facts))

    single = [(p, w[0]) for p, w in supports.items() if len(w) == 1]
    for proposition, warrant in single:
        lines.append(f"single support: {proposition} rests on {warrant} alone")

    orphan_props = [
        d.name
        for d in prog.kind("proposition")
        if not supports.get(d.name) and not any(h for h, _ in deps.get(d.name, []) if _ == "assumes")
    ]
    if orphan_props:
        lines.append("propositions with no rule and no dependent: " + ", ".join(orphan_props))

    # load-bearing: facts whose removal would leave a single-support proposition alone
    for proposition, warrant in single:
        warrant_decl = prog.by_name.get(warrant)
        if warrant_decl is None:
            continue
        premises = f_ids(warrant_decl, "from")
        for premise in premises:
            users = [name for name, kind in deps.get(premise, []) if kind == "premise"]
            if users == [warrant]:
                lines.append(
                    f"load-bearing: {premise} is the only premise of the only rule for {proposition}"
                )

    counts = collections.Counter(d.kind for d in prog.decls)
    lines.append(
        "graph: "
        + ", ".join(f"{kind}={n}" for kind, n in sorted(counts.items()))
    )
    return lines
