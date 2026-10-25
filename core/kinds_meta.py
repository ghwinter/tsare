"""One table for what each kind owes, so the analyses stop re-implementing it.

Before this file, `cost`, `assumptions` and `moves` each had their own per-kind branch and
they disagreed: cost ignored a rule without a basis, assumptions counted it. A kind's
obligation is data here, and every analysis reads the same list.
"""

from __future__ import annotations

from check import Program, f_id, f_ids, f_str, instrument_of
from syntax import VHole

TABLE: dict[str, tuple[str, str]] = {
    "fact": ("evidence someone else can check", "run the tool, or ask the person who saw it"),
    "concept": ("a positive and a negative example", "find a counterexample, or drop the concept"),
    "proposition": ("a rule that connects its premises", "name the warrant, or mark it assumed"),
    "hypothesis": ("a test or a rival", "design the observation that would refute it"),
    "question": ("two answers that split two live hypotheses", "ask it of a named respondent"),
    "blindspot": ("a witness pair plus the question that would close it", "add closes_with"),
    "value": ("an instrument and a threshold", "name the command that measures it"),
    "warrant": ("a premise and a basis", "say what makes the rule acceptable"),
    "law": ("something that would break if it failed", "point checked_by at that thing"),
    "layer": ("an interface that hides its internals", "expose exactly what others may use"),
    "note": ("nothing; it is not a commitment", "promote it to a typed declaration to make it count"),
}


def obligations(decl, prog: Program) -> list[str]:
    """Everything that still costs an external confirmation for this declaration."""
    out: list[str] = []
    for key in decl.order:
        if isinstance(decl.fields[key], VHole):
            out.append(f"hole: {key}")
    if decl.kind == "fact":
        evidence = decl.fields.get("evidence")
        if getattr(evidence, "name", None) in {"user", "model"}:
            out.append("evidence is asserted, not measured")
    elif decl.kind == "proposition":
        if not any(f_id(w, "to") == decl.name for w in prog.kind("warrant")):
            out.append("no rule supports it")
    elif decl.kind == "value" and not instrument_of(decl):
        out.append("no instrument to measure it")
    elif decl.kind == "hypothesis" and not f_str(decl, "test"):
        out.append("no test to run")
    elif decl.kind == "question" and (f_id(decl, "to") or "user") == "user":
        out.append("needs an answer from the user")
    elif decl.kind == "blindspot" and not f_ids(decl, "closes_with"):
        out.append("no question would close it")
    elif decl.kind == "warrant" and not f_ids(decl, "basis"):
        out.append("rule with no basis: why should it be acceptable?")
    elif decl.kind == "law" and not f_ids(decl, "checked_by"):
        out.append("no checker")
    return out


def obligation_text(kind: str) -> tuple[str, str]:
    return TABLE.get(kind, ("", ""))
