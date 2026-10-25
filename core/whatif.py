"""Counterfactual: remove one assumption and see which conclusions stop standing.

Only facts and warrants can be removed; they are referenced from lists, so the edit is
`strip every list entry that names it`.  A scalar anchor (a warrant's `to`, a hypothesis's
`rival`) is reported instead of silently dangling.

This is the mechanical form of "which axiom is load-bearing" (research/31, moves R1/R2).
"""

from __future__ import annotations

from argue import conclusions
from check import Program, check
from syntax import Decl, VId, VList

REMOVABLE = {"fact", "warrant"}


def strip(value, name: str):
    if isinstance(value, VList):
        items = [
            item for item in value.items if not (isinstance(item, VId) and item.name == name)
        ]
        if len(items) != len(value.items):
            return VList(value.span, items)
    return value


def remove(decls: list[Decl], name: str) -> tuple[list[Decl] | None, str | None]:
    target = next((decl for decl in decls if decl.name == name), None)
    if target is None:
        return None, f"{name} is not declared"
    if target.kind not in REMOVABLE:
        return None, f"refusing to remove a {target.kind}: only facts and warrants are assumptions"

    import copy

    out: list[Decl] = []
    for decl in decls:
        if decl.name == name:
            continue
        clone = copy.copy(decl)
        clone.fields = dict(decl.fields)
        clone.order = list(decl.order)
        for key in clone.order:
            clone.fields[key] = strip(clone.fields[key], name)
        out.append(clone)
    return out, None


def statuses(prog: Program) -> dict[str, str]:
    return {
        name: status
        for name, status, _ in conclusions(prog)
        if name in prog.by_name and prog.by_name[name].kind == "proposition"
    }


def compare(before: list[Decl], after: list[Decl], base) -> dict:
    prog_before, diags_before = check(before, base)
    prog_after, diags_after = check(after, base)
    codes_before = {d.code for d in diags_before}
    codes_after = {d.code for d in diags_after}
    sb, sa = statuses(prog_before), statuses(prog_after)
    changed = {name: (sb[name], sa.get(name, "gone")) for name in sb if sb[name] != sa.get(name)}
    return {
        "errors_before": sum(1 for d in diags_before if d.severity == "error"),
        "errors_after": sum(1 for d in diags_after if d.severity == "error"),
        "warnings_before": sum(1 for d in diags_before if d.severity == "warning"),
        "warnings_after": sum(1 for d in diags_after if d.severity == "warning"),
        "new_codes": sorted(codes_after - codes_before),
        "lost_codes": sorted(codes_before - codes_after),
        "changed": changed,
        "newly_asserted": [
            name for name, status in sa.items() if status == "asserted" and sb.get(name) != "asserted"
        ],
    }
