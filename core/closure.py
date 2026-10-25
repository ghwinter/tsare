"""The one closure both `derive` and `engine` need.

A rule applies when it survives attack and every premise is already held; the least set
closed under that rule is the same set for both callers.  Keeping the iteration here means
there is one definition of "grounded", not two that can drift.
"""

from __future__ import annotations

from argue import grounded
from check import Program, f_id, f_ids
from syntax import VId, VList, VNum, VRec, VStr


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
    """(warrant, premises, conclusion) for named and class-headed rules."""
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


def fixpoint(prog: Program) -> dict:
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
    return {
        "facts": facts,
        "held": held,
        "parent": parent,
        "rules": rules,
        "inside": inside,
        "outside": outside,
        "undecided": undecided,
    }
