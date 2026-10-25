"""L2: repair candidates that are verified by recompiling, not guessed.

Every candidate is an edit to the declaration list.  A candidate is accepted only if a
full re-check shows fewer errors (or, for a warning-only candidate, fewer warnings without
new errors).  Nothing is printed that has not been tried.

The printer at the bottom is the other half: it turns the checked declaration list back
into canonical source, so a fix can be written out.
"""

from __future__ import annotations

import copy
from pathlib import Path

import re

from check import SPECS, check, f_id, f_ids, f_num, f_str
from syntax import Decl, Span, VCall, VHole, VId, VList, VNum, VRec, VStr


def clone(decls: list[Decl]) -> list[Decl]:
    out = []
    for d in decls:
        new = copy.copy(d)
        new.fields = dict(d.fields)
        new.order = list(d.order)
        out.append(new)
    return out


def score(decls: list[Decl], base: Path | None, names: dict | None = None) -> tuple[int, int]:
    """Warnings that a fix should reduce. Holes are progress markers, not noise."""
    _, diags = check(decls, base, names)
    errors = sum(1 for d in diags if d.severity == "error")
    warnings = sum(1 for d in diags if d.severity == "warning" and d.code != "W-HOLE")
    return errors, warnings


def with_field(decls: list[Decl], name: str, field: str, value) -> list[Decl]:
    new = clone(decls)
    for d in new:
        if d.name == name:
            d.fields[field] = value
            if field not in d.order:
                d.order.append(field)
    return new


def with_warrant(decls: list[Decl], prop: Decl) -> list[Decl]:
    new = clone(decls)
    premises = f_ids(prop, "from")
    if not premises:
        return new
    fields = {
        "claim": VHole(prop.span),
        "from": VList(prop.span, [VId(prop.span, name) for name in premises]),
        "to": VId(prop.span, prop.name),
    }
    new.append(
        Decl(
            name=f"W_AUTO_{prop.name}",
            kind="warrant",
            fields=fields,
            order=["claim", "from", "to"],
            span=prop.span,
            kind_span=prop.span,
        )
    )
    return new


def candidates(decls: list[Decl], base: Path | None) -> list[tuple[str, list[Decl]]]:
    out: list[tuple[str, list[Decl]]] = []
    warranted = {f_id(w, "to") for w in decls if w.kind == "warrant"}

    # a duplicate name is the one repair a machine can make without guessing meaning:
    # keep the first declaration, drop the later one; references still resolve
    seen_names: dict[str, Decl] = {}
    for d in decls:
        if d.name in seen_names:
            trial = [other for other in decls if other is not d]
            out.append((f"{d.name}: drop the duplicate at line {d.span.line}", trial))
        else:
            seen_names[d.name] = d

    for d in decls:
        spec = SPECS.get(d.kind, {})
        missing = [
            field
            for field, (_, required) in spec.items()
            if required and field not in d.fields and field not in {"claim", "evidence", "name", "to"}
        ]
        if missing:
            trial = decls
            for field in missing:
                trial = with_field(trial, d.name, field, VHole(d.span))
            out.append((f"{d.name}: add " + ", ".join(f"`{f}: ?`" for f in missing), trial))
        if d.kind == "value":
            absent = [field for field in ("unit", "threshold") if field not in d.fields]
            if absent:
                trial = decls
                for field in absent:
                    trial = with_field(trial, d.name, field, VHole(d.span))
                out.append((f"{d.name}: add " + ", ".join(f"`{f}: ?`" for f in absent), trial))
        if d.kind == "proposition" and f_ids(d, "from") and d.name not in warranted:
            out.append(
                (
                    f"{d.name}: add a warrant over its premises",
                    with_warrant(decls, d),
                )
            )
        if d.kind == "fact":
            claim = f_str(d, "claim") or ""
            if any(word in claim for word in ("所有", "都", "全部", "always", "every", "all ")) and "scope" not in d.fields:
                out.append(
                    (
                        f"{d.name}: add `scope: ?`",
                        with_field(decls, d.name, "scope", VHole(d.span)),
                    )
                )

    # tension: give the later value a priority
    values = [d for d in decls if d.kind == "value"]
    for i, a in enumerate(values):
        for b in values[i + 1 :]:
            if (
                f_str(a, "metric") == f_str(b, "metric")
                and f_id(a, "direction")
                and f_id(b, "direction")
                and f_id(a, "direction") != f_id(b, "direction")
                and f_num(a, "priority") is None
                and f_num(b, "priority") is None
            ):
                out.append(
                    (
                        f"{b.name}: add `priority: 2` to outrank {a.name}",
                        with_field(decls, b.name, "priority", VNum(b.span, 2.0, "2")),
                    )
                )
    return out


def verified(decls: list[Decl], base: Path | None) -> list[tuple[str, list[Decl], tuple[int, int]]]:
    before_e, before_w = score(decls, base)
    accepted = []
    seen: set[str] = set()
    for description, trial in candidates(decls, base):
        if description in seen:
            continue
        seen.add(description)
        after_e, after_w = score(trial, base)
        better = after_e < before_e or (after_e == before_e and after_w < before_w)
        if better:
            accepted.append((description, trial, (after_e - before_e, after_w - before_w)))
    return accepted, (before_e, before_w)


def render_value(value) -> str:
    if isinstance(value, VHole):
        return "?"
    if isinstance(value, VStr):
        escaped = value.text.replace("\\", "\\\\").replace('"', '\\"').replace("\n", "\\n")
        return f'"{escaped}"'
    if isinstance(value, VNum):
        return value.raw
    if isinstance(value, VId):
        return value.name
    if isinstance(value, VList):
        return "[" + ", ".join(render_value(item) for item in value.items) + "]"
    if isinstance(value, VRec):
        inner = ", ".join(f"{key}: {render_value(value.fields[key])}" for key in value.order)
        return "{ " + inner + " }"
    if isinstance(value, VCall):
        return value.name + "(" + ", ".join(render_value(arg) for arg in value.args) + ")"
    return "?"


def render(decls: list[Decl]) -> str:
    chunks = []
    for d in decls:
        lines = [f"{d.kind} {d.name} {{"]
        for key in d.order:
            lines.append(f"  {key}: {render_value(d.fields[key])},")
        lines.append("}")
        chunks.append("\n".join(lines))
    return "\n\n".join(chunks) + "\n"


def repairs(decls: list[Decl], base: Path | None, names: dict | None = None):
    """Candidates that fix a diagnostic without adding errors, even if the score is flat.

    A repair may trade one warning for another (add a warrant, gain a missing basis).  The
    strict erified filter drops those, which is why no MaybeIncorrect suggestion could be
    produced; this keeps them, and the caller decides what may be applied.
    """
    before_e, before_w = score(decls, base, names)
    _, before_diags = check(decls, base, names)
    import collections
    before_codes = collections.Counter(d.code for d in before_diags)
    accepted = []
    for description, trial in candidates(decls, base):
        after_e, after_w = score(trial, base, names)
        if after_e > before_e:
            continue
        _, after_diags = check(trial, base, names)
        after_codes = collections.Counter(d.code for d in after_diags)
        if any(after_codes[c] < n for c, n in before_codes.items()):
            accepted.append((description, trial, (after_e - before_e, after_w - before_w)))
    return accepted


KIND_MSG = re.compile(r"^(\w+)\.(\w+) expects (\w+), but (\w+) is \w+$")
UNDEF_MSG = re.compile(r"^(\w+)\.(\w+) refers to (\w+),")


def drop_ref(decls: list[Decl], owner: str, key: str, ref: str):
    new = clone(decls)
    for d in new:
        if d.name != owner:
            continue
        value = d.fields.get(key)
        if not isinstance(value, VList):
            return None
        items = [item for item in value.items if not (isinstance(item, VId) and item.name == ref)]
        if len(items) == len(value.items):
            return None
        d.fields[key] = VList(value.span, items)
        return new
    return None


def from_diagnostics(decls: list[Decl], base: Path | None, names: dict | None = None):
    """Repairs read off a diagnostic: drop the reference that does not belong."""
    before_e, _ = score(decls, base, names)
    _, diags = check(decls, base, names)
    out = []
    seen = set()
    for d in diags:
        kind_match = KIND_MSG.match(d.message)
        if kind_match:
            owner, key, want, ref = kind_match.groups()
            candidates = [x.name for x in decls if x.kind == want]
            trial = swap_ref(decls, owner, key, ref, candidates[0]) if candidates else None
        else:
            match = UNDEF_MSG.match(d.message)
            if not match:
                continue
            owner, key, ref = match.groups()
            trial = drop_ref(decls, owner, key, ref)
        if (owner, key, ref) in seen:
            continue
        seen.add((owner, key, ref))
        if trial is None:
            continue
        after_e, _ = score(trial, base, names)
        if after_e < before_e:
            out.append((f"{owner}: replace {ref} in {key}", trial, (after_e - before_e, 0)))
    return out


def swap_ref(decls: list[Decl], owner: str, key: str, ref: str, replacement: str):
    new = clone(decls)
    for d in new:
        if d.name != owner:
            continue
        value = d.fields.get(key)
        if not isinstance(value, VList):
            return None
        items = [VId(item.span, replacement) if isinstance(item, VId) and item.name == ref else item for item in value.items]
        d.fields[key] = VList(value.span, items)
        return new
    return None
