"""Projects: a directory checked as one program, with attribution by span.

v1 guessed a diagnostic's file from the names inside its message.  Here every declaration
remembers the file it came from, and a diagnostic is attributed by its span: an exact match
against a declaration's span, which is stable and does not depend on wording.
"""

from __future__ import annotations

from pathlib import Path

from check import Program, check_conditional, check_global, check_layer_visibility, check_local
from diag import error
from syntax import load


def load_dir(directory: Path):
    files = sorted(directory.glob("*.mf"))
    per_file = {}
    all_decls = []
    for path in files:
        decls, parse_errors, src = load(path)
        per_file[path] = (decls, parse_errors, src)
        all_decls.extend(decls)
    merged = {}
    duplicates = {}
    for path, (decls, _, _) in per_file.items():
        for decl in decls:
            if decl.name in merged:
                first = merged[decl.name][0]
                duplicates.setdefault(path, []).append(
                    error(
                        "E-DUP-ID",
                        f"{decl.name} is declared in {first.name} and again in {path.name}",
                        decl.span,
                        "names are global across the project",
                    )
                )
            else:
                merged[decl.name] = (path, decl)
    return files, per_file, merged, duplicates


def span_index(per_file) -> dict[tuple[int, int, int, int], Path]:
    index = {}
    for path, (decls, _, _) in per_file.items():
        for decl in decls:
            index[(decl.span.line, decl.span.col, decl.span.end_line, decl.span.end_col)] = path
    return index


def check_project(directory: Path) -> tuple[dict[Path, list], int]:
    files, per_file, merged, duplicates = load_dir(directory)
    names = {name: decl for name, (_, decl) in merged.items()}
    all_decls = [decl for _, (decls, _, _) in per_file.items() for decl in decls]
    prog = Program(all_decls, names, directory)
    index = span_index(per_file)

    results = {}
    for path, (decls, parse_errors, _) in per_file.items():
        diags = [error("E-SYNTAX", msg, span) for msg, span in parse_errors]
        _, local = check_local(decls, path.parent, names, visibility=False)
        diags.extend(local)
        diags.extend(duplicates.get(path, []))
        for item in diags:
            item.file = path.name
        results[path] = diags

    for diagnostic in check_global(prog) + check_conditional(prog) + check_layer_visibility(prog):
        span = (diagnostic.span.line, diagnostic.span.col, diagnostic.span.end_line, diagnostic.span.end_col)
        owner = index.get(span, files[0])
        diagnostic.file = owner.name
        results.setdefault(owner, []).append(diagnostic)

    errors = sum(1 for diags in results.values() for d in diags if d.severity == "error")
    return results, errors
