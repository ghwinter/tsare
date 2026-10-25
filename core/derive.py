"""Grounding closure: a rule only derives its conclusion when every premise is derived.

`argue` answers "which rules survive attack".  It does not ask whether a premise is itself
grounded, so a proposition can be called supported while leaning on another proposition
that nothing supports.  This module computes the least set built from facts and rules whose
premises are already in that set, then compares the two answers.

A warrant may conclude a named proposition (`to`) or a class of propositions (`concludes`),
in which case it applies to every proposition whose fields match the pattern.
"""

from __future__ import annotations

from argue import conclusions
from check import Program, f_id, f_ids
from closure import applications, fixpoint, holds, matches


def holds(value, item) -> bool:
    if isinstance(item, VList):
        return all(holds(value, entry) for entry in item.items)
    if isinstance(value, VList):
        return any(holds(entry, item) for entry in value.items)
    if isinstance(value, VId) and isinstance(item, VId):
        return value.name == item.name
    if isinstance(value, VStr) and isinstance(item, VStr):
        return value.text == item.text
    if isinstance(value, VNum) and isinstance(item, VNum):
        return value.value == item.value
    return False


def matches(proposition, record: VRec) -> bool:
    for key, pattern in record.fields.items():
        if key not in proposition.fields:
            return False
        if not holds(proposition.fields[key], pattern):
            return False
    return True


def applications(prog: Program) -> list[tuple[str, list[str], str]]:
    """(warrant name, premises, conclusion) for named and class-headed rules."""
    out: list[tuple[str, list[str], str]] = []
    for warrant in prog.kind("warrant"):
        premises = f_ids(warrant, "from")
        target = f_id(warrant, "to")
        if target:
            out.append((warrant.name, premises, target))
            continue
        record = warrant.fields.get("concludes")
        if not isinstance(record, VRec):
            continue
        for proposition in prog.kind("proposition"):
            if matches(proposition, record):
                out.append((warrant.name, premises, proposition.name))
    return out


def analyze(prog: Program) -> dict:
    result = fixpoint(prog)
    facts = result["facts"]
    held = result["held"]
    rules = result["rules"]
    statuses = {name: state for name, state, _ in conclusions(prog)}
    supported = {name for name, state in statuses.items() if state == "supported"}
    derived = sorted(held - facts)
    conditional = sorted(supported - held)
    reasons: dict[str, list[str]] = {}
    for name in conditional:
        missing = []
        for warrant, premises, target in rules:
            if target != name:
                continue
            for premise in premises:
                if premise not in held:
                    missing.append(f"{warrant} needs {premise} ({statuses.get(premise, 'fact')})")
        reasons[name] = missing or ["no live rule reached a fact"]
    return {
        "facts": sorted(facts),
        "grounded": derived,
        "derived": derived,
        "conditional": conditional,
        "reasons": reasons,
        "rules_used": result["parent"],
    }
