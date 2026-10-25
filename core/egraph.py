"""Equivalence merging in the style of an e-graph: find nodes that mean the same, union
them, saturate, then rewrite every reference to the canonical name.

The checker rejects duplicates (`E-COLLAPSE`, `E-DUP-ID`).  A generator, especially an LLM,
produces them constantly, and rejecting is the least useful response.  Merging keeps the
information and removes the redundancy: the document gets shorter, and nothing else changes.
"""

from __future__ import annotations

import copy
from pathlib import Path

import fix
from check import Program, f_id, f_ids, f_num, f_str, neg, pos
from syntax import Decl, VId, VList, VRec

KINDS = {"fact", "concept", "proposition", "warrant", "value", "hypothesis", "question", "blindspot", "law"}


class UnionFind:
    def __init__(self, names: list[str]) -> None:
        self.parent = {name: name for name in names}

    def find(self, name: str) -> str:
        while self.parent[name] != name:
            self.parent[name] = self.parent[self.parent[name]]
            name = self.parent[name]
        return name

    def union(self, a: str, b: str) -> bool:
        ra, rb = self.find(a), self.find(b)
        if ra == rb:
            return False
        # canonical = lexicographically smaller, so the result is deterministic
        keep, drop = sorted((ra, rb))
        self.parent[drop] = keep
        return True


def key_of(decl: Decl, uf: UnionFind) -> tuple | None:
    """Keys are computed under the current union-find, which is what makes saturation work."""
    canon = lambda names: tuple(sorted(uf.find(name) for name in names))  # noqa: E731
    kind = decl.kind
    if kind == "fact":
        attrs = decl.fields.get("attrs")
        attr_key = (
            tuple((key, fix.render_value(attrs.fields[key])) for key in attrs.order)
            if isinstance(attrs, VRec)
            else ()
        )
        # spans differ between occurrences, so render instead of str()
        evidence = fix.render_value(decl.fields["evidence"]) if "evidence" in decl.fields else ""
        return ("fact", f_str(decl, "claim"), evidence, attr_key, f_str(decl, "scope"))
    if kind == "concept":
        return ("concept", canon(pos(decl)), canon(neg(decl)))
    if kind == "proposition":
        return ("proposition", f_str(decl, "statement"), canon(f_ids(decl, "uses")), canon(f_ids(decl, "from")))
    if kind == "warrant":
        target = f_id(decl, "to")
        return (
            "warrant",
            canon(f_ids(decl, "from")),
            uf.find(target) if target else None,
            canon(f_ids(decl, "basis")),
            canon(f_ids(decl, "attacks")),
        )
    if kind == "value":
        return ("value", f_str(decl, "metric"), f_id(decl, "direction"), f_str(decl, "unit"), f_num(decl, "threshold"))
    return None


def merge_groups(prog: Program) -> tuple[UnionFind, list[list[str]]]:
    names = [d.name for d in prog.decls if d.kind in KINDS]
    uf = UnionFind(names)
    while True:
        buckets: dict[tuple, list[str]] = {}
        for decl in prog.decls:
            key = key_of(decl, uf)
            if key is None:
                continue
            buckets.setdefault(key, []).append(decl.name)
        changed = False
        for group in buckets.values():
            for other in group[1:]:
                changed = uf.union(group[0], other) or changed
        if not changed:
            break
    classes: dict[str, list[str]] = {}
    for name in names:
        classes.setdefault(uf.find(name), []).append(name)
    return uf, [sorted(group) for group in classes.values()]


def rewrite(decls: list[Decl], mapping: dict[str, str]) -> list[Decl]:
    keep = [decl for decl in fix.clone(decls) if mapping.get(decl.name, decl.name) == decl.name]
    for decl in keep:
        for key in decl.order:
            decl.fields[key] = _rename(decl.fields[key], mapping)
    return keep


def _rename(value, mapping: dict[str, str]):
    if isinstance(value, VId):
        return VId(value.span, mapping.get(value.name, value.name))
    if isinstance(value, VList):
        return VList(value.span, [_rename(item, mapping) for item in value.items])
    if isinstance(value, VRec):
        fields = {key: _rename(value.fields[key], mapping) for key in value.order}
        return VRec(value.span, fields, list(value.order))
    return copy.copy(value)


def canonical_map(uf: UnionFind) -> dict[str, str]:
    return {name: uf.find(name) for name in uf.parent}
