"""Middle end: the kind system and its well-formedness rules.

Seven kinds, each a record type over a fixed field signature.  Checking is layered:

    structural      the declaration has the right fields, with the right value types
    resolution      every identifier refers to a declaration of the expected kind
    acyclicity      the dependency graph has no cycles
    semantic        the kind-specific rule that makes the declaration *mean* something

Nothing here judges truth.  It judges whether a declaration can be checked at all.
"""

from __future__ import annotations

from dataclasses import dataclass
from pathlib import Path

from diag import Diagnostic, error, warn
from syntax import (
    Decl,
    Span,
    VCall,
    VHole,
    VId,
    VList,
    VNum,
    VRec,
    VStr,
    Val,
    describe,
)

# kind -> field -> (type, required)
SPECS: dict[str, dict[str, tuple[str, bool]]] = {
    "fact": {
        "claim": ("str", True),
        "evidence": ("evidence", True),
        "attrs": ("attrs", False),
        "confirmed_by": ("str", False),
        "scope": ("str", False),
        "confidence": ("num", False),
    },
    "concept": {
        "name": ("str", True),
        "params": ("list:str", False),
        "grounded_by": ("list:ref:fact", True),
        "against": ("list:ref:fact", False),
        "parent": ("ref:concept", False),
    },
    "proposition": {
        "statement": ("str", True),
        "uses": ("list:ref:concept", False),
        "from": ("list:ref:any", False),
        "by": ("enum:deductive,inductive,abductive", False),
        "status": ("enum:asserted,open", False),
        "confidence": ("num", False),
    },
    "hypothesis": {
        "assumes": ("list:ref:any", True),
        "predicts": ("str", True),
        "test": ("str", False),
        "rival": ("ref:hypothesis", False),
        "verdict": ("enum:open,supported,refuted", False),
        "verdict_by": ("ref:fact", False),
    },
    "question": {
        "asks": ("str", True),
        "answers": ("list:str", True),
        "separates": ("list:ref:hypothesis", True),
        "to": ("enum:user,tool,experiment", False),
    },
    "blindspot": {
        "cannot_express": ("str", True),
        "witness": ("list:ref:any", True),
        "signature": ("list:ref:concept", True),
        "closes_with": ("list:ref:question", False),
    },
    "warrant": {
        "claim": ("str", True),
        "from": ("list:ref:any", True),
        "to": ("ref:proposition", False),
        "concludes": ("rec:any", False),
        "basis": ("list:ref:any", False),
        "attacks": ("list:ref:warrant", False),
        "confidence": ("num", False),
    },
    "layer": {
        "name": ("str", True),
        "exposes": ("list:ref:any", True),
    },
    "law": {
        "statement": ("str", True),
        "holds_in": ("ref:layer", True),
        "checked_by": ("list:ref:any", False),
    },
    "value": {
        "metric": ("str", True),
        "direction": ("enum:min,max", True),
        "unit": ("str", False),
        "threshold": ("num", False),
        "priority": ("num", False),
        "measured_by": ("instrument", False),
    },
    "note": {
        "tag": ("enum:intuition,conjecture,preference,hunch", True),
        "text": ("str", True),
        "about": ("list:ref:any", False),
    },
}

# every declaration may sit in a layer; only the layer itself does not
for _kind, _fields in SPECS.items():
    if _kind != "layer":
        _fields["layer"] = ("ref:layer", False)

KINDS = tuple(SPECS)
EVIDENCE = {"user", "model", "observation"}
EDGES = ("parent", "assumes", "from", "rival", "separates", "signature", "warrant")


@dataclass
class Program:
    decls: list[Decl]
    by_name: dict[str, Decl]
    base: Path | None = None

    def kind(self, kind: str) -> list[Decl]:
        return [d for d in self.decls if d.kind == kind]


# --------------------------------------------------------------- field accessors


def f_str(d: Decl, key: str) -> str | None:
    v = d.fields.get(key)
    return v.text if isinstance(v, VStr) else None


def f_num(d: Decl, key: str) -> float | None:
    v = d.fields.get(key)
    return v.value if isinstance(v, VNum) else None


def instrument_of(d: Decl) -> str | None:
    """The command that measures a value, whether written plain or as tool("...")."""
    value = d.fields.get("measured_by")
    if isinstance(value, VStr):
        return value.text
    if isinstance(value, VCall) and value.args and isinstance(value.args[0], VStr):
        return value.args[0].text
    return None


def f_id(d: Decl, key: str) -> str | None:
    v = d.fields.get(key)
    return v.name if isinstance(v, VId) else None


def f_ids(d: Decl, key: str) -> list[str]:
    v = d.fields.get(key)
    if isinstance(v, VList):
        return [i.name for i in v.items if isinstance(i, VId)]
    return []


def f_list(d: Decl, key: str) -> list[Val]:
    v = d.fields.get(key)
    return list(v.items) if isinstance(v, VList) else []


def span_of(d: Decl, key: str) -> Span:
    v = d.fields.get(key)
    return v.span if v is not None else d.span


# ------------------------------------------------------------------ value typing


def expect_kind(spec: str) -> str | None:
    if spec.startswith("ref:"):
        return spec[4:]
    if spec.startswith("list:ref:"):
        return spec[9:]
    return None


def type_errors(spec: str, val: Val) -> list[str]:
    if isinstance(val, VHole):
        return []
    if spec == "str":
        return [] if isinstance(val, VStr) else [f"expected a string, got {describe(val)}"]
    if spec == "num":
        return [] if isinstance(val, VNum) else [f"expected a number, got {describe(val)}"]
    if spec == "evidence":
        if isinstance(val, VId):
            if val.name in EVIDENCE:
                return []
            return [f"evidence must be one of {sorted(EVIDENCE)} or tool(..)/doc(..)/code(..), got {val.name!r}"]
        if isinstance(val, VCall):
            if val.name not in {"tool", "doc", "code"}:
                return [f"evidence call must be tool(..), doc(..), or code(..), got {val.name}(..)"]
            if len(val.args) != 1 or not isinstance(val.args[0], VStr):
                return [f"{val.name}(..) takes exactly one string argument"]
            return []
        return [f"expected evidence, got {describe(val)}"]
    if spec == "rec:any":
        if not isinstance(val, VRec):
            return [f"expected a record of field patterns, got {describe(val)}"]
        if not val.fields:
            return ["an empty pattern concludes nothing"]
        return []
    if spec == "instrument":
        # a measurement is named either as a plain string or as tool("..."), the same
        # spelling used for evidence; the model keeps choosing the latter and it is right
        if isinstance(val, VStr):
            return []
        if isinstance(val, VCall) and val.name in {"tool", "doc"}:
            if len(val.args) == 1 and isinstance(val.args[0], VStr):
                return []
            return [f"{val.name}(..) takes exactly one string argument"]
        return [f"expected a command string or tool(..), got {describe(val)}"]
    if spec == "attrs":
        if not isinstance(val, VRec):
            return [f"expected a record of attributes, got {describe(val)}"]
        errs = []
        for key in val.order:
            item = val.fields[key]
            if not isinstance(item, (VStr, VNum, VId)):
                errs.append(f"attribute {key!r} must be a string, number, or identifier")
        return errs
    if spec.startswith("enum:"):
        allowed = spec[5:].split(",")
        if not isinstance(val, VId):
            return [f"expected one of {allowed}, got {describe(val)}"]
        if val.name not in allowed:
            return [f"expected one of {allowed}, got {val.name!r}"]
        return []
    if spec.startswith("ref:"):
        return [] if isinstance(val, VId) else [f"expected a reference, got {describe(val)}"]
    if spec.startswith("list:"):
        if not isinstance(val, VList):
            return [f"expected a list, got {describe(val)}"]
        errs = []
        for item in val.items:
            errs.extend(type_errors(spec[5:], item))
        return errs
    raise AssertionError(f"unknown spec {spec}")


# ------------------------------------------------------------------------ checker


def check(decls: list[Decl], base: Path | None = None) -> tuple[Program, list[Diagnostic]]:
    diags: list[Diagnostic] = []
    if not decls:
        # an empty document is not a quiet success: it says nothing and can be checked
        # for nothing. Whoever produced it (a person, a model, a truncated stream) the
        # verdict is the same.
        diags.append(
            error(
                "E-EMPTY-DOCUMENT",
                "the document declares nothing, so there is nothing to check",
                Span(1, 1, 1, 1),
                "an empty answer is not a passing answer",
            )
        )
    prog, local = check_local(decls, base)
    return prog, diags + local + check_global(prog) + check_conditional(prog)


def check_conditional(prog: "Program") -> list[Diagnostic]:
    """A conclusion whose premise is only asserted is conditional, not grounded.

    argue already decides which rules survive; this asks whether the premises those rules
    lean on actually reach a fact.  Imported locally because derive imports this module.
    """
    from derive import analyze

    result = analyze(prog)
    out: list[Diagnostic] = []
    for name in result["conditional"]:
        decl = prog.by_name.get(name)
        reason = "; ".join(result["reasons"][name][:2])
        out.append(
            warn(
                "W-CONDITIONAL",
                f"{name} is supported only through an ungrounded premise: {reason}",
                decl.span if decl else Span(1, 1, 1, 1),
                "discharge the premise, or state the conclusion as conditional",
            )
        )
    return out


def check_local(decls: list[Decl], base: Path | None = None, names: dict | None = None, visibility: bool = True) -> tuple[Program, list[Diagnostic]]:
    """Everything that only needs the declaration and the names it reaches.

    These checks are safe to run on a connected component of the reference graph: a
    component contains every name its declarations refer to, so nothing is missing.
    Cross-component comparisons live in check_global.
    """
    diags: list[Diagnostic] = []
    by_name: dict[str, Decl] = {}
    local_names: dict[str, Decl] = {}
    for d in decls:
        if d.name in local_names:
            first = local_names[d.name]
            diags.append(
                error(
                    "E-DUP-ID",
                    f"{d.name} is declared twice; first at line {first.span.line}",
                    d.span,
                    "give one of them a distinct name",
                )
            )
            continue
        local_names[d.name] = d
    by_name = names if names is not None else local_names

    for d in decls:
        diags.extend(check_record(d))
    diags.extend(check_refs(decls, by_name))
    diags.extend(check_cycles(decls, by_name))
    prog = Program(decls, by_name, base)
    if visibility:
        diags.extend(check_layer_visibility(prog))
    incomplete = {
        d.name
        for d in decls
        if d.kind in SPECS
        and any(required and key not in d.fields for key, (_, required) in SPECS[d.kind].items())
    }
    for kind, rules in (
        ("fact", check_fact),
        ("concept", check_concept),
        ("proposition", check_proposition),
        ("hypothesis", check_hypothesis),
        ("question", check_question),
        ("blindspot", check_blindspot),
        ("warrant", check_warrant),
        ("law", check_law),
        ("layer", check_layer),
        ("value", check_value),
    ):
        for d in prog.kind(kind):
            if d.name in incomplete:
                continue
            diags.extend(rules(d, prog))
    return prog, diags


def check_global(prog: Program) -> list[Diagnostic]:
    """Comparisons between declarations that may sit in different components."""
    return check_collapse(prog) + check_tension(prog)


def check_record(d: Decl) -> list[Diagnostic]:
    if d.kind not in SPECS:
        return [
            error(
                "E-KIND-UNKNOWN",
                f"unknown declaration kind {d.kind!r}",
                d.kind_span,
                f"expected one of {list(SPECS)}",
            )
        ]
    spec = SPECS[d.kind]
    out: list[Diagnostic] = []
    for key in d.order:
        if key not in spec:
            out.append(
                error(
                    "E-FIELD",
                    f"{d.kind} has no field {key!r}",
                    span_of(d, key),
                    f"fields: {sorted(spec)}",
                )
            )
    for key, (ftype, required) in spec.items():
        if key not in d.fields:
            if required:
                out.append(
                    error(
                        "E-MISSING",
                        f"{d.kind} {d.name} is missing required field {key!r}",
                        d.span,
                        f"add `{key}: ...`",
                    )
                )
            continue
        value = d.fields[key]
        if isinstance(value, VHole):
            out.append(
                warn(
                    "W-HOLE",
                    f"{d.name}.{key} is a hole: it compiles, but nothing can check it yet",
                    value.span,
                    "fill it in, or leave it as a question on purpose",
                )
            )
            continue
        for message in type_errors(ftype, d.fields[key]):
            out.append(error("E-TYPE", f"{d.name}.{key}: {message}", span_of(d, key)))
    return out


def check_refs(decls: list[Decl], by_name: dict[str, Decl]) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    for d in decls:
        spec = SPECS.get(d.kind, {})
        for key, (ftype, _) in spec.items():
            want = expect_kind(ftype)
            if want is None or key not in d.fields:
                continue
            for ref in iter_refs(d.fields[key]):
                target = by_name.get(ref.name)
                if target is None:
                    out.append(
                        error(
                            "E-UNDEFINED",
                            f"{d.name}.{key} refers to {ref.name}, which is not declared",
                            ref.span,
                        )
                    )
                elif target.kind == "note":
                    out.append(
                        error(
                            "E-NOTE-REF",
                            f"{d.name}.{key} leans on the note {ref.name}, which is not a commitment",
                            ref.span,
                            "promote the note to a typed declaration first; notes never support anything",
                        )
                    )
                elif want != "any" and target.kind != want:
                    out.append(
                        error(
                            "E-KIND",
                            f"{d.name}.{key} expects {want}, but {ref.name} is {target.kind}",
                            ref.span,
                            f"declare a {want} or point at one",
                        )
                    )
    return out


def iter_refs(val: Val):
    if isinstance(val, VId):
        yield val
    elif isinstance(val, VList):
        for item in val.items:
            yield from iter_refs(item)
    elif isinstance(val, VRec):
        for item in val.fields.values():
            yield from iter_refs(item)
    elif isinstance(val, VCall):
        for item in val.args:
            yield from iter_refs(item)


def check_cycles(decls: list[Decl], by_name: dict[str, Decl]) -> list[Diagnostic]:
    edges: dict[str, list[str]] = {}
    for d in decls:
        targets: list[str] = []
        for key in EDGES:
            if key in d.fields:
                targets.extend(r.name for r in iter_refs(d.fields[key]))
        edges[d.name] = [t for t in targets if t in by_name]

    color: dict[str, int] = {}
    out: list[Diagnostic] = []

    def visit(node: str, stack: list[str]) -> None:
        color[node] = 1
        stack.append(node)
        for nxt in edges.get(node, []):
            if color.get(nxt, 0) == 0:
                visit(nxt, stack)
            elif color.get(nxt) == 1:
                cycle = stack[stack.index(nxt) :] + [nxt]
                out.append(
                    error(
                        "E-CYCLE",
                        "dependency cycle: " + " -> ".join(cycle),
                        by_name[node].span,
                        "a declaration cannot be justified by itself; cut one edge",
                    )
                )
        stack.pop()
        color[node] = 2

    for d in decls:
        if color.get(d.name, 0) == 0:
            visit(d.name, [])
    return out


# ---------------------------------------------------------------- kind semantics


def check_confidence(d: Decl) -> list[Diagnostic]:
    """A number between 0 and 1, or nothing. It never affects typing."""
    if "confidence" not in d.fields:
        return []
    value = f_num(d, "confidence")
    if value is None:
        return []
    if not 0.0 <= value <= 1.0:
        return [
            error(
                "E-CONFIDENCE",
                f"{d.name}.confidence must be between 0 and 1, got {value:g}",
                span_of(d, "confidence"),
            )
        ]
    return []


def check_fact(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    claim = f_str(d, "claim")
    if claim is not None and not claim.strip():
        out.append(error("E-EMPTY", f"fact {d.name} has an empty claim", span_of(d, "claim")))
    universal = ("所有", "都", "全部", "always", "every", "all ")
    if claim and any(word in claim for word in universal) and not f_str(d, "scope"):
        out.append(
            warn(
                "W-OVERCLAIM",
                f"fact {d.name} is a universal claim with no scope",
                span_of(d, "claim"),
                "say who was sampled (scope: \"n=40, active users\"), or weaken the claim",
            )
        )
    ev = d.fields.get("evidence")
    if isinstance(ev, VId) and ev.name == "model" and not f_str(d, "confirmed_by"):
        out.append(
            warn(
                "W-UNCONFIRMED",
                f"fact {d.name} was supplied by a model, not by the user or a tool",
                span_of(d, "evidence"),
                "add confirmed_by, or change evidence to user/tool",
            )
        )
    if isinstance(ev, VCall) and ev.name in {"doc", "code"} and prog.base is not None:
        out.extend(check_evidence_path(d, ev, prog.base))
    out.extend(check_confidence(d))
    if (
        isinstance(ev, VId)
        and ev.name == "model"
        and (f_num(d, "confidence") or 0) >= 0.8
    ):
        out.append(
            warn(
                "W-CONFIDENCE-MODEL",
                f"fact {d.name} is model-supplied but claims confidence {f_num(d, 'confidence'):g}",
                span_of(d, "confidence"),
                "confidence cannot upgrade an unconfirmed source; confirm it or lower it",
            )
        )
    return out


def check_evidence_path(d: Decl, call: VCall, base: Path) -> list[Diagnostic]:
    arg = call.args[0].text
    path_text, line = arg, None
    head, sep, tail = arg.rpartition(":")
    if sep and tail.isdigit():
        path_text, line = head, int(tail)
    path = base / path_text
    if not path.exists():
        return [
            error(
                "E-EVIDENCE-MISSING",
                f"{d.name} cites {path_text!r}, which does not exist next to this document",
                span_of(d, "evidence"),
                "cite a path that exists, or use tool(..) for a command",
            )
        ]
    if line is not None:
        count = len(path.read_text(encoding="utf-8", errors="replace").splitlines())
        if not 1 <= line <= count:
            return [
                error(
                    "E-EVIDENCE-LINE",
                    f"{d.name} cites line {line} of {path_text!r}, which has {count} lines",
                    span_of(d, "evidence"),
                )
            ]
    return []


def pos(d: Decl) -> list[str]:
    return f_ids(d, "grounded_by")


def neg(d: Decl) -> list[str]:
    return f_ids(d, "against")


def check_concept(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    positives = set(pos(d))
    negatives = set(neg(d))
    if not positives:
        out.append(
            error(
                "E-UNGROUNDED",
                f"concept {d.name} has no positive example, so it denotes nothing",
                span_of(d, "grounded_by"),
                "ground it in at least one fact",
            )
        )
    if not negatives:
        out.append(
            warn(
                "W-UNTESTED",
                f"concept {d.name} is never contrasted with a counterexample",
                d.span,
                "add against: [..] or the concept cannot discriminate",
            )
        )
    both = positives & negatives
    if both:
        out.append(
            error(
                "E-CONTRADICTION",
                f"concept {d.name} counts {sorted(both)} as both positive and negative",
                span_of(d, "against"),
            )
        )
    parent = f_id(d, "parent")
    if parent:
        upper = prog.by_name.get(parent)
        if upper is not None and upper.kind == "concept" and positives:
            outside = positives - set(pos(upper))
            if outside:
                out.append(
                    error(
                        "E-HIERARCHY",
                        f"{d.name} is declared under {parent}, but {sorted(outside)} lie outside {parent}",
                        span_of(d, "parent"),
                        "a subtype must have a smaller extent: move the example up, or drop the parent",
                    )
                )
    return out


def check_proposition(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    if not f_ids(d, "uses") and not f_ids(d, "from"):
        out.append(
            error(
                "E-VACUOUS",
                f"proposition {d.name} names no concept and cites no premise, so it is anchored in nothing",
                d.span,
                "name the concepts it talks about, or the premises it rests on",
            )
        )
    if f_id(d, "status") == "open" and not f_ids(d, "from"):
        out.append(
            warn(
                "W-OPEN",
                f"proposition {d.name} is open and has no premises",
                d.span,
                "add from: [..] once it is derived, or mark it asserted",
            )
        )
    if f_ids(d, "from") and not any(f_id(w, "to") == d.name for w in prog.kind("warrant")):
        out.append(
            warn(
                "W-UNWARRANTED",
                f"proposition {d.name} cites premises, but no rule connects them to it",
                span_of(d, "from"),
                "a derivation without a stated rule is an enthymeme; name the rule as a warrant",
            )
        )
    out.extend(check_confidence(d))
    if "confidence" in d.fields and not any(f_id(w, "to") == d.name for w in prog.kind("warrant")):
        out.append(
            warn(
                "W-CONFIDENCE-ASSERTED",
                f"proposition {d.name} carries a confidence but no rule computes it",
                span_of(d, "confidence"),
                "confidence must come from premises; an assertion with a number is still an assertion",
            )
        )
    return out


def check_hypothesis(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    if not f_ids(d, "assumes"):
        out.append(
            error(
                "E-NO-ASSUMPTION",
                f"hypothesis {d.name} assumes nothing, so nothing could refute it",
                span_of(d, "assumes"),
            )
        )
    test = f_str(d, "test")
    rival = f_id(d, "rival")
    if not (test and test.strip()) and not rival:
        out.append(
            error(
                "E-UNFALSIFIABLE",
                f"hypothesis {d.name} has neither a test nor a rival, so no observation could refute it",
                d.span,
                "add test: \"..\", or rival: <other hypothesis>",
            )
        )
    if rival == d.name:
        out.append(error("E-CYCLE", f"hypothesis {d.name} is its own rival", span_of(d, "rival")))
    verdict = f_id(d, "verdict")
    if verdict in {"supported", "refuted"} and not f_id(d, "verdict_by"):
        out.append(
            error(
                "E-NO-EVIDENCE",
                f"hypothesis {d.name} is marked {verdict} but no fact decides it",
                span_of(d, "verdict"),
                "point verdict_by at the fact that settled it",
            )
        )
    return out


def check_question(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    answers = [v.text for v in f_list(d, "answers") if isinstance(v, VStr)]
    separates = f_ids(d, "separates")
    if len(answers) < 2:
        out.append(
            error(
                "E-NO-ANSWERS",
                f"question {d.name} admits fewer than two answers, so it asks nothing",
                span_of(d, "answers"),
                "an answer set must partition the possibilities",
            )
        )
    if len(separates) < 2:
        out.append(
            error(
                "E-NON-DISCRIMINATING",
                f"question {d.name} does not separate two hypotheses",
                span_of(d, "separates"),
                "a question earns its place only by splitting a live disagreement",
            )
        )
    elif len(answers) < len(separates):
        out.append(
            warn(
                "W-COARSE",
                f"question {d.name} has {len(answers)} answers for {len(separates)} hypotheses",
                d.span,
                "some hypotheses would share an answer",
            )
        )
    return out


def separates(concept: Decl, a: str, b: str) -> bool:
    """A concept separates two objects when it places one inside and the other outside,
    with evidence on both sides.  A concept that never mentions one of them is silent."""
    p, n = set(pos(concept)), set(neg(concept))
    return (a in p and b in n) or (a in n and b in p)


def check_blindspot(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    witness = f_ids(d, "witness")
    signature = f_ids(d, "signature")
    if len(witness) != 2:
        out.append(
            error(
                "E-WITNESS",
                f"blindspot {d.name} needs exactly two witnesses, got {len(witness)}",
                span_of(d, "witness"),
            )
        )
    if not signature:
        out.append(
            error(
                "E-NO-SIGNATURE",
                f"blindspot {d.name} names no signature, so it is not relative to anything",
                span_of(d, "signature"),
                "a gap is always a gap in a particular vocabulary",
            )
        )
    if not f_ids(d, "closes_with"):
        out.append(
            warn(
                "W-NO-EXIT",
                f"blindspot {d.name} closes no question, so nothing will ever resolve it",
                span_of(d, "cannot_express"),
                "add closes_with: [Q_...] -- a gap with no exit is a note, not a task",
            )
        )
    if len(witness) == 2:
        left = prog.by_name.get(witness[0])
        right = prog.by_name.get(witness[1])
        if left is not None and right is not None:
            if left.kind == "fact" and right.kind == "fact":
                for cname in signature:
                    concept = prog.by_name.get(cname)
                    if concept is not None and concept.kind == "concept" and separates(
                        concept, *witness
                    ):
                        out.append(
                            error(
                                "E-FALSE-BLINDSPOT",
                                f"{cname} separates {witness[0]} from {witness[1]}, so {d.name} is not a blind spot",
                                span_of(d, "witness"),
                                "remove that concept from the signature, or pick a real collision",
                            )
                        )
            else:
                for source, target in ((left, right), (right, left)):
                    if derives(prog, source, target.name):
                        out.append(
                            error(
                                "E-FALSE-BLINDSPOT",
                                f"{source.name} already derives {target.name}, so the vocabulary does connect them",
                                span_of(d, "witness"),
                                "the gap is not the missing link, it is something else",
                            )
                        )
    return out


def derives(prog: Program, start: Decl, target: str) -> bool:
    stack = list(f_ids(start, "from"))
    seen: set[str] = set()
    while stack:
        name = stack.pop()
        if name == target:
            return True
        if name in seen:
            continue
        seen.add(name)
        node = prog.by_name.get(name)
        if node is not None and node.kind == "proposition":
            stack.extend(f_ids(node, "from"))
    return False


def check_warrant(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    if not f_ids(d, "from"):
        out.append(
            error(
                "E-NO-PREMISE",
                f"warrant {d.name} licenses an inference from nothing",
                span_of(d, "from"),
                "name the premises this rule consumes",
            )
        )
    has_to = f_id(d, "to") is not None
    has_concludes = isinstance(d.fields.get("concludes"), VRec)
    if has_to == has_concludes:
        out.append(
            error(
                "E-WARRANT-MODE",
                f"warrant {d.name} must conclude exactly one of a named proposition or a class",
                d.span,
                "use 	o: P_X for one conclusion, or concludes: { field: value } for every match",
            )
        )
    if not f_ids(d, "basis"):
        out.append(
            warn(
                "W-UNWARRANTED-BASIS",
                f"warrant {d.name} states a rule, but nothing makes the rule acceptable",
                span_of(d, "claim"),
                "name the value or claim that licenses this rule; that is the hidden premise",
            )
        )
    if d.name in f_ids(d, "attacks"):
        out.append(
            error(
                "E-SELF-ATTACK",
                f"warrant {d.name} attacks itself",
                span_of(d, "attacks"),
                "a rule that defeats itself licenses nothing",
            )
        )
    out.extend(check_confidence(d))
    return out


def check_law(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    if not f_ids(d, "checked_by"):
        out.append(
            warn(
                "W-LAW-UNCHECKED",
                f"law {d.name} is asserted, but nothing would break if it were false",
                span_of(d, "statement"),
                "add checked_by: [..] naming what fails when the law fails",
            )
        )
    return out


def check_layer_visibility(prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    exposes = {d.name: set(f_ids(d, "exposes")) for d in prog.kind("layer")}
    for d in prog.decls:
        source = f_id(d, "layer")
        if not source or d.kind == "layer":
            continue
        for key, (ftype, _) in SPECS.get(d.kind, {}).items():
            if key == "layer" or expect_kind(ftype) is None:
                continue
            value = d.fields.get(key)
            if value is None:
                continue
            for ref in iter_refs(value):
                target = prog.by_name.get(ref.name)
                if target is None or target.kind == "layer":
                    continue
                target_layer = f_id(target, "layer")
                if target_layer and target_layer != source and ref.name not in exposes.get(target_layer, set()):
                    out.append(
                        error(
                            "E-LAYER-LEAK",
                            f"{d.name} in {source} reaches {ref.name}, which {target_layer} does not expose",
                            ref.span,
                            f"expose {ref.name} from {target_layer}, or move this declaration",
                        )
                    )
    return out


def check_layer(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    for name in f_ids(d, "exposes"):
        target = prog.by_name.get(name)
        if target is not None and f_id(target, "layer") not in (None, d.name):
            out.append(
                warn(
                    "W-EXPOSE-FOREIGN",
                    f"layer {d.name} exposes {name}, which sits in {f_id(target, 'layer')}",
                    span_of(d, "exposes"),
                )
            )
    return out


def check_value(d: Decl, prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    missing = [key for key in ("unit", "threshold") if key not in d.fields]
    blank = [
        key
        for key in ("unit",)
        if key in d.fields
        and not isinstance(d.fields[key], VHole)
        and not (f_str(d, key) or "").strip()
    ]
    if missing or blank:
        lacking = missing + blank
        out.append(
            error(
                "E-INCOMMENSURABLE",
                f"value {d.name} lacks {', '.join(lacking)}, so it cannot be compared with anything",
                d.span,
                "a value needs a unit and a threshold to be a judgement",
            )
        )
    return out


def check_collapse(prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    seen: dict[tuple[frozenset[str], frozenset[str]], str] = {}
    for d in prog.kind("concept"):
        key = (frozenset(pos(d)), frozenset(neg(d)))
        if not key[0]:
            continue
        if key in seen:
            out.append(
                error(
                    "E-COLLAPSE",
                    f"concepts {seen[key]} and {d.name} have the same extent, so they are one concept with two names",
                    d.span,
                    f"merge them, or give {d.name} an example the other lacks",
                )
            )
        else:
            seen[key] = d.name
    return out


def check_tension(prog: Program) -> list[Diagnostic]:
    out: list[Diagnostic] = []
    by_metric: dict[str, list[Decl]] = {}
    for d in prog.kind("value"):
        metric = f_str(d, "metric")
        if metric:
            by_metric.setdefault(metric, []).append(d)
    for metric, group in by_metric.items():
        for i, a in enumerate(group):
            for b in group[i + 1 :]:
                da, db = f_id(a, "direction"), f_id(b, "direction")
                if da is None or db is None or da == db:
                    continue
                pa, pb = f_num(a, "priority"), f_num(b, "priority")
                if pa is None or pb is None or pa == pb:
                    if pa is None and pb is None:
                        why = "neither has a priority"
                    elif pa is None or pb is None:
                        why = "one of them has no priority"
                    else:
                        why = f"both have priority {pa:g}"
                    out.append(
                        error(
                            "E-TENSION",
                            f"{a.name} and {b.name} constrain {metric!r} in opposite directions "
                            f"({da} vs {db}) and {why}",
                            b.span,
                            "give one a higher priority, or split the metric by phase",
                        )
                    )
    return out
