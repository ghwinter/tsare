"""engine CLI — the JSON contract from PROTOCOL.md.

    python cli.py check FILE
    python cli.py engine FILE
    python cli.py suggest FILE
"""

from __future__ import annotations

import argparse
import json
import sys
from pathlib import Path

import apply as applymod
import diag
import project as projectmod
import engine as enginemod
import fix
import suggestion
from check import check
from syntax import load


def check_file(path: Path) -> dict:
    if path.is_dir():
        results, errors = projectmod.check_project(path)
        diagnostics = [d for diags in results.values() for d in diags]
        return {
            "diagnostics": [
                {
                    "code": d.code, "severity": d.severity, "file": d.file or path.name,
                    "line": d.span.line, "col": d.span.col, "message": d.message, "hint": d.hint,
                }
                for d in diagnostics
            ],
            "errors": errors,
            "warnings": sum(1 for d in diagnostics if d.severity == "warning"),
        }
    decls, parse_errors, _ = load(path)
    diagnostics = [diag.error("E-SYNTAX", msg, span) for msg, span in parse_errors]
    prog, more = check(decls, path.parent)
    diagnostics.extend(more)
    for item in diagnostics:
        item.file = path.name
    return {
        "diagnostics": [
            {
                "code": d.code,
                "severity": d.severity,
                "file": d.file or path.name,
                "line": d.span.line,
                "col": d.span.col,
                "message": d.message,
                "hint": d.hint,
            }
            for d in diagnostics
        ],
        "errors": sum(1 for d in diagnostics if d.severity == "error"),
        "warnings": sum(1 for d in diagnostics if d.severity == "warning"),
    }


def cmd_check(args: argparse.Namespace) -> int:
    report = check_file(Path(args.file))
    print(json.dumps(report, ensure_ascii=False, indent=2))
    return 1 if report["errors"] else 0


def cmd_engine(args: argparse.Namespace) -> int:
    path = Path(args.file)
    if path.is_dir():
        prog = projectmod.load_program(path)
    else:
        decls, parse_errors, _ = load(path)
        if parse_errors:
            print(json.dumps({"error": "syntax errors"}, ensure_ascii=False))
            return 1
        prog, _ = check(decls, path.parent)
    result = enginemod.run(prog)
    print(
        json.dumps(
            {
                "held": result["held"],
                "conditional": result["conditional"],
                "blocked": [{"warrant": w, "missing": m} for w, m in result["dead_rules"]],
                "chains": result["chains"],
                "gates": result["gates"],
            },
            ensure_ascii=False,
            indent=2,
        )
    )
    return 0


def cmd_apply(args: argparse.Namespace) -> int:
    path = Path(args.file)
    text = path.read_text(encoding="utf-8")
    decls, parse_errors, _ = load(path)
    if parse_errors:
        print(json.dumps({"applied": [], "error": "syntax errors"}, ensure_ascii=False))
        return 1
    items = []
    for description, trial, _ in fix.repairs(decls, path.parent):
        items.append(suggestion.from_fix(decls, trial, True, description))
    for description, trial, _ in fix.from_diagnostics(decls, path.parent):
        items.append(suggestion.Suggestion(description, suggestion.Applicability.MaybeIncorrect, fix.render(trial)))
    before = applymod.score(text, path.parent)
    updated, applied = applymod.apply_best(text, path.parent, items)
    after = applymod.score(updated, path.parent)
    if not args.dry_run:
        path.write_text(updated, encoding="utf-8")
    print(json.dumps({"applied": applied, "before": list(before), "after": list(after)}, ensure_ascii=False, indent=2))
    return 0


def cmd_suggest(args: argparse.Namespace) -> int:
    path = Path(args.file)
    if path.is_dir():
        files, per_file, merged, _ = projectmod.load_dir(path)
        names = {name: decl for name, (_, decl) in merged.items()}
        suggestions = []
        for source in files:
            decls, parse_errors, _ = per_file[source][0], per_file[source][1], None
            if parse_errors:
                continue
            for description, trial, _ in fix.repairs(decls, source.parent, names):
                item = suggestion.from_fix(decls, trial, True, description)
                suggestions.append({"applicability": item.applicability.name, "message": item.message, "file": source.name, "replacement": item.replacement})
            for description, trial, _ in fix.from_diagnostics(decls, source.parent, names):
                suggestions.append({"applicability": "MaybeIncorrect", "message": description, "file": source.name, "replacement": fix.render(trial)})
        print(json.dumps({"suggestions": suggestions}, ensure_ascii=False, indent=2))
        return 0
    decls, parse_errors, _ = load(path)
    if parse_errors:
        print(json.dumps({"suggestions": []}, ensure_ascii=False))
        return 1
    items = []
    for description, trial, _ in fix.repairs(decls, path.parent):
        item = suggestion.from_fix(decls, trial, True, description)
        items.append(
            {
                "applicability": item.applicability.name,
                "message": item.message,
                "file": path.name,
                "span": None if item.span is None else {"line": item.span.line, "col": item.span.col, "end_line": item.span.end_line, "end_col": item.span.end_col},
                "replacement": item.replacement,
            }
        )
    for description, trial, _ in fix.from_diagnostics(decls, path.parent):
        items.append(
            {
                "applicability": "MaybeIncorrect",
                "message": description,
                "file": path.name,
                "replacement": fix.render(trial),
            }
        )
    print(json.dumps({"suggestions": items}, ensure_ascii=False, indent=2))
    return 0


def main() -> int:
    parser = argparse.ArgumentParser(prog="engine")
    sub = parser.add_subparsers(dest="cmd", required=True)
    for name, func in (("check", cmd_check), ("engine", cmd_engine), ("suggest", cmd_suggest), ("apply", cmd_apply)):
        item = sub.add_parser(name)
        item.add_argument("file")
        if name == "apply":
            item.add_argument("--dry-run", action="store_true")
        item.set_defaults(func=func)
    args = parser.parse_args()
    return args.func(args)


if __name__ == "__main__":
    sys.exit(main())
