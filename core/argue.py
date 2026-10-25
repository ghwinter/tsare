"""Which conclusions survive their own rules?

Warrants can attack warrants.  The grounded extension gives the unique set of rules that
survive every attack they receive, computed by the standard fixpoint:

    IN_0 = {}
    OUT_i = { a : some b in IN_i attacks a }
    IN_{i+1} = { a : every b that attacks a is in OUT_i }

A proposition is supported when some rule for it is IN, defeated when every rule for it is
OUT, contested when the rules are undecided, and asserted when it has no rule at all.
"""

from __future__ import annotations

from check import Program, f_id, f_ids


def grounded(prog: Program) -> tuple[set[str], set[str], set[str]]:
    warrants = prog.kind("warrant")
    names = [w.name for w in warrants]
    # a rule with no premises licenses nothing: it is out from the start and cannot defeat
    invalid = {w.name for w in warrants if not f_ids(w, "from")}
    attackers: dict[str, set[str]] = {name: set() for name in names}
    for w in warrants:
        if w.name in invalid:
            continue
        for target in f_ids(w, "attacks"):
            if target in attackers:
                attackers[target].add(w.name)

    inside: set[str] = set()
    outside: set[str] = set(invalid)
    while True:
        new_inside = {
            name
            for name in names
            if name not in invalid and all(att in outside for att in attackers[name])
        }
        new_outside = set(invalid) | {
            name for name in names if any(att in new_inside for att in attackers[name])
        }
        if new_inside == inside and new_outside == outside:
            break
        inside, outside = new_inside, new_outside
    undecided = set(names) - inside - outside
    return inside, outside, undecided


def conclusions(prog: Program) -> list[tuple[str, str, str]]:
    inside, outside, undecided = grounded(prog)
    by_conclusion: dict[str, list[str]] = {}
    for w in prog.kind("warrant"):
        conclusion = f_id(w, "to")
        if conclusion:
            by_conclusion.setdefault(conclusion, []).append(w.name)

    rows: list[tuple[str, str, str]] = []
    for d in prog.kind("proposition"):
        rules = by_conclusion.get(d.name, [])
        if not rules:
            rows.append((d.name, "asserted", "no rule"))
        elif any(rule in inside for rule in rules):
            rows.append((d.name, "supported", ", ".join(r for r in rules if r in inside)))
        elif all(rule in outside for rule in rules):
            rows.append((d.name, "defeated", ", ".join(f"{r} (out)" for r in rules)))
        else:
            rows.append((d.name, "contested", ", ".join(f"{r} ({'in' if r in inside else 'undecided'})" for r in rules)))
    for name in sorted(undecided):
        rows.append((name, "undecided", "rule neither survives nor falls"))
    return rows
