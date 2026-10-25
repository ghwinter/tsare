"""Engine: one answer from three organs.

`argue` decides which rules survive attack, `derive` decides which conclusions reach a fact,
`debt` names the facts that still gate them.  Each answered its own question; this module
runs one fixpoint over all three:

    held        a fact, or a conclusion of a live rule whose premises are all held
    conditional supported by a live rule, but a premise is not held
    chain       the rules and premises that put a conclusion in `held`

Attack propagation falls out of the fixpoint: if a rule's premise is defeated, the premise
is not held, so the rule does not fire, even though the rule itself survived attack.
"""

from __future__ import annotations

from argue import conclusions, grounded
from check import Program
from debt import support_edges, walk
from derive import applications


def run(prog: Program) -> dict:
    inside, outside, undecided = grounded(prog)
    facts = {d.name for d in prog.kind("fact")}
    held = set(facts)
    parent: dict[str, tuple[str, list[str]]] = {}
    rules = [rule for rule in applications(prog) if rule[0] in inside]
    changed = True
    while changed:
        changed = False
        for warrant, premises, target in rules:
            if target in held:
                continue
            if premises and all(premise in held for premise in premises):
                held.add(target)
                parent[target] = (warrant, premises)
                changed = True

    def chain(name: str, depth: int = 0) -> str:
        if name in facts or depth > 12:
            return name
        warrant, premises = parent.get(name, (None, []))
        if not premises:
            return name
        return f"{name} <- {warrant} <- " + " & ".join(chain(p, depth + 1) for p in premises)

    statuses = {name: state for name, state, _ in conclusions(prog)}
    supported = {name for name, state in statuses.items() if state == "supported"}
    conditional = sorted(supported - held)
    reasons: dict[str, list[str]] = {}
    for name in conditional:
        missing = []
        for warrant, premises, target in rules:
            if target != name:
                continue
            for premise in premises:
                if premise not in held:
                    state = "defeated" if statuses.get(premise) == "defeated" else statuses.get(premise, "fact")
                    missing.append(f"{warrant} needs {premise} ({state})")
        reasons[name] = missing or ["no live rule reached a fact"]

    dead_rules = [
        (warrant, [p for p in premises if p not in held])
        for warrant, premises, target in rules
        if target not in held
    ]

    gates = {}
    edges = support_edges(prog)
    for name in sorted(held & set(statuses)):
        facts_below = walk(prog, name, edges)
        weak = [f"{fact} ({status})" for fact, (status, _) in facts_below.items() if status != "measured"]
        gates[name] = weak

    return {
        "facts": sorted(facts),
        "held": sorted(held - facts),
        "conditional": conditional,
        "reasons": reasons,
        "chains": {name: chain(name) for name in sorted(held - facts)},
        "dead_rules": dead_rules,
        "gates": gates,
        "outside": sorted(outside),
        "undecided": sorted(undecided),
    }
